use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};
use ui::{FileItem, ViewFontData};

use crate::media_manager::MediaItem;
use crate::user_data::Save;

macro_rules! impl_deref {
    ($( $n:ident ( $t:ty ) : $f:literal ),*) => {
        $(
        #[derive(Clone, Default, Deserialize, Serialize)]
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

        impl From<$t> for $n {
            fn from(v: $t) -> Self {
                Self(v)
            }
        }
        )*
    };
}

impl_deref! {
    FavoriteTexts(Vec<String>): "fav_texts",
    SourceSongs(Vec<FileItem>): "source_songs",
    SourceMedia(Vec<MediaItem>): "source_media"
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct AppSettings {
    pub last_screen: Option<String>,
    pub last_seen_version: Option<String>,
    pub content_font: Option<ViewFontData>,
    pub verse_font: Option<ViewFontData>,
}

impl Save for AppSettings {
    const NAME: &str = "settings";
}
