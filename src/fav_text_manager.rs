use slint::{ComponentHandle, ModelRc, SharedString, Weak};
use std::sync::{Arc, Mutex};

use crate::settings::FavoriteTexts;
use crate::user_data::UserData;
use ui::{MainState, MainWindow};

pub struct FavTextManager {
    data: Arc<UserData>,
    window: Weak<MainWindow>,

    fav_cache: Arc<Mutex<FavoriteTexts>>,
}

impl FavTextManager {
    pub fn new(window: Weak<MainWindow>, data: Arc<UserData>) -> Self {
        let fav_cache = Arc::new(Mutex::new(data.load::<FavoriteTexts>()));

        Self {
            data,
            window,
            fav_cache,
        }
    }

    pub fn initialize(&self) {
        let window = self.window.unwrap();
        let state = window.global::<MainState>();
        let favs = self.fav_cache.lock().unwrap();
        let favs = favs.iter().map(SharedString::from).collect::<Vec<_>>();

        state.set_saved_texts(ModelRc::from(favs.as_slice()));
    }

    pub fn connect_callbacks(&self) {
        let window = self.window.unwrap();

        // ---- GUARDAR TEXTO ----
        window.on_save_text({
            let fav_cache = self.fav_cache.clone();
            let data = self.data.clone();
            let window = self.window.clone();

            move |text| {
                let t = text.trim().to_string();
                if t.is_empty() {
                    return;
                }

                let mut favs = fav_cache.lock().unwrap();

                // evitar duplicados
                if !favs.contains(&t) {
                    favs.push(t.clone());
                    data.save(&*favs);
                }

                if let Some(window) = window.upgrade() {
                    let state = window.global::<MainState>();
                    let favs = favs.iter().map(SharedString::from).collect::<Vec<_>>();
                    state.set_saved_texts(ModelRc::from(favs.as_slice()));
                }
            }
        });

        // ---- BORRAR TEXTO ----
        window.on_remove_saved_text({
            let fav_cache = self.fav_cache.clone();
            let data = self.data.clone();
            let window = self.window.clone();

            move |index| {
                let mut favs = fav_cache.lock().unwrap();

                if index >= 0 && (index as usize) < favs.len() {
                    favs.remove(index as usize);
                    data.save(&*favs);
                }

                if let Some(window) = window.upgrade() {
                    let state = window.global::<MainState>();
                    let favs = favs.iter().map(SharedString::from).collect::<Vec<_>>();
                    state.set_saved_texts(ModelRc::from(favs.as_slice()));
                }
            }
        });
    }
}
