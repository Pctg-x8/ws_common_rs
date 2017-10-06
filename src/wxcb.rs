// xcb wrapper

pub use xcb::ffi::*;
use std::ffi::CString;
use std::ptr::{null, null_mut};
use std::mem::zeroed;
use std::ops::{Deref, DerefMut};
use libc::free;
use std::fmt::{Debug, Result as FmtResult, Formatter};

/// owned malloc-ed pointer box
pub struct MallocBox<T: ?Sized>(pub *mut T);
impl<T: ?Sized> Deref for MallocBox<T>
{
    type Target = T; fn deref(&self) -> &T { unsafe { &*self.0 } }
}
impl<T: ?Sized> DerefMut for MallocBox<T>
{
    fn deref_mut(&mut self) -> &mut T { unsafe { &mut *self.0 } }
}
impl<T: ?Sized> Drop for MallocBox<T>
{
    fn drop(&mut self) { unsafe { free(self.0 as *mut _); } }
}
impl<T: ?Sized> Debug for MallocBox<T> where T: Debug
{
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult
    {
        Debug::fmt(unsafe { &*self.0 }, fmt)
    }
}

pub struct Connection(*mut xcb_connection_t);
impl Drop for Connection
{
	fn drop(&mut self) { unsafe { xcb_disconnect(self.0) }; }
}
impl Connection
{
	pub fn inner(&self) -> *mut xcb_connection_t { self.0 }
	pub fn open(display: Option<&str>, screen: Option<u32>) -> Result<Self, i32>
	{
		let display_name = display.map(|x| CString::new(x).unwrap());
		let display_ptr = display_name.as_ref().map(|x| x.as_ptr()).unwrap_or(null()) as *mut _;
		let screen_ptr = screen.as_ref().map(|x| x as *const _).unwrap_or(null()) as *mut _;
		let p = unsafe { xcb_connect(display_ptr, screen_ptr) };
		match unsafe { xcb_connection_has_error(p) }
		{
			0 => Ok(Connection(p)),
			e => Err(e)
		}
	}
	pub fn iterate_setup_roots(&self) -> ScreenIterator { ScreenIterator(Some(unsafe { xcb_setup_roots_iterator(xcb_get_setup(self.0)) })) }

	pub fn generate_id(&self) -> u32 { unsafe { xcb_generate_id(self.0) } }
	pub fn intern_atom(&self, name: &str, only_if_exists: bool) -> AtomCookie
	{
		AtomCookie(unsafe { xcb_intern_atom(self.0, only_if_exists as _, name.len() as _, name.as_ptr() as *const _) }, self)
	}

	pub fn wait_event(&self) -> Option<GenericEvent>
	{
		let p = unsafe { xcb_wait_for_event(self.0) };
		if p.is_null() { None } else { Some(GenericEvent(MallocBox(p))) }
	}
	pub fn flush(&self) { unsafe { xcb_flush(self.0) }; }
}

pub struct WindowBuilder
{
	depth: u8, parent: xcb_window_t, pos: (i16, i16), sz: (u16, u16), border: u16, class: u16, visual: xcb_visualid_t,
	attributes: u32, attribute_list: xcb_create_window_value_list_t
}
impl WindowBuilder
{
	pub fn new(base_screen: &Screen) -> Self
	{
		WindowBuilder
		{
			depth: base_screen.root_depth(), visual: base_screen.root_visual(), parent: base_screen.root(),
			pos: (0, 0), sz: (128, 128), border: 0, class: XCB_WINDOW_CLASS_INPUT_OUTPUT as _,
			attributes: 0, attribute_list: unsafe { zeroed() }
		}
	}
	pub fn size(&mut self, s: (u16, u16)) -> &mut Self { self.sz = s; self }
	pub fn visual(&mut self, depth: u8, vid: xcb_visualid_t) -> &mut Self
	{
		self.depth = depth; self.visual = vid; self
	}
	pub fn border_pixel(&mut self, p: u32) -> &mut Self
	{
		self.attributes |= XCB_CW_BORDER_PIXEL; self.attribute_list.border_pixel = p; self
	}
	pub fn back_pixel(&mut self, p: u32) -> &mut Self
	{
		self.attributes |= XCB_CW_BACK_PIXEL; self.attribute_list.background_pixel = p; self
	}
	pub fn colormap(&mut self, c: &Colormap) -> &mut Self
	{
		self.attributes |= XCB_CW_COLORMAP; self.attribute_list.colormap = c.id(); self
	}

	pub fn create<'c>(&self, connection: &'c Connection) -> Window<'c>
	{
		let mut ptr = null_mut();
		unsafe { xcb_create_window_value_list_serialize(&mut ptr, self.attributes, &self.attribute_list) };
		let id = connection.generate_id();
		unsafe { xcb_create_window(connection.0, self.depth, id, self.parent,
			self.pos.0, self.pos.1, self.sz.0, self.sz.1, self.border, self.class,
			self.visual, self.attributes, ptr as *const _) };
		unsafe { ::libc::free(ptr); }
		Window::Owned(id, connection)
	}
}

