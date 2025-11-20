use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use ui::FileItem;

macro_rules! impl_deref {
    ($($n:ty ( $t:ty ) : $f:literal),*) => {
        #[derive(Default, Deserialize, Serialize)]
        pub struct $n($t);

        $(impl Deref for $n {
            type Target = $t;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $n {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        })*

        impl Save for $n {
            const NAME: &str = $f;
        }
    };
}

impl_deref! {
    FavoriteTexts(Vec<String>): "fav_texts",
    SourceSongs(Vec<FileItem>): "songs",
}
