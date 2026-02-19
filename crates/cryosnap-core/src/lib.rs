const FONT_HEIGHT_TO_WIDTH_RATIO: f32 = 1.68;
const DEFAULT_TAB_WIDTH: usize = 4;
const ANSI_TAB_WIDTH: usize = 6;
const WINDOW_CONTROLS_HEIGHT: f32 = 18.0;
const WINDOW_CONTROLS_X_OFFSET: f32 = 12.0;
const WINDOW_CONTROLS_SPACING: f32 = 19.0;
const DEFAULT_WEBP_QUALITY: f32 = 90.0;
const DEFAULT_RASTER_SCALE: f32 = 4.0;
const DEFAULT_RASTER_MAX_PIXELS: u64 = 8_000_000;
const DEFAULT_PNG_OPT_LEVEL: u8 = 0;
const MAX_PNG_OPT_LEVEL: u8 = 6;
const DEFAULT_PNG_QUANTIZE_QUALITY: u8 = 85;
const DEFAULT_PNG_QUANTIZE_SPEED: u8 = 4;
const DEFAULT_PNG_QUANTIZE_DITHER: f32 = 1.0;
const DEFAULT_TITLE_SIZE: f32 = 12.0;
const DEFAULT_TITLE_OPACITY: f32 = 0.85;
const DEFAULT_TITLE_MAX_WIDTH: usize = 80;

mod ansi;
mod config;
mod fonts;
mod input;
mod layout;
mod png;
mod render;
mod svg;
mod syntax;
mod text;
mod types;
pub use config::{
    Border, CjkRegion, Config, Font, FontSystemFallback, PngOptions, PngQuantPreset, PngStrip,
    RasterBackend, RasterOptions, Shadow, TitleAlign, TitleOptions, TitlePathStyle,
};
pub use render::{
    render, render_png, render_png_from_svg, render_svg, render_webp, render_webp_from_svg,
};
pub use types::{Error, InputSource, OutputFormat, RenderRequest, RenderResult, Result};

#[derive(Debug, Clone, Default, PartialEq)]
struct TextStyle {
    fg: Option<String>,
    bg: Option<String>,
    bold: bool,
    italic: bool,
    underline: bool,
    strike: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Span {
    pub(crate) text: String,
    style: TextStyle,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Line {
    pub(crate) spans: Vec<Span>,
}

#[cfg(test)]
mod tests;
