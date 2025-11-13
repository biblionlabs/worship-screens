use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bitstream_converter::Mp4BitstreamConverter;
use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::monitor::MonitorHandle;
use slint::{
    ComponentHandle, Image, ModelRc, Rgb8Pixel, Rgba8Pixel, SharedPixelBuffer, SharedString,
    ToSharedString,
};
use ui::*;

mod bitstream_converter;
mod user_data;
fn main() {
    let monitors: Arc<Mutex<HashMap<String, MonitorHandle>>> = Default::default();

    let main_window = MainWindow::new().unwrap();
    let settings_window = SettingsWindow::new().unwrap();
    let view_window = ViewWindow::new().unwrap();

    settings_window.on_close({
        let settings_window = settings_window.as_weak();
        move || {
            settings_window.unwrap().hide().unwrap();
        }
    });

    settings_window.on_close({
        let settings_window = settings_window.as_weak();
        move || {
            // TODO: save and sync settings before close
            settings_window.unwrap().hide().unwrap();
        }
    });

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

    main_window.on_send_to_view({
        let view_window = view_window.as_weak();
        let main_window = main_window.as_weak();
        move || {
            let view_window = view_window.unwrap();
            let main_window = main_window.unwrap();
            let main_state = main_window.global::<ViewState>();
            let view_state = view_window.global::<ViewState>();

            view_state.set_content(main_state.get_content());
            view_state.set_color(main_state.get_color());
            view_state.set_img_bg(main_state.get_img_bg());
            view_state.set_view_type(main_state.get_view_type());
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
        let view_window = view_window.as_weak();
        move || {
            let main_window = main_window.clone();
            let view_window = view_window.clone();
            // main_window
            //     .unwrap()
            //     .global::<ViewState>()
            //     .set_view_type(ViewType::Media);
            // view_window
            //     .unwrap()
            //     .global::<ViewState>()
            //     .set_view_type(ViewType::Media);
            let cb = move |pixels: SharedPixelBuffer<Rgb8Pixel>| {
                let main_window = main_window.clone();
                let view_window = view_window.clone();
                slint::invoke_from_event_loop(move || {
                    let main_window = main_window.unwrap();
                    let view_window = view_window.unwrap();
                    let state = main_window.global::<ViewState>();
                    state.set_img_bg(Image::from_rgb8(pixels.clone()));
                    // if PLAYING.fetch() {
                        let view_state = view_window.global::<ViewState>();
                        view_state.set_img_bg(Image::from_rgb8(pixels));
                    // }
                })
                .unwrap()
            };

            std::thread::spawn(move || {
                use std::fs::File;
                use std::io::BufReader;

                use mp4::*;
                use openh264::decoder::{Decoder, DecoderConfig, Flush};

                let src_file = File::open("/home/s4rch/Videos/2025-06-14 17-30-19.mp4").unwrap();
                let size = src_file.metadata().unwrap().len();
                let reader = BufReader::new(src_file);
                let mut mp4_reader = mp4::Mp4Reader::read_header(reader, size).unwrap();
                let track = mp4_reader
                    .tracks()
                    .iter()
                    .find(|(_, t)| t.media_type().unwrap() == mp4::MediaType::H264)
                    .unwrap()
                    .1;

                let decoder_options = DecoderConfig::new().flush_after_decode(Flush::NoFlush);
                let mut decoder =
                    Decoder::with_api_config(openh264::OpenH264API::from_source(), decoder_options)
                        .unwrap();

                let track_id = track.track_id();
                let width = track.width() as usize;
                let height = track.height() as usize;
                let wait_time = std::time::Duration::from_secs_f64(1.0 / track.frame_rate());
                let mut bitstream_converter = Mp4BitstreamConverter::for_mp4_track(&track).unwrap();
                let mut buffer = Vec::new();
                for i in 1..=track.sample_count() {
                    let Some(sample) = mp4_reader.read_sample(track_id, i).unwrap() else {
                        println!("Cannot read sample");
                        return;
                    };

                    // convert the packet from mp4 representation to one that openh264 can decode
                    bitstream_converter.convert_packet(&sample.bytes, &mut buffer);
                    match decoder.decode(&buffer) {
                        Ok(Some(image)) => {
                            let mut pixels =
                                SharedPixelBuffer::<Rgb8Pixel>::new(width as _, height as _);
                            image.write_rgb8(pixels.make_mut_bytes());
                            cb(pixels);
                        }
                        Ok(None) => {} // decoder is not ready to provide an image
                        Err(err) => {
                            println!("error frame {i}: {err}");
                        }
                    }
                    std::thread::sleep(wait_time);
                }

                for image in decoder.flush_remaining().unwrap() {
                    let mut pixels = SharedPixelBuffer::<Rgb8Pixel>::new(width as _, height as _);
                    image.write_rgb8(pixels.make_mut_bytes());
                    cb(pixels);
                }
            });
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
        let main_window = main_window.as_weak();
        let view_window = view_window.as_weak();
        move || {
            let view_window = view_window.unwrap();
            let main_window = main_window.unwrap();
            let state = view_window.global::<ViewState>();
            // TODO: set default content
            state.set_content(SharedString::default());
            main_window.global::<ViewState>().set_content(SharedString::default());
            state.set_off(false);
        }
    });

    main_window.window().on_close_requested({
        let view_window = view_window.as_weak();
        let settings_window = settings_window.as_weak();
        move || {
            view_window.unwrap().hide().unwrap();
            settings_window.unwrap().hide().unwrap();
            slint::CloseRequestResponse::HideWindow
        }
    });

    main_window.run().unwrap();
}