pub struct Screen(*mut xcb_screen_t);
#[allow(dead_code)]
impl Screen
{
	pub fn root(&self) -> xcb_window_t { unsafe { (*self.0).root } }
	pub fn root_visual(&self) -> xcb_visualid_t { unsafe { (*self.0).root_visual } }
	pub fn root_depth(&self) -> u8 { unsafe { (*self.0).root_depth } }
	pub fn default_colormap(&self) -> xcb_colormap_t { unsafe { (*self.0).default_colormap } }
	pub fn iterate_allowed_depths(&self) -> DepthIterator { DepthIterator(Some(unsafe { xcb_screen_allowed_depths_iterator(self.0) })) }
}

pub struct Depth(*mut xcb_depth_t);
impl Depth
{
	pub fn depth(&self) -> u8 { unsafe { (*self.0).depth } }
	pub fn iterate_visuals(&self) -> VisualTypeIterator { VisualTypeIterator(Some(unsafe { xcb_depth_visuals_iterator(self.0) })) }
}

pub struct VisualType(*mut xcb_visualtype_t);
#[allow(dead_code)]
impl VisualType
{
	pub fn id(&self) -> xcb_visualid_t { unsafe { (*self.0).visual_id } }
	pub fn class(&self) -> u8 { unsafe { (*self.0).class } }
	pub fn bits_per_rgb_value(&self) -> u8 { unsafe { (*self.0).bits_per_rgb_value } }
	pub fn colormap_entries(&self) -> u16 { unsafe { (*self.0).colormap_entries } }
	pub fn red_mask(&self) -> u32 { unsafe { (*self.0).red_mask } }
	pub fn green_mask(&self) -> u32 { unsafe { (*self.0).green_mask } }
	pub fn blue_mask(&self) -> u32 { unsafe { (*self.0).blue_mask } }
}

#[allow(dead_code)]
pub enum Window<'d> { Owned(xcb_window_t, &'d Connection), Borrowed(xcb_window_t, &'d Connection) }
impl<'d> Window<'d>
{
	pub fn id(&self) -> xcb_window_t { match self { &Window::Owned(s, _) | &Window::Borrowed(s, _) => s } }
	pub fn replace_property<PropertyDataT: Property + ?Sized>(&self, property: xcb_atom_t, data: &PropertyDataT)
	{
		match self
		{
			&Window::Owned(w, c) => data.change_property(c, w, property, XCB_PROP_MODE_REPLACE),
			_ => panic!("Changing Property of borrowed window")
		}
	}
	pub fn map(&self)
	{
		match self
		{
			&Window::Owned(w, c) => unsafe { xcb_map_window(c.0, w); },
			_ => panic!("Changing Property of borrowed window")
		}
	}
}
impl<'d> Drop for Window<'d>
{
	fn drop(&mut self)
	{
		if let &mut Window::Owned(w, c) = self
		{
			unsafe { xcb_destroy_window(c.0, w) };
		}
	}
}

pub struct Colormap(xcb_colormap_t);
impl Colormap
{
	pub fn new(con: &Connection, visual: xcb_visualid_t, window: xcb_window_t, alloc: u8) -> Self
	{
		let id = con.generate_id();
		unsafe { xcb_create_colormap(con.0, alloc, id, window, visual) };
		Colormap(id)
	}
	pub fn id(&self) -> xcb_colormap_t { self.0 }
}

pub struct GenericEvent(MallocBox<xcb_generic_event_t>);
impl GenericEvent
{
	pub fn response_type(&self) -> u8 { self.0.response_type & !0x80 }
}
pub struct ClientMessageEvent(MallocBox<xcb_client_message_event_t>);
impl AsRef<ClientMessageEvent> for GenericEvent { fn as_ref(&self) -> &ClientMessageEvent { unsafe { ::std::mem::transmute(self) } } }
#[allow(dead_code)]
impl ClientMessageEvent
{
	pub fn msg_type(&self) -> xcb_atom_t { self.0.type_ }
	pub fn data_as_u32(&self) -> u32
	{
		unsafe { *(self.0.data.data.as_ptr() as *const u32) }
	}
	pub fn data_as_u64(&self) -> u64
	{
		unsafe { *(self.0.data.data.as_ptr() as *const u64) }
	}
}
pub struct GenericError(MallocBox<xcb_generic_error_t>);
impl AsRef<GenericError> for GenericEvent { fn as_ref(&self) -> &GenericError { unsafe { ::std::mem::transmute(self) } } }
#[allow(dead_code)]
impl GenericError
{
	pub fn error_code(&self) -> u8 { self.0.error_code }
	pub fn major_code(&self) -> u8 { self.0.major_code }
	pub fn minor_code(&self) -> u16 { self.0.minor_code }
}

pub type Atom = xcb_atom_t;
pub struct AtomCookie<'d>(xcb_intern_atom_cookie_t, &'d Connection);
impl<'d> AtomCookie<'d>
{
	pub fn wait_reply(self) -> Atom
	{
		let mut err = null_mut();
		let p = unsafe { xcb_intern_atom_reply(self.1 .0, self.0, &mut err) };
		if p.is_null()
		{
			panic!("Error in waiting reply of intern_atom: {:?}", MallocBox(err));
		}
		else { MallocBox(p).atom }
	}
}
pub struct GetGeometryCookie<'d>(xcb_get_geometry_cookie_t, &'d Connection);
impl<'d> GetGeometryCookie<'d>
{
	pub fn wait_reply(self) -> Geometry
	{
		let mut err = null_mut();
		let p = unsafe { xcb_get_geometry_reply(self.1 .0, self.0, &mut err) };
		if p.is_null()
		{
			panic!("Error in waiting reply of get_geometry: {:?}", MallocBox(err));
		}
		else
		{
			let r = MallocBox(p);
			Geometry { pos: (r.x, r.y), size: (r.width, r.height) }
		}
	}
}

