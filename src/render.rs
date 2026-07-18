use aviutl2::AnyResult;
use skia_safe::{AlphaType, Color, ColorType, ImageInfo, surfaces};

mod canvas;
mod path;
mod stroke;
mod text;
mod transform;
mod util;

use self::canvas::run_canvas_commands;
use self::util::invalid_input;

pub(crate) fn render_to_rgba_buffer(w: i32, h: i32, commands: &str) -> AnyResult<Box<[u8]>> {
    if w <= 0 || h <= 0 {
        return Err(invalid_input("image size must be positive").into());
    }

    // check size
    let width = w as usize;
    let height = h as usize;
    let row_bytes = width
        .checked_mul(4)
        .ok_or_else(|| invalid_input("image row is too large"))?;
    let byte_len = width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(4))
        .ok_or_else(|| invalid_input("image size is too large"))?;

    // allocate memory
    let mut rgba = vec![0u8; byte_len].into_boxed_slice();

    // draw directly into the RGBA buffer
    {
        // create skia canvas
        let image_info = ImageInfo::new((w, h), ColorType::RGBA8888, AlphaType::Unpremul, None);
        let mut surface = surfaces::wrap_pixels(&image_info, rgba.as_mut(), Some(row_bytes), None)
            .ok_or_else(|| invalid_input("failed to create skia raster surface"))?;
        let canvas = surface.canvas();

        // clear
        canvas.clear(Color::TRANSPARENT);
        canvas.translate((w as f32 * 0.5, h as f32 * 0.5));

        // draw
        run_canvas_commands(canvas, commands)?;
    }

    Ok(rgba)
}
