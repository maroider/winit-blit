use crate::{PixelBufferCreationError, PixelBufferFormatType};

pub struct WaylandPixelBuffer {}

impl WaylandPixelBuffer {
    pub unsafe fn new(
        width: u32,
        height: u32,
        format: PixelBufferFormatType,
        handle: raw_window_handle::unix::WaylandHandle,
    ) -> Result<Self, PixelBufferCreationError> {
        todo!()
    }
}
