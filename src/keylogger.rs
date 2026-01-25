use std::{
    ffi::c_void,
    fmt::{Display, Write},
    sync::LazyLock,
};

use futures_channel::mpsc;
use serde::{Deserialize, Serialize};
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::{
                GetRawInputData, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RAWKEYBOARD,
                RID_INPUT, RIDEV_INPUTSINK, RIM_TYPEKEYBOARD, RegisterRawInputDevices,
            },
            WindowsAndMessaging::{
                DefWindowProcW, HWND_MESSAGE, RegisterClassExW, WINDOW_EX_STYLE, WINDOW_STYLE,
                WM_INPUT, WNDCLASSEXW,
            },
        },
    },
    core::PCWSTR,
};

use crate::{
    utils::UnsafeSync,
    wnd_class::{self, WndClass, wnd_proc},
};

#[derive(Clone, Copy, Debug, PartialEq, Default, Eq, Serialize, Deserialize)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub char: char,
    pub modifiers: Modifiers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyFilter {
    pub char: char,
    pub modifiers: Option<Modifiers>,
}

impl Display for KeyFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.modifiers {
            Some(modifiers) => {
                if modifiers.ctrl {
                    f.write_str("Ctrl+")?;
                }
                if modifiers.alt {
                    f.write_str("Alt+")?;
                }
                if modifiers.shift {
                    f.write_str("Shift+")?;
                }
            }
            None => {
                f.write_str("*+")?;
            }
        }
        f.write_char(self.char)
    }
}

impl KeyFilter {
    pub fn test(self, ev: KeyEvent) -> bool {
        if self.char != ev.char {
            return false;
        }
        if let Some(modifiers) = self.modifiers
            && modifiers != ev.modifiers
        {
            return false;
        }

        return true;
    }
}

impl Display for KeyEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.modifiers.ctrl {
            f.write_str("Ctrl+")?;
        }
        if self.modifiers.alt {
            f.write_str("Alt+")?;
        }
        if self.modifiers.shift {
            f.write_str("Shift+")?;
        }
        f.write_char(self.char)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyLogger {
    hwnd: HWND,
}

unsafe impl Send for KeyLogger {}
unsafe impl Sync for KeyLogger {}

struct KeyLoggerWnd {
    tx: mpsc::Sender<KeyEvent>,
    modifiers: Modifiers,
}

fn translate(ev: RAWKEYBOARD) -> Option<char> {
    let RAWKEYBOARD {
        MakeCode, Flags, ..
    } = ev;

    if Flags != 0 {
        return None;
    }

    match MakeCode {
        0x10 => Some('q'),
        0x11 => Some('w'),
        0x12 => Some('e'),
        0x13 => Some('r'),
        0x14 => Some('t'),
        0x15 => Some('y'),
        0x16 => Some('u'),
        0x17 => Some('i'),
        0x18 => Some('o'),
        0x19 => Some('p'),
        0x1E => Some('a'),
        0x1F => Some('s'),
        0x20 => Some('d'),
        0x21 => Some('f'),
        0x22 => Some('g'),
        0x23 => Some('h'),
        0x24 => Some('j'),
        0x25 => Some('k'),
        0x26 => Some('l'),
        0x2C => Some('z'),
        0x2D => Some('x'),
        0x2E => Some('c'),
        0x2F => Some('v'),
        0x30 => Some('b'),
        0x31 => Some('n'),
        0x32 => Some('m'),
        _ => None,
    }
}

impl WndClass for KeyLoggerWnd {
    fn get_class() -> &'static WNDCLASSEXW {
        static CLASS: LazyLock<UnsafeSync<WNDCLASSEXW>> = LazyLock::new(|| {
            let mut keylogger_class = WNDCLASSEXW::default();
            keylogger_class.cbSize = std::mem::size_of_val(&keylogger_class) as u32;
            keylogger_class.lpfnWndProc = Some(wnd_proc::<KeyLoggerWnd>);
            keylogger_class.lpszClassName =
                PCWSTR(widestring::u16cstr!("rawkbd_wndclass").as_ptr());
            keylogger_class.hInstance =
                unsafe { GetModuleHandleW(PCWSTR::default()).unwrap().into() };

            unsafe {
                RegisterClassExW(&raw const keylogger_class);
            }

            unsafe { UnsafeSync::new(keylogger_class) }
        });

        CLASS.get()
    }

    fn on_message(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            match msg {
                WM_INPUT => {
                    let Self { tx, modifiers } = self;

                    let mut input = RAWINPUT::default();
                    let mut rid_size = std::mem::size_of_val(&input) as u32;

                    if GetRawInputData(
                        HRAWINPUT(lparam.0 as *mut _),
                        RID_INPUT,
                        Some(&raw mut input as *mut c_void),
                        &mut rid_size as *mut _,
                        std::mem::size_of::<RAWINPUTHEADER>() as u32,
                    ) > 0
                        && input.header.dwType == RIM_TYPEKEYBOARD.0
                    {
                        match input.data.keyboard {
                            RAWKEYBOARD {
                                MakeCode: 0x2A,
                                Flags,
                                ..
                            } => {
                                modifiers.shift = Flags == 0;
                            }
                            RAWKEYBOARD {
                                MakeCode: 0x1D,
                                Flags,
                                ..
                            } => {
                                modifiers.ctrl = Flags == 0;
                            }
                            RAWKEYBOARD {
                                MakeCode: 0x38,
                                Flags,
                                ..
                            } => {
                                modifiers.alt = Flags == 0;
                            }
                            ev => {
                                if let Some(char) = translate(ev) {
                                    tx.start_send(KeyEvent {
                                        char,
                                        modifiers: *modifiers,
                                    })
                                    .unwrap();
                                }
                            }
                        }
                    }

                    DefWindowProcW(hwnd, msg, wparam, lparam)
                }
                _ => DefWindowProcW(hwnd, msg, wparam, lparam),
            }
        }
    }
}

impl KeyLogger {
    pub fn spawn(tx: mpsc::Sender<KeyEvent>) -> Self {
        let rawkbd_wnd = wnd_class::spawn(
            WINDOW_EX_STYLE::default(),
            WINDOW_STYLE::default(),
            PCWSTR::default(),
            Some(HWND_MESSAGE),
            None,
            Box::new(KeyLoggerWnd {
                tx,
                modifiers: Modifiers {
                    shift: false,
                    ctrl: false,
                    alt: false,
                },
            }),
        )
        .unwrap();

        let devs = RAWINPUTDEVICE {
            usUsagePage: 1,
            usUsage: 6,
            dwFlags: RIDEV_INPUTSINK,
            hwndTarget: rawkbd_wnd,
        };

        unsafe {
            RegisterRawInputDevices(&[devs], std::mem::size_of_val(&devs) as u32).unwrap();
        }

        Self { hwnd: rawkbd_wnd }
    }
}