pub struct Geometry
{
	pub pos: (i16, i16), pub size: (u16, u16)
}

pub struct ScreenIterator<'a>(Option<xcb_screen_iterator_t<'a>>);
pub struct DepthIterator<'a>(Option<xcb_depth_iterator_t<'a>>);
pub struct VisualTypeIterator(Option<xcb_visualtype_iterator_t>);
impl<'a> Iterator for ScreenIterator<'a>
{
	type Item = Screen;
	fn next(&mut self) -> Option<Screen>
	{
		let r = self.0.as_ref().and_then(|x| unsafe { x.data.as_mut().map(|x| Screen(x as *mut _)) });
		let end_iterate = if let Some(old) = self.0.as_mut()
		{
			unsafe { xcb_screen_next(old) }; (*old).rem == 0
		}
		else { false };
		if end_iterate { self.0 = None; }
		r
	}
}
impl<'a> Iterator for DepthIterator<'a>
{
	type Item = Depth;
	fn next(&mut self) -> Option<Depth>
	{
		let r = self.0.as_ref().and_then(|x| unsafe { x.data.as_mut().map(|x| Depth(x as *mut _)) });
		let end_iterate = if let Some(old) = self.0.as_mut()
		{
			unsafe { xcb_depth_next(old) }; (*old).rem == 0
		}
		else { false };
		if end_iterate { self.0 = None; }
		r
	}
}
impl Iterator for VisualTypeIterator
{
	type Item = VisualType;
	fn next(&mut self) -> Option<VisualType>
	{
		let r = self.0.as_ref().and_then(|x| unsafe { x.data.as_mut().map(|x| VisualType(x as *mut _)) });
		let end_iterate = if let Some(old) = self.0.as_mut()
		{
			unsafe { xcb_visualtype_next(old) }; (*old).rem == 0
		}
		else { false };
		if end_iterate { self.0 = None; }
		r
	}
}

pub trait Property
{
	fn change_property(&self, connection: &Connection, window: xcb_window_t, property: xcb_atom_t, mode: u32);
}
impl Property for str
{
	fn change_property(&self, connection: &Connection, window: xcb_window_t, property: xcb_atom_t, mode: u32)
	{
		unsafe { xcb_change_property(connection.0, mode as _, window, property, XCB_ATOM_STRING, 8, self.len() as _, self.as_ptr() as *const _) };
	}
}
pub trait Drawable
{
	fn geometry(&self) -> GetGeometryCookie;
}
impl<'d> Drawable for Window<'d>
{
	fn geometry(&self) -> GetGeometryCookie
	{
		match self
		{
			&Window::Owned(w, c) | &Window::Borrowed(w, c) => GetGeometryCookie(unsafe { xcb_get_geometry(c.0, w) }, c)
		}
	}
}

// xproto additional ffi section //

#[allow(non_camel_case_types)]
type xcb_bool32_t = u32;
#[repr(C)] #[derive(Debug, Clone, PartialEq, Eq)] #[allow(non_camel_case_types)]
pub struct xcb_create_window_value_list_t
{
	pub background_pixmap: xcb_pixmap_t,
	pub background_pixel: u32,
	pub border_pixmap: xcb_pixmap_t,
	pub border_pixel: u32,
	pub bit_gravity: u32,
	pub win_gravity: u32,
	pub backing_store: u32,
	pub backing_planes: u32,
	pub backing_pixel: u32,
	pub override_redirect: xcb_bool32_t,
	pub save_under: xcb_bool32_t,
	pub event_mask: u32,
	pub do_not_propagate_mask: u32,
	pub colormap: xcb_colormap_t,
	pub cursor: xcb_cursor_t
}

use libc::*;
extern "C"
{
	fn xcb_create_window_value_list_serialize(buffer: *mut *mut c_void, value_maks: u32, aux: *const xcb_create_window_value_list_t) -> c_int;
}
