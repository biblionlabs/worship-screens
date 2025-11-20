use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use rfd::FileDialog;
use slint::{ComponentHandle, Model, ModelRc, SharedString, Weak};
use std::sync::Mutex;
use std::{fs, path::Path, sync::Arc};

use ui::{FileItem, MainWindow, SongsState};

use crate::settings::SourceSongs;
use crate::user_data::UserData;

pub struct SongsManager {
    pub data: Arc<UserData>,
    pub watcher: RecommendedWatcher,
    pub window: Weak<MainWindow>,

    songs_origin: Arc<Mutex<SourceSongs>>,
    songs_cache: Arc<Mutex<Vec<SongItem>>>,
}

#[derive(Clone)]
struct SongItem {
    content: Vec<SharedString>,
    path: SharedString,
}

impl From<SongItem> for ui::SongItem {
    fn from(value: SongItem) -> Self {
        Self {
            path: value.path,
            content: ModelRc::from(value.content.as_slice()),
        }
    }
}

impl SongsManager {
    pub fn new(window: Weak<MainWindow>, data: Arc<UserData>) -> Self {
        let songs_cache: Arc<Mutex<Vec<SongItem>>> = Default::default();
        let watcher = notify::recommended_watcher({
            let window = window.clone();
            let songs_cache = songs_cache.clone();
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                        if let Some(path) = event.paths.first() {
                            if path.is_file() {
                                let window = window.unwrap();
                                let state = window.global::<SongsState>();
                                let mut songs_cache = songs_cache.lock().unwrap();
                                process_file_into_state(path, &state, &mut songs_cache);
                            }
                        }
                    }
                }
            }
        })
        .unwrap();

        let songs_origin = Arc::new(Mutex::new(data.load()));

        Self {
            data,
            watcher,
            window,
            songs_cache,
            songs_origin,
        }
    }

    pub fn initialize(&mut self) {
        let window = self.window.unwrap();
        let state = window.global::<SongsState>();
        let songs_origin = self.songs_origin.lock().unwrap();
        let mut songs_cache = self.songs_cache.lock().unwrap();

        state.set_songs_origin(ModelRc::from(songs_origin.as_slice()));

        for item in songs_origin.iter() {
            if item.is_folder {
                let path = Path::new(&item.path);
                let _ = self.watcher.watch(path, RecursiveMode::Recursive);
                self.watch_and_process_folder(path, &mut songs_cache);
            } else {
                process_file_into_state(Path::new(&item.path), &state, &mut songs_cache);
            }
        }
    }

    pub fn connect_callbacks(&self) {
        let window = self.window.unwrap();
        let state = window.global::<SongsState>();

        // ---- Search Song ----
        state.on_on_search({
            let window = self.window.clone();
            let songs_cache = self.songs_cache.clone();
            move |s| {
                let s = s.trim().to_lowercase();
                let songs_cache = songs_cache.lock().unwrap();

                let filtered: Vec<ui::SongItem> = if s.is_empty() {
                    // restaurar todo
                    songs_cache
                        .iter()
                        .cloned()
                        .map(ui::SongItem::from)
                        .collect::<Vec<_>>()
                } else {
                    songs_cache
                        .iter()
                        .cloned()
                        .map(ui::SongItem::from)
                        .filter(|song| {
                            let name_ok = song.path.to_string().to_lowercase().contains(&s);

                            let content_ok = song
                                .content
                                .iter()
                                .any(|p| p.to_string().to_lowercase().contains(&s));

                            name_ok || content_ok
                        })
                        .collect()
                };

                if let Some(window) = window.upgrade() {
                    let state = window.global::<SongsState>();
                    state.set_songs(ModelRc::from(filtered.as_slice()));
                }
            }
        });

        // ---- open-file-dialog ----
        state.on_open_file_dialog({
            let window = self.window.clone();
            let data = self.data.clone();
            let songs_origin = self.songs_origin.clone();
            let songs_cache = self.songs_cache.clone();
            move |is_folder| {
                let path = if is_folder {
                    FileDialog::new().pick_folder()
                } else {
                    FileDialog::new().pick_file()
                };

                if let Some(path) = path {
                    let path_str = path.to_string_lossy().to_string();
                    let mut songs_origin = songs_origin.lock().unwrap();

                    songs_origin.push(FileItem {
                        path: path_str.into(),
                        is_folder,
                    });
                    data.save(&*songs_origin);
                    let window = window.unwrap();
                    let state = window.global::<SongsState>();

                    state.set_songs_origin(ModelRc::from(songs_origin.as_slice()));

                    let mut songs_cache = songs_cache.lock().unwrap();
                    if is_folder {
                        process_folder_recursive(&path, &state, &mut songs_cache);
                    } else {
                        process_file_into_state(&path, &state, &mut songs_cache);
                    }
                }
            }
        });

        // ---- remove-song-origin ----
        state.on_remove_song_origin({
            let data = self.data.clone();
            let window = self.window.clone();
            let songs_origin = self.songs_origin.clone();
            move |index| {
                let mut songs_origin = songs_origin.lock().unwrap();
                if index >= 0 && (index as usize) < songs_origin.len() {
                    songs_origin.remove(index as usize);
                    data.save(&*songs_origin);
                }
                let window = window.unwrap();
                let state = window.global::<SongsState>();

                state.set_songs_origin(ModelRc::from(songs_origin.as_slice()));
            }
        });
    }

    fn watch_and_process_folder(&self, folder: &Path, song_list: &mut Vec<SongItem>) {
        let window = self.window.unwrap();
        let state = window.global::<SongsState>();
        process_folder_recursive(folder, &state, song_list);
    }
}

fn process_folder_recursive<'a>(
    folder: &Path,
    state: &SongsState<'a>,
    song_list: &mut Vec<SongItem>,
) {
    if !folder.exists() {
        return;
    }

    let Ok(entries) = fs::read_dir(folder) else {
        return;
    };

    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            process_folder_recursive(&p, state, song_list);
        } else {
            process_file_into_state(&p, state, song_list);
        }
    }
}

fn process_file_into_state<'a>(path: &Path, state: &SongsState<'a>, song_list: &mut Vec<SongItem>) {
    if !path.is_file() {
        return;
    }

    let content = fs::read_to_string(path).unwrap_or_default();
    let paragraphs: Vec<String> = content
        .trim()
        .replace("\r\n", "\n")
        .split("\n\n")
        .map(|s| s.to_string())
        .collect();

    song_list.push(SongItem {
        path: path
            .with_extension("")
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned())
            .into(),
        content: paragraphs
            .into_iter()
            .map(|p| SharedString::from(p))
            .collect::<Vec<_>>(),
    });

    state.set_songs(ModelRc::from(
        song_list
            .iter()
            .cloned()
            .map(ui::SongItem::from)
            .collect::<Vec<_>>()
            .as_slice(),
    ));
}
