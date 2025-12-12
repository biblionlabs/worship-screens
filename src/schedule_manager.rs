use slint::{ComponentHandle, ModelRc, SharedString, Weak};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use ui::{MainWindow, ScheduleState, ScheduledItem, ScheduledKind, ViewData};

pub struct ScheduleManager {
    window: Weak<MainWindow>,
    schedule_cache: Arc<Mutex<Vec<ScheduledItem>>>,
    id_counter: Arc<AtomicUsize>,
}

impl ScheduleManager {
    pub fn new(window: Weak<MainWindow>) -> Self {
        Self {
            window,
            schedule_cache: Arc::new(Mutex::new(Vec::new())),
            id_counter: Arc::new(AtomicUsize::new(1)),
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
            move |vd: ViewData, kind: ScheduledKind, label: SharedString| {
                println!(
                    "ScheduleManager: received add-processed-item kind={:?} label={}",
                    kind, label
                );
                let raw = id_counter.fetch_add(1, Ordering::SeqCst);

                let item = ScheduledItem {
                    id: raw as i32,
                    kind,
                    label: label.clone(),
                    view_data: vd,
                };

                {
                    let mut guard = cache.lock().unwrap();
                    guard.push(item);

                    if let Some(window) = window_weak.upgrade() {
                        let state = window.global::<ScheduleState>();
                        state.set_items(ModelRc::from(guard.as_slice()));
                    }
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

        window.on_schedule_request_move_up({
            let cache = cache.clone();
            let window_weak = window_weak.clone();
            move |index: i32| {
                let mut guard = cache.lock().unwrap();
                let idx = index as usize;
                if idx > 0 && idx < guard.len() {
                    guard.swap(idx - 1, idx);
                    if let Some(window) = window_weak.upgrade() {
                        let state = window.global::<ScheduleState>();
                        state.set_items(ModelRc::from(guard.as_slice()));
                    }
                }
            }
        });

        window.on_schedule_request_move_down({
            let cache = cache.clone();
            let window_weak = window_weak.clone();
            move |index: i32| {
                let mut guard = cache.lock().unwrap();
                let idx = index as usize;
                if idx + 1 < guard.len() {
                    guard.swap(idx + 1, idx);
                    if let Some(window) = window_weak.upgrade() {
                        let state = window.global::<ScheduleState>();
                        state.set_items(ModelRc::from(guard.as_slice()));
                    }
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

        window.on_schedule_request_send({
            move |index: i32| {
                eprintln!("schedule_request_send called for index={}", index);
            }
        });
    }
}
