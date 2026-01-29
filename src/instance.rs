use std::{marker::PhantomData, sync::LazyLock};

use widestring::U16Str;
use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        Graphics::Gdi::{
            GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromWindow,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            DefWindowProcW, EnumWindows, GWL_STYLE, GetForegroundWindow, GetWindowLongW,
            GetWindowRect, GetWindowTextW, HSHELL_WINDOWCREATED, HWND_MESSAGE, RegisterClassExW,
            RegisterShellHookWindow, RegisterWindowMessageW, SWP_FRAMECHANGED, SWP_NOSENDCHANGING,
            SetWindowLongW, SetWindowPos, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSEXW, WS_BORDER,
            WS_DLGFRAME, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
        },
    },
    core::{BOOL, PCWSTR},
};

use crate::{
    utils::UnsafeSync,
    wnd_class::{self, WndClass, wnd_proc},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MinecraftInstance {
    pub hwnd: HWND,
}
unsafe impl Send for MinecraftInstance {}
unsafe impl Sync for MinecraftInstance {}

impl MinecraftInstance {
    pub fn new(hwnd: HWND) -> Self {
        Self { hwnd }
    }

    pub fn is_foreground(&self) -> bool {
        unsafe { self.hwnd == GetForegroundWindow() }
    }

    pub fn get_monitor_info(&self) -> MONITORINFO {
        unsafe {
            let monitor = MonitorFromWindow(self.hwnd, MONITOR_DEFAULTTOPRIMARY);
            let mut lpmi = MONITORINFO::default();
            lpmi.cbSize = std::mem::size_of_val(&lpmi) as u32;
            GetMonitorInfoW(monitor, &raw mut lpmi).unwrap();
            lpmi
        }
    }

    pub fn set_window_pos(&self, left: i32, top: i32, width: i32, height: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                None,
                left,
                top,
                width,
                height,
                SWP_NOSENDCHANGING | SWP_FRAMECHANGED,
            )
            .unwrap()
        };
    }

    pub fn get_window_rect(&self) -> RECT {
        let mut rect = RECT::default();
        unsafe {
            GetWindowRect(self.hwnd, &raw mut rect).unwrap();
        }
        rect
    }
}

struct MinecraftInstanceListenerWindow {
    cb: Box<dyn FnMut(HWND)>,
}

impl MinecraftInstanceListenerWindow {
    fn run_cb(&mut self, hwnd: HWND) {
        log::debug!("Found new minecraft instance: {:?}", hwnd);
        unsafe {
            let mut style = GetWindowLongW(hwnd, GWL_STYLE);
            style &= !(WS_BORDER
                | WS_DLGFRAME
                | WS_THICKFRAME
                | WS_MINIMIZEBOX
                | WS_MAXIMIZEBOX
                | WS_SYSMENU)
                .0 as i32;
            SetWindowLongW(hwnd, GWL_STYLE, style);
        }

        (self.cb)(hwnd)
    }
}

fn is_minecraft_window(hwnd: HWND) -> bool {
    unsafe {
        let mut str = [0u16; 256];
        let len = GetWindowTextW(hwnd, &mut str[..]) as usize;
        let str = U16Str::from_slice(&str[..len]);

        const MINECRAFT: &U16Str = widestring::u16str!("Minecraft*");

        str.as_slice().starts_with(MINECRAFT.as_slice())
    }
}

impl WndClass for MinecraftInstanceListenerWindow {
    fn get_class() -> &'static WNDCLASSEXW {
        static CLASS: LazyLock<UnsafeSync<WNDCLASSEXW>> = LazyLock::new(|| {
            let mut projector_class = WNDCLASSEXW::default();
            projector_class.cbSize = std::mem::size_of_val(&projector_class) as u32;
            projector_class.lpfnWndProc = Some(wnd_proc::<MinecraftInstanceListenerWindow>);
            projector_class.lpszClassName = PCWSTR(widestring::u16cstr!("listener").as_ptr());
            projector_class.hInstance =
                unsafe { GetModuleHandleW(PCWSTR::default()).unwrap().into() };

            unsafe {
                RegisterClassExW(&raw const projector_class);
            }

            unsafe { UnsafeSync::new(projector_class) }
        });

        CLASS.get()
    }

    fn on_message(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            let shell_hook_msg =
                RegisterWindowMessageW(PCWSTR(widestring::u16cstr!("SHELLHOOK").as_ptr()));

            log::trace!(
                "Received message: {:?} {:?}",
                (msg, wparam),
                (shell_hook_msg, WPARAM(HSHELL_WINDOWCREATED as usize))
            );

            if msg == shell_hook_msg && wparam == WPARAM(HSHELL_WINDOWCREATED as usize) {
                log::trace!("New window created: {:?}", hwnd);
                let hwnd: HWND = HWND(lparam.0 as *mut _);

                if is_minecraft_window(hwnd) {
                    self.run_cb(hwnd);
                }
            }

            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
}

pub struct MinecraftInstanceListener {
    _state: PhantomData<MinecraftInstanceListenerWindow>,
}

impl MinecraftInstanceListener {
    pub fn spawn(cb: Box<dyn FnMut(HWND)>) -> Self {
        let mut state = Box::new(MinecraftInstanceListenerWindow { cb });

        unsafe {
            EnumWindows(Some(Self::cb), LPARAM(state.as_mut() as *mut _ as isize)).unwrap();
        }

        let hwnd = wnd_class::spawn(
            WINDOW_EX_STYLE::default(),
            WINDOW_STYLE::default(),
            PCWSTR::default(),
            Some(HWND_MESSAGE),
            None,
            state,
        )
        .unwrap();

        unsafe {
            RegisterShellHookWindow(hwnd).unwrap();
        }

        Self {
            _state: PhantomData,
        }
    }

    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            let state = &mut *(lparam.0 as *mut MinecraftInstanceListenerWindow);

            if is_minecraft_window(hwnd) {
                state.run_cb(hwnd);
            }

            BOOL::from(true)
        }
    }
}
