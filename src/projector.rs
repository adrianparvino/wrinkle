use arc_swap::{ArcSwap, ArcSwapOption};
use std::sync::{Arc, LazyLock};
use windows::Win32::Foundation::{COLORREF, FALSE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CLIP_DEFAULT_PRECIS, CreateFontW, CreateSolidBrush, DEFAULT_CHARSET,
    DEFAULT_QUALITY, DT_CENTER, DT_NOCLIP, DT_SINGLELINE, DT_VCENTER, DeleteObject, DrawTextA,
    EndPaint, FIXED_PITCH, FW_SEMIBOLD, FillRect, GetDC, HBRUSH, InvalidateRect, MM_TEXT,
    OUT_DEFAULT_PRECIS, PAINTSTRUCT, ReleaseDC, SRCCOPY, SelectObject, SetBkMode, SetMapMode,
    SetTextColor, StretchBlt, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, GWL_HWNDPARENT, GetWindowRect, RegisterClassExW, SW_HIDE, SW_SHOW,
    SWP_FRAMECHANGED, SWP_SHOWWINDOW, SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_PAINT,
    WM_SHOWWINDOW, WNDCLASSEXW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::PCWSTR;

use crate::config::{Config, Hotkey};
use crate::instance::MinecraftInstance;
use crate::utils::UnsafeSync;
use crate::wnd_class::{self, WndClass, wnd_proc};
pub struct ProjectorWindow {
    instance: Arc<ArcSwapOption<MinecraftInstance>>,
    hotkey: Arc<ArcSwap<Option<Hotkey>>>,
    ruler: Ruler,
    width: i32,
    height: i32,
}

#[derive(Clone, Debug)]
pub struct Projector {
    hwnd: HWND,
    hotkey: Arc<ArcSwap<Option<Hotkey>>>,
}

unsafe impl Send for Projector {}
unsafe impl Sync for Projector {}

#[derive(Clone, Copy, Debug)]
pub struct Ruler {
    hwnd: HWND,
}

pub struct RulerWindow {
    config: Arc<ArcSwap<Config>>,
    white: HBRUSH,
}

impl WndClass for RulerWindow {
    fn get_class() -> &'static WNDCLASSEXW {
        static CLASS: LazyLock<UnsafeSync<WNDCLASSEXW>> = LazyLock::new(|| {
            let mut projector_class = WNDCLASSEXW::default();
            projector_class.cbSize = std::mem::size_of_val(&projector_class) as u32;
            projector_class.lpfnWndProc = Some(wnd_proc::<RulerWindow>);
            projector_class.lpszClassName = PCWSTR(widestring::u16cstr!("ruler").as_ptr());
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
            match (msg, wparam) {
                (WM_PAINT, _) => {
                    let mut ps = PAINTSTRUCT::default();
                    let ruler_hdc = BeginPaint(hwnd, &raw mut ps);

                    let rect_width = 60;

                    let RECT {
                        left,
                        top,
                        right,
                        bottom,
                    } = {
                        let mut rect = RECT::default();
                        GetWindowRect(hwnd, &raw mut rect).unwrap();
                        rect
                    };

                    let width = right - left;
                    let height = bottom - top;

                    FillRect(
                        ruler_hdc,
                        &RECT {
                            left: width / 2 - 1,
                            top: 0,
                            bottom: height,
                            right: width / 2,
                        } as *const RECT,
                        self.white,
                    );

                    let config = *self.config.load_full();

                    let brushes = config.colors.map(|color| CreateSolidBrush(color.into()));
                    for i in -config.ruler..config.ruler {
                        let rect = &mut RECT {
                            left: width / 2 + (i) * width / rect_width,
                            top: height / 2 - 40,
                            bottom: height / 2 + 40,
                            right: width / 2 + (i + 1) * width / rect_width,
                        } as *mut RECT;

                        FillRect(ruler_hdc, rect, brushes[(i % 2).abs() as usize]);
                    }
                    for brush in brushes {
                        DeleteObject(brush.into()).unwrap();
                    }

                    SetMapMode(ruler_hdc, MM_TEXT);
                    let font = CreateFontW(
                        48,
                        0,
                        0,
                        0,
                        FW_SEMIBOLD.0 as i32,
                        FALSE.0 as u32,
                        FALSE.0 as u32,
                        FALSE.0 as u32,
                        DEFAULT_CHARSET,
                        OUT_DEFAULT_PRECIS,
                        CLIP_DEFAULT_PRECIS,
                        DEFAULT_QUALITY,
                        FIXED_PITCH.0 as u32,
                        PCWSTR::default(),
                    );
                    SelectObject(ruler_hdc, font.into());
                    SetTextColor(ruler_hdc, COLORREF(0x00FFFFFF));
                    SetBkMode(ruler_hdc, TRANSPARENT);

                    let mut s = *b"0123456789";
                    for i in -config.ruler..config.ruler {
                        let s = {
                            let j = if i >= 0 { i + 1 } else { -i } as usize;
                            let j = j % 10;

                            &mut s[j..j + 1]
                        };

                        let rect = &mut RECT {
                            left: width / 2 + i * width / rect_width,
                            top: height / 2 - 40,
                            bottom: height / 2 + 40,
                            right: width / 2 + (i + 1) * width / rect_width,
                        } as *mut RECT;

                        DrawTextA(
                            ruler_hdc,
                            s,
                            rect,
                            DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOCLIP,
                        );
                    }
                    EndPaint(hwnd, &raw const ps).unwrap();

                    LRESULT(0)
                }

                _ => DefWindowProcW(hwnd, msg, wparam, lparam),
            }
        }
    }
}

impl Ruler {
    pub fn set_window_pos(&self, bg: HWND, left: i32, top: i32, width: i32, height: i32) {
        unsafe {
            SetWindowPos(
                self.hwnd,
                Some(bg),
                left,
                top,
                width,
                height,
                SWP_FRAMECHANGED,
            )
            .unwrap()
        };
    }

    pub fn set_owner(&self, hwnd: HWND) {
        unsafe {
            SetWindowLongPtrW(self.hwnd, GWL_HWNDPARENT, hwnd.0 as isize);
        }
    }

    pub fn spawn(config: Arc<ArcSwap<Config>>) -> Self {
        let hwnd = wnd_class::spawn(
            WS_EX_TOPMOST,
            WS_POPUP,
            PCWSTR(widestring::u16cstr!("Ruler").as_ptr()),
            None,
            None,
            unsafe {
                Box::new(RulerWindow {
                    config,
                    white: CreateSolidBrush(COLORREF(0x00FFFFFF)),
                })
            },
        )
        .unwrap();

        Self { hwnd }
    }
}

impl WndClass for ProjectorWindow {
    fn get_class() -> &'static WNDCLASSEXW {
        static CLASS: LazyLock<UnsafeSync<WNDCLASSEXW>> = LazyLock::new(|| {
            let mut projector_class = WNDCLASSEXW::default();
            projector_class.cbSize = std::mem::size_of_val(&projector_class) as u32;
            projector_class.lpfnWndProc = Some(wnd_proc::<ProjectorWindow>);
            projector_class.lpszClassName = PCWSTR(widestring::u16cstr!("projector").as_ptr());
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
            match (msg, wparam) {
                (WM_SHOWWINDOW, _) => {
                    let Some(instance) = self.instance.load_full() else {
                        return DefWindowProcW(hwnd, msg, wparam, lparam);
                    };

                    let hotkey = *self.hotkey.load_full();

                    let rect = instance.get_window_rect();
                    let lpmi = instance.get_monitor_info();
                    let projector_width = rect.left - lpmi.rcMonitor.left;
                    let projector_height = if hotkey == Some(Hotkey::Tall) {
                        800
                    } else {
                        1400
                    };
                    let cy = lpmi.rcMonitor.top.midpoint(lpmi.rcMonitor.bottom);

                    self.width = projector_width;
                    self.height = projector_height;
                    SetWindowPos(
                        hwnd,
                        None,
                        0,
                        cy - projector_height / 2,
                        projector_width,
                        projector_height,
                        SWP_FRAMECHANGED,
                    )
                    .unwrap();
                    self.ruler.set_window_pos(
                        hwnd,
                        0,
                        cy - projector_height / 2,
                        projector_width,
                        projector_height,
                    );

                    LRESULT(1)
                }
                (WM_PAINT, _) => {
                    let Some(instance) = self.instance.load_full() else {
                        return DefWindowProcW(hwnd, msg, wparam, lparam);
                    };

                    let hotkey = *self.hotkey.load_full();

                    let rect = instance.get_window_rect();
                    let width = rect.right - rect.left;
                    let height = rect.bottom - rect.top;

                    let mut ps = PAINTSTRUCT::default();
                    let source_hdc = GetDC(Some(instance.hwnd));
                    let projector_hdc = BeginPaint(hwnd, &raw mut ps);

                    match hotkey {
                        Some(Hotkey::Tall) => {
                            let _ = ShowWindow(self.ruler.hwnd, SW_SHOW);
                        }
                        _ => {
                            let _ = ShowWindow(self.ruler.hwnd, SW_HIDE);
                        }
                    }

                    match hotkey {
                        Some(Hotkey::Tall) => {
                            let rect_width = 60;
                            let rect_height = 500;

                            StretchBlt(
                                projector_hdc,
                                0,
                                0,
                                self.width,
                                self.height,
                                Some(source_hdc),
                                width / 2 - rect_width / 2,
                                height / 2 - rect_height / 2,
                                rect_width,
                                rect_height,
                                SRCCOPY,
                            )
                            .unwrap();
                        }
                        Some(Hotkey::Thin) => {
                            let e_height = self.width / 11;

                            let pie_height = self.height - e_height;
                            let pie_width = self.width;

                            StretchBlt(
                                projector_hdc,
                                0,
                                0,
                                self.width,
                                e_height,
                                Some(source_hdc),
                                0,
                                37,
                                99,
                                9,
                                SRCCOPY,
                            )
                            .unwrap();

                            StretchBlt(
                                projector_hdc,
                                0,
                                e_height,
                                pie_width,
                                pie_height,
                                Some(source_hdc),
                                width - 340,
                                height - 420,
                                340,
                                420,
                                SRCCOPY,
                            )
                            .unwrap();
                        }
                        _ => {}
                    }

                    ReleaseDC(Some(instance.hwnd), source_hdc);
                    EndPaint(hwnd, &raw const ps).unwrap();

                    InvalidateRect(Some(hwnd), None, true).unwrap();

                    LRESULT(0)
                }

                _ => DefWindowProcW(hwnd, msg, wparam, lparam),
            }
        }
    }
}

impl Projector {
    pub fn hotkey_hook(&self, hotkey: Option<Hotkey>) {
        unsafe {
            self.hotkey.store(Arc::new(hotkey));
            let _ = ShowWindow(self.hwnd, if hotkey.is_some() { SW_SHOW } else { SW_HIDE });
        }
    }

    pub fn spawn(
        instance: Arc<ArcSwapOption<MinecraftInstance>>,
        config: Arc<ArcSwap<Config>>,
    ) -> Self {
        let ruler = Ruler::spawn(config.clone());

        let hotkey = Arc::new(ArcSwap::from_pointee(None));

        let projector_wnd = wnd_class::spawn(
            WS_EX_TOPMOST,
            WS_POPUP,
            PCWSTR(widestring::u16str!("Projector").as_ptr()),
            None,
            None,
            Box::new(ProjectorWindow {
                instance,
                width: 0,
                height: 0,
                ruler,
                hotkey: hotkey.clone(),
            }),
        )
        .unwrap();

        ruler.set_owner(projector_wnd);

        Self {
            hwnd: projector_wnd,
            hotkey,
        }
    }
}
