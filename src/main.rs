use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use bitstream_converter::Mp4BitstreamConverter;
use setup_core::{Selection, SqliteDbSink, event};
use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::monitor::MonitorHandle;
use slint::{
    ComponentHandle, Image, Model, ModelRc, Rgb8Pixel, SharedPixelBuffer, SharedString,
    ToSharedString,
};
use ui::*;

use self::fav_text_manager::FavTextManager;
use self::song_manager::SongsManager;
use self::user_data::UserData;

mod bitstream_converter;
mod fav_text_manager;
mod settings;
mod song_manager;
mod user_data;

const PLAYING: AtomicBool = AtomicBool::new(false);

fn main() {
    let monitors: Arc<Mutex<HashMap<String, MonitorHandle>>> = Default::default();
    let data_manager: Arc<UserData> = Default::default();
    let selection: Arc<Mutex<Selection>> = Default::default();

    let main_window = MainWindow::new().unwrap();
    let settings_window = SettingsWindow::new().unwrap();
    let view_window = ViewWindow::new().unwrap();

    let mut song_manager = SongsManager::new(main_window.as_weak(), data_manager.clone());
    song_manager.initialize();
    song_manager.connect_callbacks();

    let fav_manager = FavTextManager::new(main_window.as_weak(), data_manager.clone());
    fav_manager.initialize();
    fav_manager.connect_callbacks();

    let database = Arc::new(SqliteDbSink::from(data_manager.data_dir(&["bibles.db"])));
    let source_variants = setup_core::SetupBuilder::new().cache_path(data_manager.data_dir(&["cache"]))
        // Add Reina Valera 1960 Bible
        .add_bible_from_url("spa_rv1960", "https://raw.githubusercontent.com/biblionlabs/extra_data_source/refs/heads/main/bibles/spa_rv1960/manifest.json", "https://raw.githubusercontent.com/biblionlabs/extra_data_source/refs/heads/main/bibles/spa_rv1960/desc.json", Some("https://raw.githubusercontent.com/biblionlabs/extra_data_source/refs/heads/main/bibles/{bible_id}/books/{book}.json"))
        .on::<event::Message>({
            let main_window = main_window.as_weak();
            let settings_window = settings_window.as_weak();
            move |msg| {
            //     let main_window = main_window.unwrap();
            //     let state = main_window.global::<MainState>();
                println!("Msg: {msg}");
            }})
        .on::<event::Completed>({
            let main_window = main_window.as_weak();
            let settings_window = settings_window.as_weak();
            move |_msg| {
                // let main_window = main_window.unwrap();
                // let state = main_window.global::<MainState>();

            }})
        .on::<event::Error>({
            let main_window = main_window.as_weak();
            move |e| {
                // let main_window = main_window.unwrap();
                // let state = main_window.global::<MainState>();
                eprintln!("Error: {e}");
            }})
        .on::<event::Progress>({
            let main_window = main_window.as_weak();
            let settings_window = settings_window.as_weak();
            move |(step_id, current, total)| {
                if step_id == "crossrefs" {
                    return;
                }
            let settings_window = settings_window.clone();
                slint::invoke_from_event_loop(move || {
                // let main_window = main_window.unwrap();
                // let state = main_window.global::<MainState>();

                let settings_window = settings_window.unwrap();
                let bibles = settings_window.get_bibles();
                let Some(idx) = bibles.iter().position(|b| b.id == step_id) else {
                    return;
                };
                let mut bible = bibles.row_data(idx).unwrap();
                bible.installing = current != total;
                bible.installed = current == total;
                bible.progress = current as f32 / total as f32;
                bibles.set_row_data(idx, bible);

                // TODO: send notification about state
                }).unwrap();
            }})
        .build().1;

    let source_variants: Arc<setup_core::Setup> = Arc::new(source_variants);
    std::thread::spawn({
        let source_variants = source_variants.clone();
        let database = database.clone();
        move || {
            source_variants.install_cross(database.as_ref()).unwrap();
            source_variants
                .install_langs(database.as_ref(), &[])
                .unwrap();
        }
    });
    std::thread::spawn({
        let source_variants = source_variants.clone();
        let settings_window = settings_window.as_weak();
        move || {
            let settings_window = settings_window.clone();
            let bibles = source_variants
                .list_bibles()
                .map(|bibles| {
                    bibles
                        .iter()
                        .map(|(id, name, english, _lang)| Bible {
                            id: id.into(),
                            english_name: english.into(),
                            installed: source_variants.is_bible_installed(id),
                            installing: false,
                            name: name.into(),
                            progress: 0.0,
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap();
            slint::invoke_from_event_loop(move || {
                settings_window
                    .unwrap()
                    .set_bibles(ModelRc::from(bibles.as_slice()));
            })
            .unwrap();
        }
    });

    settings_window.on_close({
        let settings_window = settings_window.as_weak();
        move || {
            settings_window.unwrap().hide().unwrap();
        }
    });

    settings_window.on_search({
        let source_variants = source_variants.clone();
        let settings_window = settings_window.as_weak();
        move |s| {
            let s = s.as_str().to_lowercase();
            let settings_window = settings_window.unwrap();
            let bibles = source_variants
                .list_bibles()
                .map(|bibles| {
                    bibles
                        .iter()
                        .filter_map(|(id, name, english, _lang)| {
                            (english.to_lowercase().contains(&s)
                                || name.to_lowercase().contains(&s))
                            .then_some(Bible {
                                id: id.into(),
                                english_name: english.into(),
                                installed: source_variants.is_bible_installed(id),
                                installing: false,
                                name: name.into(),
                                progress: 0.0,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap();
            settings_window.set_bibles(ModelRc::from(bibles.as_slice()));
        }
    });

    settings_window.on_save({
        let settings_window = settings_window.as_weak();
        move || {
            // TODO: sync and save settings
        }
    });

    settings_window.on_select_bible({
        let setup = source_variants.clone();
        let source_variants = source_variants.clone();
        let selection = selection.clone();
        let database = database.clone();
        move |bible_id| {
            let bible_id = bible_id.as_str().to_string();
            let mut selection = selection.lock().unwrap();
            if selection.bibles.contains(&bible_id) {
                return;
            }
            selection.bibles.push(bible_id.clone());
            source_variants.save_selection(&selection).unwrap();
            std::thread::spawn({
                let setup = setup.clone();
                let database = database.clone();
                move || {
                    setup
                        .install_bibles(database.as_ref(), &[bible_id])
                        .unwrap();
                }
            });
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

    main_window.on_search_verse({
        let main_window = main_window.as_weak();
        let database = database.clone();
        move |s| {
            let s = s.as_str();
            let main_window = main_window.unwrap();
            let main_state = main_window.global::<MainState>();

            const MAX_CHARS: usize = 20;

            let verses =
                setup_core::service_db::SearchedVerse::from_search(database.conn.clone(), s)
                    .unwrap()
                    .iter()
                    .flat_map(|v| {
                        let text = &v.text;
                        let text_len = text.len();

                        if text_len <= MAX_CHARS {
                            vec![Verse {
                                bible: Bible {
                                    english_name: v.bible.english_name.to_shared_string(),
                                    id: v.bible.id.to_shared_string(),
                                    installed: false,
                                    installing: false,
                                    name: v.bible.name.to_shared_string(),
                                    progress: 0.0,
                                },
                                part: 0,
                                book: v.book.to_shared_string(),
                                chapter: v.chapter,
                                text: text.to_shared_string(),
                                verse: v.verse,
                            }]
                        } else {
                            let mut parts = Vec::new();
                            let words: Vec<&str> = text.split_whitespace().collect();
                            let total_words = words.len();
                            let mut part = 1;
                            let mut i = 0;

                            while i < total_words {
                                let end = (i + MAX_CHARS).min(total_words);
                                let part_text = words[i..end].join(" ");
                                parts.push(Verse {
                                    bible: Bible {
                                        english_name: v.bible.english_name.to_shared_string(),
                                        id: v.bible.id.to_shared_string(),
                                        installed: false,
                                        installing: false,
                                        name: v.bible.name.to_shared_string(),
                                        progress: 0.0,
                                    },
                                    part,
                                    book: v.book.to_shared_string(),
                                    chapter: v.chapter,
                                    text: part_text.to_shared_string(),
                                    verse: v.verse,
                                });
                                part += 1;
                                i = end;
                            }

                            parts
                        }
                    })
                    .collect::<Vec<_>>();

            main_state.set_verses(ModelRc::from(verses.as_slice()));
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
            main_window
                .global::<ViewState>()
                .set_content(SharedString::default());
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
