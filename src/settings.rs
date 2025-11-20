use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use ui::FileItem;

use crate::user_data::Save;

macro_rules! impl_deref {
    ($( $n:ident ( $t:ty ) : $f:literal ),*) => {
        $(
        #[derive(Default, Deserialize, Serialize)]
        pub struct $n($t);

        impl Deref for $n {
            type Target = $t;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl DerefMut for $n {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl Save for $n {
            const NAME: &str = $f;
        }
        )*
    };
}

impl_deref! {
    FavoriteTexts(Vec<String>): "fav_texts",
    SourceSongs(Vec<FileItem>): "source_songs"
}
