use std::{convert::TryInto, io};

use raw_window_handle::RawWindowHandle;

use crate::{PixelBufferCreationError, PixelBufferFormatSupported, PixelBufferFormatType};

mod wayland;
mod xlib;

#[cfg(feature = "rayon")]
use rayon::prelude::*;

pub struct PixelBuffer {
    backend: PixelBufferBackend,
}

enum PixelBufferBackend {
    #[cfg(feature = "x11")]
    Xlib(xlib::PixelBuffer),
    #[cfg(feature = "wayland")]
    Wayland(wayland::PixelBuffer),
}

impl PixelBufferFormatSupported for crate::BGRA {}
impl PixelBufferFormatSupported for crate::BGR {}
impl PixelBufferFormatSupported for crate::RGBA {}
impl PixelBufferFormatSupported for crate::RGB {}
pub type NativeFormat = crate::BGRA;

impl PixelBuffer {
    pub unsafe fn new(
        width: u32,
        height: u32,
        format: PixelBufferFormatType,
        raw_window_handle: RawWindowHandle,
    ) -> Result<PixelBuffer, PixelBufferCreationError> {
        match raw_window_handle {
            #[cfg(feature = "x11")]
            RawWindowHandle::Xlib(handle) => xlib::PixelBuffer::new(width, height, format, handle)
                .map(|backend| Self {
                    backend: PixelBufferBackend::Xlib(backend),
                }),
            #[cfg(feature = "x11")]
            RawWindowHandle::Xcb(_handle) => panic!("XCB is currently not supported!"),
            #[cfg(feature = "wayland")]
            RawWindowHandle::Wayland(handle) => {
                wayland::PixelBuffer::new(width, height, format, handle).map(|backend| Self {
                    backend: PixelBufferBackend::Wayland(backend),
                })
            }
            _ => panic!("Unsupported window handle type: {:?}", raw_window_handle),
        }
    }

    pub unsafe fn blit(&self, handle: RawWindowHandle) -> io::Result<()> {
        self.blit_rect((0, 0), (0, 0), (self.width(), self.height()), handle)
    }

    pub unsafe fn blit_rect(
        &self,
        src_pos: (u32, u32),
        dst_pos: (u32, u32),
        blit_size: (u32, u32),
        handle: RawWindowHandle,
    ) -> io::Result<()> {
        match &self.backend {
            #[cfg(feature = "x11")]
            PixelBufferBackend::Xlib(buffer) => {
                let handle = match handle {
                    RawWindowHandle::Xlib(handle) => handle,
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            "Expected Xlib handle, got an XCB handle",
                        ))
                    }
                };
                buffer.blit_rect(src_pos, dst_pos, blit_size, handle)
            }
            #[cfg(feature = "wayland")]
            PixelBufferBackend::Wayland(buffer) => {
                buffer.blit_rect(src_pos, dst_pos, blit_size, handle)
            }
        }
    }

    pub fn bits_per_pixel(&self) -> usize {
        self.bytes_per_pixel() * 8
    }

    pub fn bytes_per_pixel(&self) -> usize {
        match &self.backend {
            #[cfg(feature = "x11")]
            PixelBufferBackend::Xlib(buffer) => buffer.bytes_per_pixel(),
            #[cfg(feature = "wayland")]
            PixelBufferBackend::Wayland(buffer) => buffer.bytes_per_pixel(),
        }
    }

    pub fn width(&self) -> u32 {
        match &self.backend {
            #[cfg(feature = "x11")]
            PixelBufferBackend::Xlib(buffer) => buffer.width(),
            #[cfg(feature = "wayland")]
            PixelBufferBackend::Wayland(buffer) => buffer.width(),
        }
    }

    pub fn row_len(&self) -> usize {
        match &self.backend {
            #[cfg(feature = "x11")]
            PixelBufferBackend::Xlib(buffer) => buffer.row_len(),
            #[cfg(feature = "wayland")]
            PixelBufferBackend::Wayland(buffer) => buffer.row_len(),
        }
    }

    pub fn height(&self) -> u32 {
        match &self.backend {
            #[cfg(feature = "x11")]
            PixelBufferBackend::Xlib(buffer) => buffer.height(),
            #[cfg(feature = "wayland")]
            PixelBufferBackend::Wayland(buffer) => buffer.height(),
        }
    }

    fn bytes(&self) -> &[u8] {
        match &self.backend {
            #[cfg(feature = "x11")]
            PixelBufferBackend::Xlib(buffer) => buffer.bytes(),
            #[cfg(feature = "wayland")]
            PixelBufferBackend::Wayland(buffer) => buffer.bytes(),
        }
    }

    fn bytes_mut(&mut self) -> &mut [u8] {
        match &mut self.backend {
            #[cfg(feature = "x11")]
            PixelBufferBackend::Xlib(buffer) => buffer.bytes_mut(),
            #[cfg(feature = "wayland")]
            PixelBufferBackend::Wayland(buffer) => buffer.bytes_mut(),
        }
    }

    pub fn row(&self, row: u32) -> Option<&[u8]> {
        let index = self.tlo_to_blo(row) as usize * self.row_len();
        let pixel_len = self.width() as usize * self.bytes_per_pixel();
        self.bytes().get(index..index + pixel_len)
    }

    pub fn row_mut(&mut self, row: u32) -> Option<&mut [u8]> {
        let index = self.tlo_to_blo(row) as usize * self.row_len();
        let pixel_len = self.width() as usize * self.bytes_per_pixel();
        self.bytes_mut().get_mut(index..index + pixel_len)
    }

    pub fn rows<'a>(&'a self) -> impl ExactSizeIterator + DoubleEndedIterator<Item = &'a [u8]> {
        let stride = match self.row_len() {
            0 => 1,
            l => l,
        };
        let pixel_len = self.width() as usize * self.bytes_per_pixel();
        self.bytes()
            .chunks(stride)
            .rev()
            .map(move |row| &row[..pixel_len])
    }

    pub fn rows_mut<'a>(
        &'a mut self,
    ) -> impl ExactSizeIterator + DoubleEndedIterator<Item = &'a mut [u8]> {
        let stride = match self.row_len() {
            0 => 1,
            l => l,
        };
        let pixel_len = self.width() as usize * self.bytes_per_pixel();
        self.bytes_mut()
            .chunks_mut(stride)
            .rev()
            .map(move |row| &mut row[..pixel_len])
    }

    #[cfg(feature = "rayon")]
    pub fn par_rows<'a>(&'a self) -> impl IndexedParallelIterator<Item = &'a [u8]> {
        let stride = match self.row_len() {
            0 => 1,
            l => l,
        };
        let pixel_len = self.width() as usize * self.bytes_per_pixel();
        self.bytes()
            .par_chunks(stride)
            .rev()
            .map(move |row| &row[..pixel_len])
    }

    #[cfg(feature = "rayon")]
    pub fn par_rows_mut<'a>(&'a mut self) -> impl IndexedParallelIterator<Item = &'a mut [u8]> {
        let stride = match self.row_len() {
            0 => 1,
            l => l,
        };
        let pixel_len = self.width() as usize * self.bytes_per_pixel();
        self.bytes_mut()
            .par_chunks_mut(stride)
            .rev()
            .map(move |row| &mut row[..pixel_len])
    }

    fn tlo_to_blo(&self, tlo_row: u32) -> u32 {
        self.height() - 1 - tlo_row
    }
}
