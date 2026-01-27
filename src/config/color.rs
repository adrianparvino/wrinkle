use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::COLORREF;

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
