//! Win32 ownership and message dispatch. The Box holding State outlives HWND;
//! GWLP_USERDATA borrows that allocation and never owns/frees it. RefCell avoids
//! aliasing mutable Rust references when synchronous Windows calls reenter.

use crate::{document::Document, render::Renderer};
use std::{cell::RefCell, panic::{catch_unwind, AssertUnwindSafe}};
use windows::{
    core::{w, Error, PCWSTR, Result},
    Win32::{
        Foundation::*,
        Graphics::Gdi::{BeginPaint, EndPaint, InvalidateRect, PAINTSTRUCT},
        System::{Com::*, LibraryLoader::GetModuleHandleW},
        UI::WindowsAndMessaging::*,
    },
};

struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        // SAFETY: constructed only after successful CoInitializeEx on this thread,
        // dropped on the same thread after renderer and window resources.
        unsafe { CoUninitialize(); }
    }
}

struct Window(HWND);
impl Drop for Window {
    fn drop(&mut self) {
        // SAFETY: only this UI thread destroys the window; IsWindow covers the
        // normal WM_CLOSE path where it has already been destroyed.
        unsafe { if IsWindow(Some(self.0)).as_bool() { let _ = DestroyWindow(self.0); } }
    }
}

struct WindowClass(HINSTANCE);
impl Drop for WindowClass {
    fn drop(&mut self) {
        // SAFETY: the registered class is unregistered after its sole HWND dies.
        unsafe { let _ = UnregisterClassW(w!("AruaMarkIWindow"), Some(self.0)); }
    }
}

struct State { document: Document, renderer: Renderer, error: Option<Error> }

pub fn show_error(message: &str) {
    let text: Vec<u16> = message.replace('\0', "�").encode_utf16().chain(Some(0)).collect();
    // SAFETY: buffer and caption are valid NUL-terminated UTF-16 throughout the
    // synchronous message box. There is no borrowed window or callback state.
    unsafe { MessageBoxW(None, PCWSTR(text.as_ptr()), w!("Aruã Mark I — erro"), MB_OK | MB_ICONERROR); }
}

pub fn run(document: Document) -> Result<()> {
    // SAFETY: initializes this UI thread once; Apartment balances even early returns.
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?; }
    let _apartment = Apartment;
    // State precedes Window in declaration order so the window guard drops first.
    let state = Box::new(RefCell::new(State { document, renderer: Renderer::new()?, error: None }));
    let title: Vec<u16> = state.borrow().document.title.encode_utf16().chain(Some(0)).collect();
    // SAFETY: class strings and title are NUL-terminated; callback uses the ABI
    // required by Win32. The stable Box pointer stays valid until Window drops.
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc), hInstance: instance,
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            lpszClassName: w!("AruaMarkIWindow"),
            style: CS_HREDRAW | CS_VREDRAW, ..Default::default()
        };
        if RegisterClassW(&class) == 0 { return Err(Error::from_thread()); }
        let _class = WindowClass(instance);
        let hwnd = CreateWindowExW(WINDOW_EX_STYLE::default(), w!("AruaMarkIWindow"),
            PCWSTR(title.as_ptr()), WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT, CW_USEDEFAULT, 960, 640,
            None, None, Some(instance), Some((&*state as *const RefCell<State>).cast()))?;
        let _window = Window(hwnd);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let mut message = MSG::default();
        loop {
            let result = GetMessageW(&mut message, None, 0, 0).0;
            if result == -1 { return Err(Error::from_thread()); }
            if result == 0 { break; }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if let Some(error) = state.borrow_mut().error.take() { return Err(error); }
        if message.wParam.0 != 0 { return Err(Error::new(E_FAIL, "Falha no processamento da janela.")); }
        Ok(())
    }
}

unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // SAFETY: Win32 invokes this with the registered callback ABI. Catching panic
    // prevents Rust unwinding through the system's foreign stack frames.
    match catch_unwind(AssertUnwindSafe(|| unsafe { handle_message(hwnd, message, wp, lp) })) {
        Ok(result) => result,
        Err(_) => {
            // SAFETY: hwnd is the callback window. PostMessage does not reenter;
            // failure to post falls back to terminating our message loop.
            unsafe {
                let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<State>;
                if let Some(cell) = pointer.as_ref() {
                    if let Ok(mut state) = cell.try_borrow_mut() { state.error = Some(Error::new(E_FAIL, "Falha interna na janela.")); }
                }
                if PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).is_err() { PostQuitMessage(1); }
            }
            LRESULT(0)
        }
    }
}

unsafe fn handle_message(hwnd: HWND, message: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // SAFETY: CREATESTRUCTW is supplied by CreateWindowExW only for WM_NCCREATE.
    // Its lpCreateParams is our stable Box; stored as a non-owning pointer.
    unsafe {
        if message == WM_NCCREATE {
            let create = &*(lp.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
            // Let Win32 initialize its non-client state, including caption text.
            return DefWindowProcW(hwnd, message, wp, lp);
        }
        if message == WM_NCDESTROY {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            return DefWindowProcW(hwnd, message, wp, lp);
        }
        if message == WM_DESTROY { PostQuitMessage(0); return LRESULT(0); }
        if message == WM_ERASEBKGND { return LRESULT(1); }
        let pointer = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<State>;
        // WM_NCDESTROY clears the slot; the run() owner outlives every callback.
        if let Some(cell) = pointer.as_ref() {
            if let Ok(mut state) = cell.try_borrow_mut() {
                if message == WM_PAINT {
                    let mut paint = PAINTSTRUCT::default();
                    BeginPaint(hwnd, &mut paint);
                    let mut rect = RECT::default();
                    let result = GetClientRect(hwnd, &mut rect).and_then(|_| {
                        let State { renderer, document, .. } = &mut *state;
                        renderer.paint(hwnd, (rect.right - rect.left).max(0) as u32,
                            (rect.bottom - rect.top).max(0) as u32, document)
                    });
                    // Always paired, including a failed Direct2D paint.
                    let _ = EndPaint(hwnd, &paint);
                    match result {
                        Ok(true) => { let _ = InvalidateRect(Some(hwnd), None, false); },
                        Ok(false) => {},
                        Err(error) => fail(&mut state, hwnd, error),
                    }
                    return LRESULT(0);
                }
                if message == WM_SIZE {
                    let width = (lp.0 as u32 & 0xffff) as u32;
                    let height = ((lp.0 as u32 >> 16) & 0xffff) as u32;
                    if let Err(error) = state.renderer.resize(width, height) { fail(&mut state, hwnd, error); }
                    let _ = InvalidateRect(Some(hwnd), None, false);
                    return LRESULT(0);
                }
            }
        }
        DefWindowProcW(hwnd, message, wp, lp)
    }
}

fn fail(state: &mut State, hwnd: HWND, error: Error) {
    if state.error.is_none() { state.error = Some(error); }
    // SAFETY: queues a close for our live HWND; no reentrant destruction while
    // the caller holds RefMut<State>. run() later returns the recorded error.
    unsafe { if PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).is_err() { PostQuitMessage(1); } }
}
