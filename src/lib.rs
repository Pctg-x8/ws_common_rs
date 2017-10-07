
extern crate libc;
#[cfg(feature = "target_x11")] extern crate xcb;
#[cfg(windows)] extern crate winapi;
#[cfg(windows)] extern crate kernel32;
#[cfg(windows)] extern crate user32;
#[cfg(feature = "with_ferrite")] extern crate ferrite;

#[macro_use] extern crate appinstance;

#[cfg(feature = "target_x11")] pub mod wxcb;
#[cfg(feature = "target_x11")] mod pf_xcb;
#[cfg(feature = "target_x11")] pub use pf_xcb::*;

#[cfg(windows)] mod windows;
#[cfg(windows)] pub use windows::*;
