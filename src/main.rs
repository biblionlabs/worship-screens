use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::monitor::MonitorHandle;
use slint::{ComponentHandle, Image, ModelRc, Rgba8Pixel, SharedPixelBuffer, ToSharedString};
use ui::*;

fn main() {
    let monitors: Arc<Mutex<HashMap<String, MonitorHandle>>> = Default::default();

    let main_window = MainWindow::new().unwrap();
    let settings_window = SettingsWindow::new().unwrap();
    let view_window = ViewWindow::new().unwrap();

    main_window.on_start_window({
        let main_window = main_window.as_weak();
        let monitors = monitors.clone();
        let view_window = view_window.as_weak();
        move || {
            let main_window = main_window.unwrap();
            main_window
                .window()
                .with_winit_window(|window| {
                    let mut monitors = monitors.lock().unwrap();
                    *monitors = window
                        .available_monitors()
                        .flat_map(|m| m.name().map(|n| (n, m)))
                        .collect::<HashMap<_, _>>();

                    let settings = main_window.global::<Settings>();
                    settings.set_monitors(ModelRc::from(
                        monitors
                            .iter()
                            .map(|(n, _)| n.to_shared_string())
                            .collect::<Vec<_>>()
                            .as_slice(),
                    ));

                    let Some(current) = window.current_monitor() else {
                        return;
                    };

                    let Some(name) = current.name() else {
                        return;
                    };

                    let Some((n, (_, second_screen))) =
                        monitors.iter().enumerate().find(|(_, (m, _))| **m != name)
                    else {
                        return;
                    };

                    let view_window = view_window.unwrap();
                    let state = view_window.global::<ViewState>();
                    state.set_window_width(second_screen.size().width as _);
                    state.set_window_height(second_screen.size().height as _);
                    settings.set_selected_monitor(n as _);

                    slint::spawn_local({
                        let view_window = view_window.as_weak();
                        let second_screen = second_screen.clone();
                        async move {
                            let w = view_window.unwrap().window().winit_window().await.unwrap();
                            w.set_fullscreen(Some(
                                slint::winit_030::winit::window::Fullscreen::Borderless(Some(
                                    second_screen,
                                )),
                            ));
                        }
                    })
                    .unwrap();
                    view_window.show().unwrap();
                })
                .unwrap();
        }
    });

    main_window.on_change_monitor({
        let view_window = view_window.as_weak();
        let monitors = monitors.clone();
        move |name| {
            let monitors = monitors.lock().unwrap();
            let monitor = monitors.get(&name.to_string()).unwrap();
            slint::spawn_local({
                let view_window = view_window.unwrap().as_weak();
                let monitor = monitor.clone();
                async move {
                    let view_window = view_window.unwrap();
                    let w = view_window.window().winit_window().await.unwrap();
                    let state = view_window.global::<ViewState>();
                    state.set_window_width(monitor.size().width as _);
                    state.set_window_height(monitor.size().height as _);
                    w.set_fullscreen(Some(
                        slint::winit_030::winit::window::Fullscreen::Borderless(Some(monitor)),
                    ));
                }
            })
            .unwrap();
        }
    });

    main_window.on_set_image({
        let main_window = main_window.as_weak();
        move || {
            let file = rfd::FileDialog::new()
                .set_title("Select Image")
                .pick_file()
                .unwrap();
            let file = image::open(file).unwrap();
            let file = file.to_rgba8();

            let mut pixels = SharedPixelBuffer::<Rgba8Pixel>::new(file.width(), file.height());
            pixels.make_mut_bytes().copy_from_slice(&file.into_raw());
            let img_frame = Image::from_rgba8(pixels);

            main_window.unwrap().set_source(img_frame);
        }
    });

    main_window.on_open_settings({
        let settings_window = settings_window.as_weak();
        move || {
            settings_window.unwrap().show().unwrap();
        }
    });

    main_window.on_shutdown_output({
        let view_window = view_window.as_weak();
        move || {
            let view_window = view_window.unwrap();
            let state = view_window.global::<ViewState>();
            // TODO: set default content
            state.set_off(true);
        }
    });

    main_window.on_clear_output({
        let view_window = view_window.as_weak();
        move || {
            let view_window = view_window.unwrap();
            let state = view_window.global::<ViewState>();
            // TODO: set default content
            state.set_off(false);
        }
    });

    main_window.window().on_close_requested({
        let view_window = view_window.as_weak();
        move || {
            view_window.unwrap().hide().unwrap();
            slint::CloseRequestResponse::HideWindow
        }
    });

    main_window.run().unwrap();
}
