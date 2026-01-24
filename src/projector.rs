use arc_swap::{ArcSwap, ArcSwapOption};
use std::sync::{Arc, LazyLock};
use windows::Win32::Foundation::{COLORREF, FALSE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CLIP_DEFAULT_PRECIS, CreateFontW, CreateSolidBrush, DEFAULT_CHARSET,
    DEFAULT_QUALITY, DT_CENTER, DT_NOCLIP, DT_SINGLELINE, DT_VCENTER, DrawTextA, EndPaint,
    FIXED_PITCH, FW_SEMIBOLD, FillRect, GetDC, HBRUSH, MM_TEXT, OUT_DEFAULT_PRECIS, PAINTSTRUCT,
    RDW_INVALIDATE, RedrawWindow, ReleaseDC, SRCCOPY, SelectObject, SetBkMode, SetMapMode,
    SetTextColor, StretchBlt, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, GWL_HWNDPARENT, GWLP_USERDATA, GetWindowLongPtrW, GetWindowRect,
    RegisterClassExW, SW_HIDE, SW_SHOW, SWP_FRAMECHANGED, SWP_SHOWWINDOW, SetTimer,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_PAINT, WM_SHOWWINDOW, WM_TIMER, WNDCLASSEXW,
    WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::PCWSTR;

use crate::config::Config;
use crate::instance::MinecraftInstance;
use crate::utils::UnsafeSync;
use crate::wnd_class::{self, WndClass, wnd_proc};
pub struct ProjectorWindow {
    instance: Arc<ArcSwapOption<MinecraftInstance>>,
    ruler: Ruler,
    width: i32,
    height: i32,
}

#[derive(Clone, Debug)]
pub struct Projector {
    hwnd: HWND,
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
    purple: HBRUSH,
    lavender: HBRUSH,
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
                    let Some(state) =
                        (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut RulerWindow).as_mut()
                    else {
                        return LRESULT(1);
                    };

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
                        state.white,
                    );

                    let config = *state.config.load_full();

                    for i in -config.ruler..config.ruler {
                        let rect = &mut RECT {
                            left: width / 2 + (i) * width / rect_width,
                            top: height / 2 - 40,
                            bottom: height / 2 + 40,
                            right: width / 2 + (i + 1) * width / rect_width,
                        } as *mut RECT;

                        FillRect(
                            ruler_hdc,
                            rect,
                            if i % 2 == 0 {
                                state.purple
                            } else {
                                state.lavender
                            },
                        );
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
                    purple: CreateSolidBrush(COLORREF(0x00C000C0)),
                    lavender: CreateSolidBrush(COLORREF(0x00C080C0)),
                })
            },
        )
        .unwrap();

        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }

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
                (WM_TIMER, WPARAM(1)) => {
                    RedrawWindow(Some(hwnd), None, None, RDW_INVALIDATE).unwrap();

                    LRESULT(0)
                }
                (WM_SHOWWINDOW, _) => {
                    let Some(instance) = self.instance.load_full() else {
                        return DefWindowProcW(hwnd, msg, wparam, lparam);
                    };

                    let rect = instance.get_window_rect();
                    let width = rect.right - rect.left;

                    let lpmi = instance.get_monitor_info();
                    let projector_width = (lpmi.rcMonitor.right - lpmi.rcMonitor.left - width) / 2;
                    let projector_height = 800;
                    let cy = lpmi.rcMonitor.top.midpoint(lpmi.rcMonitor.bottom);

                    SetWindowPos(
                        hwnd,
                        None,
                        0,
                        cy - projector_height / 2,
                        projector_width,
                        projector_height,
                        SWP_FRAMECHANGED | SWP_SHOWWINDOW,
                    )
                    .unwrap();
                    self.ruler.set_window_pos(
                        hwnd,
                        0,
                        cy - projector_height / 2,
                        projector_width,
                        projector_height,
                    );

                    self.width = projector_width;
                    self.height = projector_height;

                    LRESULT(1)
                }
                (WM_PAINT, _) => {
                    let Some(instance) = self.instance.load_full() else {
                        return DefWindowProcW(hwnd, msg, wparam, lparam);
                    };

                    let rect = instance.get_window_rect();
                    let width = rect.right - rect.left;
                    let height = rect.bottom - rect.top;

                    let mut ps = PAINTSTRUCT::default();
                    let source_hdc = GetDC(Some(instance.hwnd));
                    let projector_hdc = BeginPaint(hwnd, &raw mut ps);

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
                    ReleaseDC(Some(instance.hwnd), source_hdc);
                    EndPaint(hwnd, &raw const ps).unwrap();

                    LRESULT(0)
                }

                _ => DefWindowProcW(hwnd, msg, wparam, lparam),
            }
        }
    }
}

impl Projector {
    pub fn set_visibility(&self, visibility: bool) {
        unsafe {
            let _ = ShowWindow(self.hwnd, if visibility { SW_SHOW } else { SW_HIDE });
        }
    }

    pub fn spawn(
        instance: Arc<ArcSwapOption<MinecraftInstance>>,
        config: Arc<ArcSwap<Config>>,
    ) -> Self {
        let ruler = Ruler::spawn(config.clone());

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
            }),
        )
        .unwrap();

        ruler.set_owner(projector_wnd);

        unsafe {
            SetTimer(Some(projector_wnd), 1, 10, None);
        }

        Self {
            hwnd: projector_wnd,
        }
    }
}
