use std::env;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::process::Command;

use serde::Serialize;
use serde::de::DeserializeOwned;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");

pub struct UserData {
    data_path: PathBuf,
}

pub trait Save: Default + Serialize + DeserializeOwned {
    const NAME: &str;
}

impl Default for UserData {
    fn default() -> Self {
        let data_path = app_data_dir();

        if !data_path.exists() {
            std::fs::create_dir_all(&data_path).unwrap();
        }

        Self { data_path }
    }
}

impl UserData {
    fn file<T: Save>(&self) -> PathBuf {
        let mut file = self.data_path.join(T::NAME);
        file.set_extension("json");
        file
    }

    pub fn data_dir(&self, sub_dir: &[impl AsRef<Path>]) -> PathBuf {
        if sub_dir.is_empty() {
            return self.data_path.clone();
        }

        sub_dir
            .iter()
            .fold(self.data_path.clone(), |path, b| path.join(b))
    }

    pub fn load<T: Save>(&self) -> T {
        let file = self.file::<T>();
        let content = std::fs::read_to_string(file).unwrap_or_default();

        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn save<T: Save>(&self, value: &T) {
        let file = self.file::<T>();
        let content = serde_json::to_string(value).unwrap_or_default();

        std::fs::write(file, content).unwrap_or_default();
    }
}

fn app_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = env::var_os("APPDATA") {
            return PathBuf::from(appdata).join(APP_NAME);
        }

        if let Some(localappdata) = env::var_os("LOCALAPPDATA") {
            return PathBuf::from(localappdata).join(APP_NAME);
        }

        if let Some(userprofile) = env::var_os("USERPROFILE") {
            return PathBuf::from(userprofile)
                .join("AppData")
                .join("Roaming")
                .join(APP_NAME);
        }

        if let Some(user) = env::var_os("USERNAME") {
            let base = PathBuf::from("C:\\Users").join(user);
            return base.join("AppData").join("Roaming").join(APP_NAME);
        }

        return env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(APP_NAME);
    }

    #[cfg(any(target_os = "macos", target_os = "linux",))]
    {
        let home = env::var_os("HOME")
            .and_then(|h| h.into_string().ok())
            .or_else(|| {
                if let Some(user) = env::var_os("USER") {
                    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
                    let path = PathBuf::from("/home").join(&user);
                    #[cfg(any(target_os = "macos", target_os = "ios"))]
                    let path = PathBuf::from("/Users").join(&user);
                    return path.to_str().map(ToString::to_string);
                }

                #[cfg(unix)]
                return Command::new("id")
                    .arg("-u")
                    .output()
                    .ok()
                    .and_then(|output| {
                        if output.status.success() {
                            return String::from_utf8_lossy(&output.stdout)
                                .trim()
                                .parse::<i32>()
                                .ok()
                                .and_then(|user| {
                                    if user == 0 {
                                        return Some("/root".to_string());
                                    }
                                    None
                                });
                        }
                        None
                    })
                    .or_else(|| {
                        Command::new("whoami").output().ok().and_then(|output| {
                            if output.status.success() {
                                let user =
                                    String::from_utf8_lossy(&output.stdout).trim().to_string();
                                if !user.is_empty() {
                                    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
                                    let path = PathBuf::from("/home").join(&user);
                                    #[cfg(any(target_os = "macos", target_os = "ios"))]
                                    let path = PathBuf::from("/Users").join(&user);
                                    return path.to_str().map(ToString::to_string);
                                }
                            }
                            None
                        })
                    });
                #[cfg(not(unix))]
                None
            });

        let home_path = home.map_or_else(
            || env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            PathBuf::from,
        );

        #[cfg(target_os = "macos")]
        return home_path
            .join("Library")
            .join("Application Support")
            .join(APP_NAME);

        #[cfg(target_os = "linux")]
        {
            if let Some(xdg) = env::var_os("XDG_DATA_HOME") {
                use std::path::PathBuf;

                return PathBuf::from(xdg).join(APP_NAME);
            }
            home_path.join(".local").join("share").join(APP_NAME)
        }
    }
}
