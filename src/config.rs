use std::{
    fs::{File, create_dir_all},
    sync::LazyLock,
};

use directories::ProjectDirs;
use figment::{
    Figment,
    providers::{Format, Serialized, Toml},
};
use serde::{Deserialize, Serialize};
use std::io::Write;

use crate::keylogger::{KeyFilter, Modifiers};

static PROJECT_DIR: LazyLock<ProjectDirs> =
    LazyLock::new(|| ProjectDirs::from("", "", "wrinkle").unwrap());

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hotkey {
    Thin,
    Tall,
    Wide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub width: i32,
    pub height: i32,
    pub ruler: i32,
    pub thin: Option<KeyFilter>,
    pub tall: Option<KeyFilter>,
    pub wide: Option<KeyFilter>,
}

impl Default for Config {
    fn default() -> Config {
        Self {
            height: 16384,
            width: 384,
            ruler: 19,
            thin: Some(KeyFilter {
                char: 'h',
                modifiers: Some(Modifiers::default()),
            }),
            tall: Some(KeyFilter {
                char: 'h',
                modifiers: Some(Modifiers {
                    shift: true,
                    ..Modifiers::default()
                }),
            }),
            wide: Some(KeyFilter {
                char: 'h',
                modifiers: Some(Modifiers {
                    ctrl: true,
                    ..Modifiers::default()
                }),
            }),
        }
    }
}

impl Config {
    pub fn load_from_file() -> Self {
        let config_dir = PROJECT_DIR.config_dir();
        let buf = config_dir.join("config.toml");
        let config_file = buf.to_str().unwrap();

        let config = Figment::from(Serialized::defaults(Config::default()))
            .merge(Toml::file(config_file))
            .extract()
            .unwrap();

        config
    }

    pub fn save_to_file(&self) -> std::io::Result<()> {
        let config_dir = PROJECT_DIR.config_dir();
        let buf = config_dir.join("config.toml");
        let config_file = buf.to_str().unwrap();

        create_dir_all(config_dir)?;
        let mut file = File::create(config_file)?;
        write!(file, "{}", toml::to_string(self).unwrap())?;
        file.flush()?;

        Ok(())
    }

    pub fn set_hotkey(mut self, hotkey: Hotkey, key_filter: Option<KeyFilter>) -> Self {
        match hotkey {
            Hotkey::Thin => {
                self.thin = key_filter;
            }
            Hotkey::Tall => {
                self.tall = key_filter;
            }
            Hotkey::Wide => {
                self.wide = key_filter;
            }
        }
        self
    }

    pub fn get_hotkey(&self, hotkey: Hotkey) -> Option<KeyFilter> {
        match hotkey {
            Hotkey::Thin => self.thin,
            Hotkey::Tall => self.tall,
            Hotkey::Wide => self.wide,
        }
    }
}
