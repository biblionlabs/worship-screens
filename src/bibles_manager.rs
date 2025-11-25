use slint::{ComponentHandle, Model, ModelRc, Weak};
use std::sync::{Arc, Mutex};

use setup_core::{BibleInstallStatus, Setup, SqliteDbSink};
use ui::{Bible, MainState, MainWindow};

pub struct BiblesManager {
    setup: Arc<Setup>,
    database: Arc<SqliteDbSink>,
    window: Weak<MainWindow>,
    bibles_cache: Arc<Mutex<Vec<BibleItem>>>,
}

#[derive(Clone, Debug)]
struct BibleItem {
    id: String,
    name: String,
    english_name: String,
    language: String,
    installed: bool,
    installing: bool,
    progress: f32,
}

impl From<BibleItem> for Bible {
    fn from(item: BibleItem) -> Self {
        Self {
            id: item.id.into(),
            name: item.name.into(),
            english_name: item.english_name.into(),
            installed: item.installed,
            installing: item.installing,
            progress: item.progress,
        }
    }
}

impl BiblesManager {
    pub fn new(window: Weak<MainWindow>, setup: Arc<Setup>, database: Arc<SqliteDbSink>) -> Self {
        Self {
            setup,
            database,
            window,
            bibles_cache: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn initialize(&self) {
        let mut cache = self.bibles_cache.lock().unwrap();

        if let Ok(bibles) = self.setup.list_bibles(self.database.as_ref()) {
            *cache = bibles
                .iter()
                .map(|(id, name, english, lang, status)| BibleItem {
                    id: id.clone(),
                    name: name.clone(),
                    english_name: english.clone(),
                    language: lang.clone(),
                    installed: status.is_complete(),
                    installing: false,
                    progress: Self::calculate_progress(status),
                })
                .collect();
        }

        self.update_ui_from_cache(&cache);
    }

    pub fn connect_callbacks(&self) {
        self.on_search();
        self.on_select_bible();
    }

    fn on_search(&self) {
        let window = self.window.unwrap();
        let bibles_cache = self.bibles_cache.clone();

        window.global::<MainState>().on_search_bible({
            let window = self.window.clone();
            let bibles_cache = bibles_cache.clone();

            move |query| {
                let cache = bibles_cache.lock().unwrap();
                let query_lower = query.to_lowercase();

                let filtered: Vec<Bible> = if query_lower.is_empty() {
                    cache.iter().cloned().map(Bible::from).collect()
                } else {
                    cache
                        .iter()
                        .filter(|bible| {
                            bible.name.to_lowercase().contains(&query_lower)
                                || bible.english_name.to_lowercase().contains(&query_lower)
                        })
                        .cloned()
                        .map(Bible::from)
                        .collect()
                };

                slint::invoke_from_event_loop({
                    let window = window.clone();
                    move || {
                        if let Some(window) = window.upgrade() {
                            let state = window.global::<MainState>();
                            state.set_bibles(ModelRc::from(filtered.as_slice()));
                        }
                    }
                })
                .ok();
            }
        });
    }

    fn on_select_bible(&self) {
        let window = self.window.unwrap();
        let setup = self.setup.clone();
        let database = self.database.clone();
        let bibles_cache = self.bibles_cache.clone();

        window.global::<MainState>().on_install_bible({
            let window = self.window.clone();

            move |bible_id| {
                let bible_id_str = bible_id.to_string();

                {
                    let mut cache = bibles_cache.lock().unwrap();
                    if let Some(bible) = cache.iter_mut().find(|b| b.id == bible_id_str) {
                        bible.installing = true;
                        bible.progress = 0.0;
                    }
                }

                slint::invoke_from_event_loop({
                    let window = window.clone();
                    let bible_id = bible_id.clone();

                    move || {
                        if let Some(window) = window.upgrade() {
                            let state = window.global::<MainState>();
                            let bibles = state.get_bibles();

                            if let Some(idx) = bibles.iter().position(|b| b.id == bible_id) {
                                if let Some(mut bible) = bibles.row_data(idx) {
                                    bible.installing = true;
                                    bible.progress = 0.0;
                                    bibles.set_row_data(idx, bible);
                                }
                            }
                        }
                    }
                })
                .ok();

                std::thread::spawn({
                    let setup = setup.clone();
                    let database = database.clone();

                    move || {
                        if let Err(e) = setup.install_bibles(database.as_ref(), &[bible_id_str]) {
                            eprintln!("Error installing bible: {}", e);
                        }
                    }
                });
            }
        });
    }

    pub fn update_progress(&self, bible_id: &str, current: u64, total: u64) {
        let is_complete = current == total;
        let progress = if total > 0 {
            current as f32 / total as f32
        } else {
            0.0
        };

        {
            let mut cache = self.bibles_cache.lock().unwrap();
            if let Some(bible) = cache.iter_mut().find(|b| b.id == bible_id) {
                bible.installing = !is_complete;
                bible.installed = is_complete;
                bible.progress = progress;
            }
        }

        let window = self.window.unwrap();
        let state = window.global::<MainState>();
        let bibles = state.get_bibles();

        if let Some(idx) = bibles.iter().position(|b| b.id == bible_id) {
            if let Some(mut bible) = bibles.row_data(idx) {
                bible.installing = !is_complete;
                bible.installed = is_complete;
                bible.progress = progress;
                bibles.set_row_data(idx, bible);
            }
        }
    }

    fn update_ui_from_cache(&self, cache: &[BibleItem]) {
        let window = self.window.unwrap();
        let state = window.global::<MainState>();

        let ui_bibles: Vec<Bible> = cache.iter().cloned().map(Bible::from).collect();

        state.set_bibles(ModelRc::from(ui_bibles.as_slice()));
    }

    fn calculate_progress(status: &BibleInstallStatus) -> f32 {
        status.completion_percentage() / 100.0
    }
}
