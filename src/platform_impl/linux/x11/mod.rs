use std::{
    mem,
    os::raw::c_void,
    sync::{Arc, Mutex},
};

use lazy_static::lazy_static;
use x11_dl::xlib::{BadValue, XImage, XYPixmap, Xlib, GC};

use crate::{PixelBufferCreationError, PixelBufferFormatType};

lazy_static! {
    static ref XLIB: Xlib = Xlib::open().unwrap();
}

pub struct XlibPixelBuffer {
    pub(super) data: Vec<u8>,
    width: u32,
    height: u32,
    image: *mut XImage,
    pixmap: u64,
    gc: GC,
    window: u64,
    display: *mut c_void,
}

impl XlibPixelBuffer {
    pub unsafe fn new(
        width: u32,
        height: u32,
        format: PixelBufferFormatType,
        handle: raw_window_handle::unix::XlibHandle,
    ) -> Result<Self, PixelBufferCreationError> {
        let mut data: Vec<u8> = std::iter::repeat(0)
            .take((width as usize).saturating_mul(height as usize))
            .collect();
        let default_visual = (XLIB.XDefaultVisual)(mem::transmute(handle.display), 0);
        let image = (XLIB.XCreateImage)(
            mem::transmute(handle.display),
            default_visual,
            32,
            XYPixmap,
            0,
            data.as_mut_ptr() as *mut _,
            width,
            height,
            32,
            width as i32,
        );
        dbg!(image);
        let pixmap = (XLIB.XCreatePixmap)(
            mem::transmute(handle.display),
            handle.window,
            width,
            height,
            32,
        );
        assert_ne!(pixmap, x11_dl::xlib::BadValue as u64);
        dbg!(pixmap);
        let gc = (XLIB.XDefaultGC)(mem::transmute(handle.display), 0);
        Ok(Self {
            data,
            width,
            height,
            image,
            pixmap,
            gc,
            window: handle.window,
            display: handle.display,
        })
    }

    pub unsafe fn blit(
        &self,
        src_x: u32,
        src_y: u32,
        dest_x: u32,
        dest_y: u32,
        width: Option<u32>,
        height: Option<u32>,
    ) {
        dbg!();
        let ret = (XLIB.XPutImage)(
            mem::transmute(self.display),
            self.pixmap,
            self.gc,
            self.image,
            src_x as i32,
            src_y as i32,
            dest_x as i32,
            dest_y as i32,
            width.unwrap_or(self.width),
            height.unwrap_or(self.height),
        );
        dbg!(ret);
        let ret = (XLIB.XCopyArea)(
            mem::transmute(self.display),
            self.pixmap,
            self.window,
            self.gc,
            src_x as i32,
            src_y as i32,
            width.unwrap_or(self.width),
            height.unwrap_or(self.height),
            dest_x as i32,
            dest_y as i32,
        );
    }

    pub fn rows<'a>(&'a self) -> impl ExactSizeIterator + DoubleEndedIterator<Item = &'a [u8]> {
        self.data.chunks_exact(self.width as usize)
    }

    pub fn rows_mut<'a>(
        &'a mut self,
    ) -> impl ExactSizeIterator + DoubleEndedIterator<Item = &'a mut [u8]> {
        self.data.chunks_exact_mut(self.width as usize)
    }
}

impl std::ops::Drop for XlibPixelBuffer {
    fn drop(&mut self) {
        unsafe { (XLIB.XFreePixmap)(mem::transmute(self.display), self.pixmap) };
    }
}
