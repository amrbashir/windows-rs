use windows::Win32::Foundation::*;
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::BOOL;

const SUBCLASS_ID: usize = 0x4b535743;

pub(super) struct ChildWindow {
    hwnd: HWND,
    parent: HWND,
}

impl ChildWindow {
    pub(super) fn new(hwnd: *mut core::ffi::c_void, parent: usize) -> windows::core::Result<Self> {
        let this = Self {
            hwnd: HWND(hwnd),
            parent: HWND(parent as _),
        };
        unsafe {
            SetWindowSubclass(this.hwnd, Some(subclass), SUBCLASS_ID, 0).ok()?;
            let style = WINDOW_STYLE(GetWindowLongPtrW(this.hwnd, GWL_STYLE) as u32);
            SetWindowLongPtrW(
                this.hwnd,
                GWL_STYLE,
                ((style
                    & !(WS_POPUP
                        | WS_CAPTION
                        | WS_THICKFRAME
                        | WS_SYSMENU
                        | WS_MINIMIZEBOX
                        | WS_MAXIMIZEBOX))
                    | WS_CHILD
                    | WS_CLIPSIBLINGS)
                    .0 as _,
            );
            let ex_style = WINDOW_EX_STYLE(GetWindowLongPtrW(this.hwnd, GWL_EXSTYLE) as u32);
            SetWindowLongPtrW(
                this.hwnd,
                GWL_EXSTYLE,
                ((ex_style & !WS_EX_APPWINDOW)
                    | WS_EX_NOACTIVATE
                    | WS_EX_NOREDIRECTIONBITMAP
                    | WS_EX_TOOLWINDOW)
                    .0 as _,
            );
            SetParent(this.hwnd, Some(this.parent))?;
        }
        this.resize()?;
        Ok(this)
    }

    pub(super) fn resize(&self) -> windows::core::Result<()> {
        unsafe {
            let _ = EnumChildWindows(Some(self.hwnd), Some(configure_input), LPARAM(0));
            let mut rect = RECT::default();
            GetClientRect(self.parent, &mut rect)?;
            SetWindowPos(
                self.hwnd,
                Some(HWND_TOP),
                0,
                0,
                (rect.right - rect.left).max(1),
                (rect.bottom - rect.top).max(1),
                SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
            )
        }
    }
}

impl Drop for ChildWindow {
    fn drop(&mut self) {
        unsafe {
            let _ = RemoveWindowSubclass(self.hwnd, Some(subclass), SUBCLASS_ID);
        }
    }
}

unsafe extern "system" fn subclass(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    match msg {
        WM_NCCALCSIZE => LRESULT(0),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as _),
        WM_POINTERACTIVATE => LRESULT(PA_NOACTIVATE as _),
        WM_NCDESTROY => {
            let _ = unsafe { RemoveWindowSubclass(hwnd, Some(subclass), SUBCLASS_ID) };
            unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
        }
        _ => unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) },
    }
}

unsafe extern "system" fn configure_input(hwnd: HWND, _: LPARAM) -> BOOL {
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_NOACTIVATE.0 as isize);
        let _ = SetWindowSubclass(hwnd, Some(input_subclass), SUBCLASS_ID + 1, 0);
    }
    true.into()
}

unsafe extern "system" fn input_subclass(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    match msg {
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as _),
        WM_POINTERACTIVATE => LRESULT(PA_NOACTIVATE as _),
        WM_NCDESTROY => unsafe {
            let _ = RemoveWindowSubclass(hwnd, Some(input_subclass), SUBCLASS_ID + 1);
            DefSubclassProc(hwnd, msg, wparam, lparam)
        },
        _ => unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::core::w;

    struct OwnedWindow(HWND);
    impl Drop for OwnedWindow {
        fn drop(&mut self) {
            unsafe {
                let _ = DestroyWindow(self.0);
            }
        }
    }

    fn hidden_window(width: i32, height: i32) -> OwnedWindow {
        OwnedWindow(unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                None,
                WS_POPUP,
                0,
                0,
                width,
                height,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        })
    }

    #[test]
    fn embedding_preserves_parent_geometry_and_rejects_activation() {
        let parent = hidden_window(100, 40);
        let window = hidden_window(640, 480);
        let _child = ChildWindow::new(window.0.0, parent.0.0 as usize).unwrap();
        unsafe {
            assert_eq!(GetParent(window.0).unwrap(), parent.0);
            let style = WINDOW_STYLE(GetWindowLongPtrW(window.0, GWL_STYLE) as u32);
            assert!(style.contains(WS_CHILD));
            assert_eq!(style.0 & (WS_CAPTION | WS_THICKFRAME).0, 0);
            let mut rect = RECT::default();
            GetClientRect(parent.0, &mut rect).unwrap();
            assert_eq!((rect.right, rect.bottom), (100, 40));
            GetClientRect(window.0, &mut rect).unwrap();
            assert_eq!((rect.right, rect.bottom), (100, 40));
            assert_eq!(
                SendMessageW(window.0, WM_MOUSEACTIVATE, None, None).0,
                MA_NOACTIVATE as isize
            );
            assert_eq!(
                SendMessageW(window.0, WM_POINTERACTIVATE, None, None).0,
                PA_NOACTIVATE as isize
            );
        }
    }
}
