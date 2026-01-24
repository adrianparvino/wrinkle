use std::ffi::c_void;

use windows::{
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::WindowsAndMessaging::{
            CREATESTRUCTW, CreateWindowExW, DefWindowProcW, GWLP_USERDATA, GetWindowLongPtrW,
            HMENU, SetWindowLongPtrW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_NCCREATE, WM_NCDESTROY,
            WNDCLASSEXW,
        },
    },
    core::PCWSTR,
};

pub trait WndClass {
    fn get_class() -> &'static WNDCLASSEXW;
    fn on_message(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
}

pub fn spawn<T>(
    ex_style: WINDOW_EX_STYLE,
    style: WINDOW_STYLE,
    window_name: PCWSTR,
    parent: Option<HWND>,
    menu: Option<HMENU>,
    state: Box<T>,
) -> windows::core::Result<HWND>
where
    T: WndClass,
{
    let class = T::get_class();

    unsafe {
        CreateWindowExW(
            ex_style,
            class.lpszClassName,
            window_name,
            style,
            0,
            0,
            0,
            0,
            parent,
            menu,
            Some(class.hInstance),
            Some(Box::into_raw(state) as *const c_void),
        )
    }
}

pub unsafe extern "system" fn wnd_proc<T: WndClass>(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_NCCREATE => {
                let p_create = lparam.0 as *mut CREATESTRUCTW;
                SetWindowLongPtrW(
                    hwnd,
                    GWLP_USERDATA,
                    (*p_create).lpCreateParams as usize as isize,
                );

                LRESULT(1)
            }
            WM_NCDESTROY => {
                let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut T;

                if !state.is_null() {
                    drop(Box::from_raw(state));
                }

                LRESULT(0)
            }
            _ => {
                let Some(state) = (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut T).as_mut()
                else {
                    return DefWindowProcW(hwnd, msg, wparam, lparam);
                };

                state.on_message(hwnd, msg, wparam, lparam)
            }
        }
    }
}
