#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use notify_rust::Notification;
use setup_core::{TantivySink, event};
use slint::winit_030::WinitWindowAccessor;
use slint::winit_030::winit::monitor::MonitorHandle;
use slint::{ComponentHandle, Model, ModelRc, SharedString, ToSharedString};
use tracing::error;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

use ui::*;

use self::bibles_manager::BiblesManager;
use self::check_update::check_for_updates;
use self::fav_text_manager::FavTextManager;
use self::media_manager::MediaManager;
use self::schedule_manager::ScheduleManager;
use self::settings::AppSettings;
use self::song_manager::SongsManager;
use self::user_data::UserData;
use self::utils::list_system_fonts;

mod bibles_manager;
mod bitstream_converter;
mod check_update;
mod fav_text_manager;
mod media_manager;
mod schedule_manager;
mod settings;
mod song_manager;
mod user_data;
mod utils;

fn main() {
    let monitors: Arc<Mutex<HashMap<String, MonitorHandle>>> = Default::default();
    let data_manager: Arc<UserData> = Default::default();

    // Start Traing
    let builder = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_suffix(".log")
        .build(data_manager.data_dir(&["logs"]))
        .unwrap();
    let (non_blocking, _guard) = tracing_appender::non_blocking(builder);
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .try_from_env()
                .unwrap_or_default(),
        )
        .with_writer(non_blocking)
        .init();

    let main_window = MainWindow::new().unwrap();
    let view_window = ViewWindow::new().unwrap();
    main_window
        .global::<ViewState>()
        .set_installed_fonts(ModelRc::from(list_system_fonts().as_slice()));

    let mut song_manager = SongsManager::new(main_window.as_weak(), data_manager.clone());
    song_manager.initialize();
    song_manager.connect_callbacks();

    let fav_manager = FavTextManager::new(main_window.as_weak(), data_manager.clone());
    fav_manager.initialize();
    fav_manager.connect_callbacks();

    let schedule_manager = ScheduleManager::new(main_window.as_weak(), Arc::new(song_manager));
    schedule_manager.initialize();
    schedule_manager.connect_callbacks();

    let media_manager = Arc::new(MediaManager::new(
        main_window.as_weak(),
        view_window.as_weak(),
        data_manager.clone(),
    ));
    let bibles_manager = Arc::new(OnceLock::<BiblesManager>::new());

    let cache_dir = data_manager.data_dir(&["cache"]);
    let need_update = check_for_updates(&cache_dir);

    let database = Arc::new(TantivySink::from(data_manager.data_dir(&["index"])));
    let source_variants = setup_core::SetupBuilder::new().cache_path(cache_dir)
        // Add Reina Valera 1960 Bible
        .add_bible_from_url(
            "spa_rv1960",
            "https://raw.githubusercontent.com/biblionlabs/extra_data_source/refs/heads/main/bibles/spa_rv1960/manifest.json", 
            "https://raw.githubusercontent.com/biblionlabs/extra_data_source/refs/heads/main/bibles/spa_rv1960/desc.json",
            Some("https://raw.githubusercontent.com/biblionlabs/extra_data_source/refs/heads/main/bibles/{bible_id}/books/{book}.json")
        )
        .on::<event::Error>({
            move |e| {
                Notification::new().summary("Worship Screens Failed to install Bible").body(&e).show().inspect_err(|e| error!("{e}")).unwrap();
            }})
        .on::<event::Progress>({
            let main_window = main_window.as_weak();
        let bibles_manager = bibles_manager.clone();
            move |(step_id, current, total)| {
                if step_id == "crossrefs" {
                    return;
                }
                slint::invoke_from_event_loop({
                    let main_window = main_window.clone();
                    let bibles_manager = bibles_manager.clone();
                    let step_id = step_id.clone();
                    move || {
                        if let Some(window) = main_window.upgrade() {
                            let state = window.global::<MainState>();
                            let bibles = state.get_bibles();
                            if let Some((idx, mut bible)) = bibles.iter().position(|b| b.id == step_id).and_then(|row| bibles.row_data(row).map(|b| (row, b))) {
                                bible.installing = current != total;
                                bible.installed = current == total;
                                bible.progress = current as f32 / total as f32;
                                if current == total {
                                    _ = Notification::new()
                                        .summary("Worship Screens Bible Installed")
                                        .body(&format!("{} success installed", bible.name.as_str()))
                                        .show()
                                        .inspect_err(|e| error!("{e}"));
                                }
                                bibles.set_row_data(idx, bible);
                            }
                        }
                        if let Some(bibles_manager) = bibles_manager.get() {
                            bibles_manager.update_progress(&step_id, current, total);
                        }
                    }
                }).unwrap();
            }})
        .build().1;

    let source_variants: Arc<setup_core::Setup> = Arc::new(source_variants);

    _ = bibles_manager.set(BiblesManager::new(
        main_window.as_weak(),
        source_variants.clone(),
        database.clone(),
    ));

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

    main_window.on_open(move |url| {
        let _ = open::that(url.as_str())
            .inspect_err(|e| error!("Failed to open URL: {} => {e}", url.as_str()));
    });
    main_window.on_open_release({
        let need_update = need_update.clone();
        move || {
            if let Some(latest_release) = need_update.as_ref() {
                let _ = open::that(&latest_release.html_url).inspect_err(|e| {
                    error!("Failed to open URL: {} => {e}", latest_release.html_url)
                });
            }
        }
    });

    main_window.on_start_window({
        let main_window = main_window.as_weak();
        let monitors = monitors.clone();
        let view_window = view_window.as_weak();
        let media_manager = media_manager.clone();
        let data_manager = data_manager.clone();
        let bibles_manager = bibles_manager.clone();
        move || {
            let main_window = main_window.unwrap();
            media_manager.initialize();
            media_manager.clone().connect_callbacks();

            if let Some(bibles_manager) = bibles_manager.get() {
                bibles_manager.initialize();
                bibles_manager.connect_callbacks();
            }

            main_window
                .global::<MainState>()
                .set_need_update(need_update.is_some());

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

                    // ---------- NEW: show changelog dialog ----------
                    let current_version = env!("CARGO_PKG_VERSION");
                    let last_seen = updated_settings.last_seen_version.clone();

                    if last_seen.as_deref() != Some(current_version) {
                        let raw_env = env!("LAST_CHANGELOG");
                        let lines = utils::parse_last_changelog_to_markdown_lines(raw_env);
                        let main_state = main_window.global::<MainState>();

                        main_state.set_last_changelog(ModelRc::from(lines.as_slice()));

                        main_state.set_show_changelog_on_start(true);

                        let mut save_settings = updated_settings.clone();
                        save_settings.last_seen_version = Some(current_version.to_string());
                        data_manager.save(&save_settings);
                    }

                    let mut main_shared_view = main_window.global::<ViewState>().get_shared_view();
                    if let Some(font) = updated_settings.content_font {
                        main_shared_view.font = font;
                    }
                    if let Some(font) = updated_settings.verse_font {
                        main_shared_view.verse_font = font;
                    }
                    main_window
                        .global::<ViewState>()
                        .set_shared_view(main_shared_view);

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

            let Ok(verses_found) =
                setup_core::service_db::SearchedVerse::from_search(s, database.verse_index())
            else {
                return;
            };
            let verses = verses_found
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

    main_window.on_shutdown_output({
        let view_window = view_window.as_weak();
        move || {
            let view_window = view_window.unwrap();
            let state = view_window.global::<ViewState>();

            state.set_off(!state.get_off());
        }
    });

    main_window.on_clear_image({
        let main_window = main_window.as_weak();
        let view_window = view_window.as_weak();
        let media_manager = media_manager.clone();
        move || {
            media_manager.stop_preview_video();
            media_manager.stop_output_video();
            let view_window = view_window.unwrap();
            let main_window = main_window.unwrap();
            let state = view_window.global::<ViewState>();

            let mut shared = state.get_shared_view();
            shared.show_img = false;

            state.set_shared_view(shared.clone());
            main_window.global::<ViewState>().set_shared_view(shared);
        }
    });

    main_window.on_clear_output({
        let main_window = main_window.as_weak();
        let view_window = view_window.as_weak();
        move || {
            let view_window = view_window.unwrap();
            let main_window = main_window.unwrap();
            let state = view_window.global::<ViewState>();

            let mut shared = state.get_shared_view();
            shared.content = SharedString::default();
            shared.verse = SharedString::default();

            state.set_shared_view(shared.clone());
            main_window.global::<ViewState>().set_shared_view(shared);

            state.set_off(false);
        }
    });

    main_window.window().on_close_requested({
        let main_window = main_window.as_weak();
        let view_window = view_window.as_weak();
        let data_manager = data_manager.clone();
        move || {
            let main_window = main_window.unwrap();
            let mut settings = data_manager.load::<AppSettings>();
            let shared_view = main_window.global::<ViewState>().get_shared_view();

            settings.content_font.replace(shared_view.font);
            settings.verse_font.replace(shared_view.verse_font);

            data_manager.save(&settings);

            view_window.unwrap().hide().unwrap();
            slint::CloseRequestResponse::HideWindow
        }
    });

    main_window.run().unwrap();
}
