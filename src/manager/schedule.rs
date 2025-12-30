use slint::{ComponentHandle, ModelRc, SharedString, Weak};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use tracing::debug;

use ui::{MainWindow, ScheduleState, ScheduledItem, ScheduledKind, ViewData, ViewState};

use super::SongsManager;

pub struct ScheduleManager {
    window: Weak<MainWindow>,
    schedule_cache: Arc<Mutex<Vec<ScheduledItem>>>,
    id_counter: Arc<AtomicI32>,
    song_manager: Arc<SongsManager>,
}

impl ScheduleManager {
    pub fn new(window: Weak<MainWindow>, song_manager: Arc<SongsManager>) -> Self {
        Self {
            window,
            song_manager,
            schedule_cache: Arc::new(Mutex::new(Vec::new())),
            id_counter: Arc::new(AtomicI32::new(1)),
        }
    }

    pub fn initialize(&self) {
        if let Some(window) = self.window.upgrade() {
            let state = window.global::<ScheduleState>();
            let empty: Vec<ScheduledItem> = Vec::new();
            state.set_items(ModelRc::from(empty.as_slice()));
        }
    }

    pub fn connect_callbacks(&self) {
        let window = match self.window.upgrade() {
            Some(w) => w,
            None => return,
        };

        let cache = self.schedule_cache.clone();
        let window_weak = self.window.clone();
        let id_counter = self.id_counter.clone();

        window.on_add_processed_item({
            let cache = cache.clone();
            let window_weak = window_weak.clone();
            let id_counter = id_counter.clone();
            let song_manager = self.song_manager.clone();
            move |vd: ViewData, kind: ScheduledKind, label: SharedString| {
                debug!(
                    "ScheduleManager: received add-processed-item kind={kind:?} label={label} path={}",
                    vd.path
                );
                let id = id_counter.fetch_add(1, Ordering::SeqCst);

                let Some(window) = window_weak.upgrade() else {
                    return;
                };

                {
                    let mut guard = cache.lock().unwrap();
                    if kind == ScheduledKind::Song {
                        let state = window.global::<ViewState>().get_shared_view();
                        let Some(media) = song_manager
                            .songs_cache
                            .lock()
                            .map(|media| {
                                media.iter().find(|m| m.path.ends_with(label.as_str())).map(
                                    move |m| {
                                        m.content
                                            .iter()
                                            .map(|c| ScheduledItem {
                                                id,
                                                kind,
                                                label: c.clone(),
                                                view_data: ViewData {
                                                    content: c.clone(),
                                                    path: m.path.clone(),
                                                    ..state.clone()
                                                },
                                            })
                                            .collect::<Vec<_>>()
                                    },
                                )
                            })
                            .ok()
                            .flatten()
                        else {
                            return;
                        };
                        guard.extend_from_slice(media.as_slice());
                    } else {
                        guard.push(ScheduledItem {
                            id,
                            kind,
                            label: label.clone(),
                            view_data: vd,
                        });
                    }

                    let state = window.global::<ScheduleState>();
                    state.set_items(ModelRc::from(guard.as_slice()));
                }
            }
        });

        window.on_schedule_request_remove({
            let cache = cache.clone();
            let window_weak = window_weak.clone();
            move |index: i32| {
                let mut guard = cache.lock().unwrap();
                let idx = index as usize;
                if idx < guard.len() {
                    guard.remove(idx);
                    if let Some(window) = window_weak.upgrade() {
                        let state = window.global::<ScheduleState>();
                        state.set_items(ModelRc::from(guard.as_slice()));
                    }
                }
            }
        });

        window.on_schedule_request_move_by({
            let cache = cache.clone();
            let window_weak = window_weak.clone();
            move |start_index: i32, offset: i32| {
                let mut guard = cache.lock().unwrap();
                let len = guard.len();
                if len == 0 {
                    return;
                }
                let s = start_index as isize;
                if s < 0 || (s as usize) >= len {
                    return;
                }

                let dest_isize = s + (offset as isize);

                let mut dest = if dest_isize < 0 {
                    0usize
                } else if dest_isize as usize >= len {
                    len - 1
                } else {
                    dest_isize as usize
                };

                let s_usize = s as usize;
                let item = guard.remove(s_usize);

                if dest > guard.len() {
                    dest = guard.len();
                }

                guard.insert(dest, item);

                if let Some(window) = window_weak.upgrade() {
                    let state = window.global::<ScheduleState>();
                    state.set_items(ModelRc::from(guard.as_slice()));
                }
            }
        });

        window.on_schedule_request_clear({
            let cache = cache.clone();
            let window_weak = window_weak.clone();
            move || {
                let mut guard = cache.lock().unwrap();
                guard.clear();
                if let Some(window) = window_weak.upgrade() {
                    let state = window.global::<ScheduleState>();
                    let empty: Vec<ScheduledItem> = Vec::new();
                    state.set_items(ModelRc::from(empty.as_slice()));
                }
            }
        });
    }
}
