//! Platform Dependent: xcb

use wxcb as x11;
use wxcb::Drawable;
#[cfg(feature = "with_ferrite")] use ferrite as fe;

pub struct Atoms
{
    wm_protocols: x11::Atom, wm_delete_window: x11::Atom
}
pub struct WindowServer
{
    connection: x11::Connection, screen: x11::Screen, visual: x11::xcb_visualid_t, cmap: x11::Colormap, atom: Atoms
}
impl WindowServer
{
    AppInstance!(pub static instance: WindowServer = WindowServer::new());

    fn new() -> Self
    {
        let connection = x11::Connection::open(None, None).expect("Failed to open connection with x11");
        let screen = connection.iterate_setup_roots().next().expect("Could not find root screen");
        let depth = screen.iterate_allowed_depths().find(|x| x.depth() == 32).expect("Could not find 32bpp in supported depth");
        let visual = depth.iterate_visuals().find(|x| x.class() == x11::XCB_VISUAL_CLASS_TRUE_COLOR as u8)
            .expect("Could not find visuals which has 32bpp depth and is TrueColor");
        let cmap = x11::Colormap::new(&connection, visual.id(), screen.root(), x11::XCB_COLORMAP_ALLOC_NONE as _);
        let atom = 
        {
            let wm_protocols_ck = connection.intern_atom("WM_PROTOCOLS", false);
            let wm_delete_window_ck = connection.intern_atom("WM_DELETE_WINDOW", false);
            Atoms
            {
                wm_protocols: wm_protocols_ck.wait_reply(), wm_delete_window: wm_delete_window_ck.wait_reply()
            }
        };

        WindowServer { screen, visual: visual.id(), cmap, atom, connection }
    }
    pub fn process_events(&self)
    {
        while let Some(e) = self.connection.wait_event()
        {
            match e.response_type()
            {
                x11::XCB_CLIENT_MESSAGE =>
                {
                    let e: &x11::ClientMessageEvent = e.as_ref();
                    if e.msg_type() == self.atom.wm_protocols && e.data_as_u32() == self.atom.wm_delete_window { break; }
                },
                _ => {}
            }
        }
    }

    // Ferrite Integration //
    #[cfg(feature = "with_ferrite")]
    pub fn presentation_support(&self, adapter: &fe::PhysicalDevice, rendered_qf: u32) -> bool
    {
        adapter.xcb_presentation_support(rendered_qf, self.connection.inner(), self.visual)
    }
    #[cfg(feature = "with_ferrite")]
    pub fn new_render_surface(&self, native: &NativeWindow, apicontext: &fe::Instance) -> fe::Result<fe::Surface>
    {
        fe::Surface::new_xcb(apicontext, self.connection.inner(), native.0.id())
    }
}

pub struct NativeWindow(x11::Window<'static>);
impl NativeWindow
{
    pub fn new(initial_size: (u16, u16), caption: &str) -> Self
    {
        let w = x11::WindowBuilder::new(&WindowServer::instance().screen).size(initial_size)
            .visual(32, WindowServer::instance().visual).back_pixel(0).border_pixel(0)
            .colormap(&WindowServer::instance().cmap).create(&WindowServer::instance().connection);
        w.replace_property(x11::XCB_ATOM_WM_NAME, caption);
    	WindowServer::instance().connection.flush();
        NativeWindow(w)
    }
    pub fn show(&self) { self.0.map(); }
    pub fn client_size(&self) -> (usize, usize)
    {
        let g = self.0.geometry().wait_reply();
        (g.size.0 as _, g.size.1 as _)
    }
}
