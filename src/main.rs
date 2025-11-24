use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use setup_core::{Selection, SqliteDbSink, event};
use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::monitor::MonitorHandle;
use slint::{ComponentHandle, Model, ModelRc, SharedString, ToSharedString};
use ui::*;

use self::fav_text_manager::FavTextManager;
use self::media_manager::MediaManager;
use self::settings::AppSettings;
use self::song_manager::SongsManager;
use self::user_data::UserData;

mod bitstream_converter;
mod fav_text_manager;
mod media_manager;
mod settings;
mod song_manager;
mod user_data;

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

    let media_manager = Arc::new(MediaManager::new(
        main_window.as_weak(),
        view_window.as_weak(),
        data_manager.clone(),
    ));

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
    // std::thread::spawn({
    //     let source_variants = source_variants.clone();
    //     let database = database.clone();
    //     move || {
    //         source_variants.install_cross(database.as_ref()).unwrap();
    //         source_variants
    //             .install_langs(database.as_ref(), &[])
    //             .unwrap();
    //     }
    // });
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
        let media_manager = media_manager.clone();
        let data_manager = data_manager.clone();
        move || {
            let main_window = main_window.unwrap();
            media_manager.initialize();
            media_manager.clone().connect_callbacks();

            main_window
                .window()
                .with_winit_window(|window| {
                    let mut monitors_map = monitors.lock().unwrap();
                    *monitors_map = window
                        .available_monitors()
                        .flat_map(|m| m.name().map(|n| (n, m)))
                        .collect::<HashMap<_, _>>();

                    let settings = main_window.global::<Settings>();
                    let mut monitor_names: Vec<String> = monitors_map.keys().cloned().collect();
                    monitor_names.sort();

                    settings.set_monitors(ModelRc::from(
                        monitor_names
                            .iter()
                            .map(|n| n.to_shared_string())
                            .collect::<Vec<_>>()
                            .as_slice(),
                    ));

                    let Some(current) = window.current_monitor() else {
                        return;
                    };

                    let Some(current_name) = current.name() else {
                        return;
                    };

                    let app_settings = data_manager.load::<AppSettings>();

                    let target_screen = if let Some(ref last_screen) = app_settings.last_screen {
                        if monitors_map.contains_key(last_screen) && last_screen != &current_name {
                            Some((
                                last_screen.clone(),
                                monitors_map.get(last_screen).unwrap().clone(),
                            ))
                        } else {
                            monitors_map
                                .iter()
                                .find(|(name, _)| **name != current_name)
                                .map(|(name, monitor)| (name.clone(), monitor.clone()))
                        }
                    } else {
                        monitors_map
                            .iter()
                            .find(|(name, _)| **name != current_name)
                            .map(|(name, monitor)| (name.clone(), monitor.clone()))
                    };

                    let Some((screen_name, second_screen)) = target_screen else {
                        let view_window = view_window.unwrap();
                        view_window.show().unwrap();
                        return;
                    };

                    let mut updated_settings = app_settings;
                    updated_settings.last_screen = Some(screen_name.clone());
                    data_manager.save(&updated_settings);

                    if let Some(idx) = monitor_names.iter().position(|n| n == &screen_name) {
                        settings.set_selected_monitor(idx as _);
                    }

                    let view_window = view_window.unwrap();
                    let state = view_window.global::<ViewState>();
                    state.set_window_width(second_screen.size().width as _);
                    state.set_window_height(second_screen.size().height as _);

                    slint::spawn_local({
                        let view_window = view_window.as_weak();
                        let second_screen = second_screen.clone();
                        async move {
                            let view_window = view_window.unwrap();
                            let w = view_window.window().winit_window().await.unwrap();
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

            view_state.set_shared_view(main_state.get_shared_view());
        }
    });

    main_window.on_change_monitor({
        let view_window = view_window.as_weak();
        let monitors = monitors.clone();
        let data_manager = data_manager.clone();
        move |name| {
            let monitors_map = monitors.lock().unwrap();
            let Some(monitor) = monitors_map.get(&name.to_string()) else {
                return;
            };

            let mut app_settings = data_manager.load::<AppSettings>();
            app_settings.last_screen = Some(name.to_string());
            data_manager.save(&app_settings);

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

    main_window.on_open_settings({
        let settings_window = settings_window.as_weak();
        move || {
            settings_window.unwrap().show().unwrap();
        }
    });

    main_window.on_shutdown_output({
        let main_window = main_window.as_weak();
        let view_window = view_window.as_weak();
        let media_manager = media_manager.clone();
        move || {
            let main_window = main_window.unwrap();
            let view_window = view_window.unwrap();
            let state = view_window.global::<ViewState>();
            let mut shared = state.get_shared_view();
            shared.content = SharedString::default();
            state.set_shared_view(shared.clone());
            main_window.global::<ViewState>().set_shared_view(shared);

            state.set_off(!state.get_off());
            media_manager.stop_output_video();
        }
    });

    main_window.on_clear_output({
        let main_window = main_window.as_weak();
        let view_window = view_window.as_weak();
        let media_manager = media_manager.clone();
        move || {
            let view_window = view_window.unwrap();
            let main_window = main_window.unwrap();
            let state = view_window.global::<ViewState>();

            let mut shared = state.get_shared_view();
            shared.content = SharedString::default();

            state.set_shared_view(shared.clone());
            main_window.global::<ViewState>().set_shared_view(shared);

            state.set_off(false);
            media_manager.stop_output_video();
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
