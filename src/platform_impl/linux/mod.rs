use crate::{PixelBufferCreationError, PixelBufferFormatSupported, PixelBufferFormatType};
use raw_window_handle::{
    unix::{WaylandHandle, XcbHandle, XlibHandle},
    RawWindowHandle,
};
use std::io;

#[cfg(all(not(feature = "x11"), not(feature = "wayland")))]
compile_error!("Please select a feature to build for unix: `x11`, `wayland`");

#[cfg(feature = "wayland")]
mod wayland;
#[cfg(feature = "x11")]
mod x11;

pub struct PixelBuffer {
    inner: PixelBufferInner,
    handle: RawWindowHandle,
}

// TODO: Figure out if BGRA is a good format for the "native format".
//       My gut is telling me "yes", but it never hurts to check properly.
pub type NativeFormat = crate::BGRA;

impl PixelBuffer {
    pub unsafe fn new(
        width: u32,
        height: u32,
        format: PixelBufferFormatType,
        raw_window_handle: RawWindowHandle,
    ) -> Result<PixelBuffer, PixelBufferCreationError> {
        match raw_window_handle {
            RawWindowHandle::Xlib(handle) => Ok(PixelBuffer {
                inner: PixelBufferInner::new_xlib(width, height, format, handle)?,
                handle: raw_window_handle,
            }),
            RawWindowHandle::Xcb(_handle) => panic!("XCB is currently not supported."),
            RawWindowHandle::Wayland(handle) => {
                todo!("Wayland support is currently not implemented")
            }
            _ => panic!("Unsupported handle type"),
        }
    }

    pub unsafe fn blit(&self, handle: RawWindowHandle) -> io::Result<()> {
        if handle == self.handle {
            self.inner.blit();
            Ok(())
        } else {
            todo!("Give an appropriate error")
        }
    }

    pub unsafe fn blit_rect(
        &self,
        src_pos: (u32, u32),
        dst_pos: (u32, u32),
        blit_size: (u32, u32),
        handle: RawWindowHandle,
    ) -> io::Result<()> {
        todo!()
    }

    pub fn bits_per_pixel(&self) -> usize {
        todo!()
    }

    pub fn bytes_per_pixel(&self) -> usize {
        self.bits_per_pixel() / 8
    }

    pub fn width(&self) -> u32 {
        todo!()
    }

    pub fn row_len(&self) -> usize {
        todo!()
    }

    pub fn height(&self) -> u32 {
        todo!()
    }

    pub fn bytes(&self) -> &[u8] {
        todo!()
    }

    pub fn bytes_mut(&mut self) -> &mut [u8] {
        todo!()
    }

    pub fn row(&self, row: u32) -> Option<&[u8]> {
        todo!()
    }

    pub fn row_mut(&mut self, row: u32) -> Option<&mut [u8]> {
        todo!()
    }

    pub fn rows<'a>(&'a self) -> impl ExactSizeIterator + DoubleEndedIterator<Item = &'a [u8]> {
        self.inner.rows()
    }

    pub fn rows_mut<'a>(
        &'a mut self,
    ) -> impl ExactSizeIterator + DoubleEndedIterator<Item = &'a mut [u8]> {
        self.inner.rows_mut()
    }

    #[cfg(feature = "rayon")]
    pub fn par_rows<'a>(&'a self) -> impl IndexedParallelIterator<Item = &'a [u8]> {
        todo!()
    }

    #[cfg(feature = "rayon")]
    pub fn par_rows_mut<'a>(&'a mut self) -> impl IndexedParallelIterator<Item = &'a mut [u8]> {
        todo!()
    }
}

impl PixelBufferFormatSupported for crate::BGRA {}

enum PixelBufferInner {
    Xlib(x11::XlibPixelBuffer),
    Wayland(wayland::WaylandPixelBuffer),
}

impl PixelBufferInner {
    unsafe fn new_xlib(
        width: u32,
        height: u32,
        format: PixelBufferFormatType,
        handle: raw_window_handle::unix::XlibHandle,
    ) -> Result<Self, PixelBufferCreationError> {
        Ok(Self::Xlib(x11::XlibPixelBuffer::new(
            width, height, format, handle,
        )?))
    }

    unsafe fn new_wayland(
        width: u32,
        height: u32,
        format: PixelBufferFormatType,
        handle: raw_window_handle::unix::WaylandHandle,
    ) -> Result<Self, PixelBufferCreationError> {
        Ok(Self::Wayland(wayland::WaylandPixelBuffer::new(
            width, height, format, handle,
        )?))
    }

    fn rows<'a>(&'a self) -> impl ExactSizeIterator + DoubleEndedIterator<Item = &'a [u8]> {
        match self {
            Self::Xlib(buffer) => buffer.rows(),
            Self::Wayland(_buffer) => todo!(),
        }
    }

    fn rows_mut<'a>(
        &'a mut self,
    ) -> impl ExactSizeIterator + DoubleEndedIterator<Item = &'a mut [u8]> {
        match self {
            Self::Xlib(buffer) => buffer.rows_mut(),
            Self::Wayland(_buffer) => todo!(),
        }
    }

    unsafe fn blit(&self) {
        match self {
            Self::Xlib(buffer) => buffer.blit(0, 0, 0, 0, None, None),
            Self::Wayland(_buffer) => todo!(),
        }
    }
}
