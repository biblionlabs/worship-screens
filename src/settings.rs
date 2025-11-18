use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
pub struct FavoriteTexts(Vec<String>);

impl Deref for FavoriteTexts {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FavoriteTexts {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
