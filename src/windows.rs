//! Platform Dependent: Windows

use winapi::*; use kernel32::*; use user32::*;
use std::mem::{size_of, uninitialized, zeroed, transmute};
use std::ptr::{null, null_mut};
use std::ffi::CString;
#[cfg(feature = "with_ferrite")] use ferrite as fe;

pub struct NativeWindow(HWND);
impl NativeWindow
{
    pub fn new(initial_size: (u16, u16), caption: &str) -> Self
    {
        let capz = CString::new(caption).unwrap();
        let w = unsafe
        {
            CreateWindowExA(WS_EX_NOREDIRECTIONBITMAP, transmute(WindowServer::instance().wc as usize), capz.as_ptr(), WS_OVERLAPPEDWINDOW,
                CW_USEDEFAULT, CW_USEDEFAULT, initial_size.0 as _, initial_size.1 as _, null_mut(), null_mut(), GetModuleHandleA(null()), null_mut())
        };
        if w.is_null() { panic!("Failed to create window"); }
        NativeWindow(w)
    }
    pub fn show(&self) { unsafe { ShowWindow(self.0, SW_SHOWNORMAL) }; }
    pub fn client_size(&self) -> (usize, usize)
    {
        let mut r = unsafe { uninitialized() };
        unsafe { GetClientRect(self.0, &mut r) };
        ((r.right - r.left) as _, (r.bottom - r.top) as _)
    }
}

pub struct WindowServer { wc: ATOM }
impl WindowServer
{
    AppInstance!(pub static instance: WindowServer = WindowServer::new());
    const WNDCLASS_NAME: &'static str = "ws_common::CommonWindow\x00";

    fn new() -> Self
    {
        unsafe
        {
            let wc = RegisterClassExW(&WNDCLASSEXW
            {
                cbSize: size_of::<WNDCLASSEXW>() as _, hInstance: GetModuleHandleA(null()),
                lpfnWndProc: Some(Self::messages), lpszClassName: Self::WNDCLASS_NAME.as_ptr() as _, hCursor: LoadCursorA(null_mut(), IDC_ARROW as _),
                style: CS_OWNDC, .. zeroed()
            });
            if wc == 0 { panic!("Failed to register window class"); }
            WindowServer { wc }
        }
    }

    pub fn process_events(&self)
    {
        let mut msg = unsafe { uninitialized() };
        while unsafe { GetMessageW(&mut msg, null_mut(), 0, 0) > 0 }
        {
            unsafe { TranslateMessage(&mut msg); DispatchMessageW(&mut msg); }
        }
    }

    extern "system" fn messages(hwnd: HWND, msg: UINT, wp: WPARAM, lp: LPARAM) -> LRESULT
    {
        match msg
        {
            WM_DESTROY => unsafe { PostQuitMessage(0); 0 },
            _ => unsafe { DefWindowProcA(hwnd, msg, wp, lp) }
        }
    }
}

/// Ferrite integration
#[cfg(feature = "with_ferrite")]
impl WindowServer
{
    pub fn presentation_support(&self, adapter: &fe::PhysicalDevice, rendered_qf: u32) -> bool
    {
        adapter.win32_presentation_support(rendered_qf)
    }
    pub fn new_render_surface(&self, native: &NativeWindow, apicontext: &fe::Instance) -> fe::Result<fe::Surface>
    {
        fe::Surface::new_win32(apicontext, unsafe { GetModuleHandleA(null()) }, native.0)
    }
}
