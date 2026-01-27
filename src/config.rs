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
use windows::Win32::Foundation::COLORREF;

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
pub struct Resolution {
    pub height: i32,
    pub width: i32,
}

impl Resolution {
    fn new(width: impl Into<i32>, height: impl Into<i32>) -> Self {
        Self {
            width: width.into(),
            height: height.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Color(pub u8, pub u8, pub u8);

impl From<Color> for COLORREF {
    fn from(color: Color) -> Self {
        let r = color.0 as u32;
        let g = color.1 as u32;
        let b = color.2 as u32;

        COLORREF(b << 16 | g << 8 | r)
    }
}
impl From<Color> for iced::Color {
    fn from(color: Color) -> Self {
        iced::Color::from_rgb8(color.0, color.1, color.2)
    }
}

impl From<iced::Color> for Color {
    fn from(color: iced::Color) -> Self {
        let [r, g, b, _] = color.into_rgba8();

        Color(r, g, b)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub thin: Resolution,
    pub tall: Resolution,
    pub wide: Resolution,
    pub ruler: i32,
    pub thin_key: Option<KeyFilter>,
    pub tall_key: Option<KeyFilter>,
    pub wide_key: Option<KeyFilter>,
    pub colors: [Color; 2],
}

impl Default for Config {
    fn default() -> Config {
        Self {
            tall: Resolution::new(384, 16384),
            thin: Resolution::new(400, 1800),
            wide: Resolution::new(1920, 300),
            ruler: 19,
            thin_key: Some(KeyFilter {
                char: 'h',
                modifiers: Some(Modifiers::default()),
            }),
            tall_key: Some(KeyFilter {
                char: 'h',
                modifiers: Some(Modifiers {
                    shift: true,
                    ..Modifiers::default()
                }),
            }),
            wide_key: Some(KeyFilter {
                char: 'h',
                modifiers: Some(Modifiers {
                    ctrl: true,
                    ..Modifiers::default()
                }),
            }),
            colors: [Color(91, 207, 250), Color(245, 171, 185)],
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
                self.thin_key = key_filter;
            }
            Hotkey::Tall => {
                self.tall_key = key_filter;
            }
            Hotkey::Wide => {
                self.wide_key = key_filter;
            }
        }
        self
    }

    pub fn get_hotkey(&self, hotkey: Hotkey) -> Option<KeyFilter> {
        match hotkey {
            Hotkey::Thin => self.thin_key,
            Hotkey::Tall => self.tall_key,
            Hotkey::Wide => self.wide_key,
        }
    }
}
