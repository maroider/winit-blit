use core::{panic, slice};
use std::{convert::TryInto, io, mem, ptr};

use once_cell::sync::OnceCell;
use raw_window_handle::unix::XlibHandle;
use x11_dl::xlib;

use crate::{PixelBufferCreationError, PixelBufferFormatType};

static XLIB_CONTEXT: OnceCell<XlibContext> = OnceCell::new();

struct XlibContext {
    xlib: xlib::Xlib,
    display: usize,
    screen: i32,
    depth: i32,
    gc: xlib::GC,
}

impl XlibContext {
    pub fn get(display: *mut xlib::Display) -> Result<&'static XlibContext, InitError> {
        let context = XLIB_CONTEXT.get_or_try_init(|| Self::init(display))?;
        assert!(context.display == display as usize);
        Ok(context)
    }

    fn init(display: *mut xlib::Display) -> Result<Self, InitError> {
        let xlib = xlib::Xlib::open().map_err(|err| InitError::OpenXlib(err))?;

        let screen = unsafe { (xlib.XDefaultScreen)(display) };
        let depth = unsafe { (xlib.XDefaultDepth)(display, screen) };

        let gc = unsafe { (xlib.XDefaultGC)(display, screen) };

        Ok(XlibContext {
            xlib,
            display: display as usize,
            screen,
            depth,
            gc,
        })
    }
}

// TODO: Verify that this is safe!
unsafe impl Send for XlibContext {}
unsafe impl Sync for XlibContext {}

#[derive(Clone, Debug)]
enum InitError {
    OpenXlib(x11_dl::error::OpenError),
}

pub struct PixelBuffer {
    xlib_context: &'static XlibContext,
    image: *mut xlib::XImage,
    buf: Vec<u8>,
    width: u32,
    height: u32,
    pixel_byte_count: u32,
}

impl PixelBuffer {
    pub unsafe fn new(
        width: u32,
        height: u32,
        format: PixelBufferFormatType,
        handle: XlibHandle,
    ) -> Result<Self, PixelBufferCreationError> {
        let xlib_context = XlibContext::get(handle.display as *mut _).unwrap();

        let visinfos = {
            let mut count = 0;
            let (red_mask, green_mask, blue_mask) = match format {
                PixelBufferFormatType::BGR | PixelBufferFormatType::BGRA => {
                    (0xFF0000, 0x00FF00, 0x0000FF)
                }
                PixelBufferFormatType::RGB | PixelBufferFormatType::RGBA => {
                    (0x0000FF, 0x00FF00, 0xFF0000)
                }
            };
            let mut template = xlib::XVisualInfo {
                screen: xlib_context.screen,
                depth: xlib_context.depth.try_into().unwrap(),
                red_mask,
                green_mask,
                blue_mask,
                bits_per_rgb: 8,
                ..mem::zeroed()
            };
            let visinfos = (xlib_context.xlib.XGetVisualInfo)(
                handle.display as *mut _,
                xlib::VisualScreenMask
                    | xlib::VisualDepthMask
                    | xlib::VisualRedMaskMask
                    | xlib::VisualGreenMaskMask
                    | xlib::VisualBlueMaskMask
                    | xlib::VisualBitsPerRGBMask,
                &mut template,
                &mut count,
            );
            assert!(!visinfos.is_null(), "No matching visual found");

            slice::from_raw_parts_mut(visinfos, count.try_into().unwrap())
        };

        let pixel_byte_count = match format {
            PixelBufferFormatType::BGR => 3,
            PixelBufferFormatType::BGRA => 4,
            PixelBufferFormatType::RGB => 3,
            PixelBufferFormatType::RGBA => 4,
        };
        let mut buf = vec![0; (pixel_byte_count * width * height).try_into().unwrap()];
        let image = (xlib_context.xlib.XCreateImage)(
            handle.display as *mut _,
            visinfos[0].visual,
            visinfos[0].depth.try_into().unwrap(),
            xlib::ZPixmap,
            0,
            buf.as_mut_ptr() as *mut _,
            width,
            height,
            32,
            0,
        );
        if image.is_null() {
            panic!();
        }

        let mut attributes = xlib::XSetWindowAttributes {
            bit_gravity: xlib::StaticGravity,
            ..mem::zeroed()
        };
        (xlib_context.xlib.XChangeWindowAttributes)(
            handle.display as *mut _,
            handle.window,
            xlib::CWBitGravity,
            &mut attributes,
        );

        (xlib_context.xlib.XFree)(visinfos.as_mut_ptr().cast());
        mem::forget(visinfos);

        Ok(Self {
            xlib_context,
            image,
            buf,
            width,
            height,
            pixel_byte_count,
        })
    }

    pub unsafe fn blit_rect(
        &self,
        src_pos: (u32, u32),
        dst_pos: (u32, u32),
        blit_size: (u32, u32),
        handle: XlibHandle,
    ) -> io::Result<()> {
        let result = (self.xlib_context.xlib.XPutImage)(
            handle.display as *mut _,
            handle.window,
            self.xlib_context.gc,
            self.image,
            src_pos.0.try_into().unwrap(),
            src_pos.1.try_into().unwrap(),
            dst_pos.0.try_into().unwrap(),
            dst_pos.1.try_into().unwrap(),
            blit_size.0,
            blit_size.1,
        )
        .try_into()
        .unwrap();
        if result != xlib::Success {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                match result {
                    xlib::BadDrawable => "BadDrawable".to_string(),
                    xlib::BadGC => "BadGC".to_string(),
                    xlib::BadMatch => "BadMatch".to_string(),
                    xlib::BadValue => "BadValue".to_string(),
                    _ => format!("Unknown error: {:#X}", result),
                },
            ));
        }
        Ok(())
    }

    pub fn bytes_per_pixel(&self) -> usize {
        self.pixel_byte_count.try_into().unwrap()
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn row_len(&self) -> usize {
        (self.pixel_byte_count * self.width).try_into().unwrap()
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn bytes(&self) -> &[u8] {
        &self.buf
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.buf
    }
}

impl Drop for PixelBuffer {
    fn drop(&mut self) {
        unsafe { (*self.image).data = ptr::null_mut() };
        unsafe { (self.xlib_context.xlib.XDestroyImage)(self.image) };
    }
}
