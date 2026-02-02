use base64::Engine;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

const FONT_HEIGHT_TO_WIDTH_RATIO: f32 = 1.68;
const DEFAULT_TAB_WIDTH: usize = 4;
const ANSI_TAB_WIDTH: usize = 6;
const WINDOW_CONTROLS_HEIGHT: f32 = 18.0;
const WINDOW_CONTROLS_X_OFFSET: f32 = 12.0;
const WINDOW_CONTROLS_SPACING: f32 = 19.0;
const DEFAULT_WEBP_QUALITY: f32 = 90.0;
const DEFAULT_RASTER_SCALE: f32 = 4.0;
const DEFAULT_RASTER_MAX_PIXELS: u64 = 8_000_000;
const DEFAULT_PNG_OPT_LEVEL: u8 = 4;
const MAX_PNG_OPT_LEVEL: u8 = 6;
const DEFAULT_PNG_QUANTIZE_QUALITY: u8 = 85;
const DEFAULT_PNG_QUANTIZE_SPEED: u8 = 4;
const DEFAULT_PNG_QUANTIZE_DITHER: f32 = 1.0;
const DEFAULT_TITLE_SIZE: f32 = 12.0;
const DEFAULT_TITLE_OPACITY: f32 = 0.85;
const DEFAULT_TITLE_MAX_WIDTH: usize = 80;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub theme: String,
    pub background: String,
    #[serde(deserialize_with = "deserialize_box")]
    pub padding: Vec<f32>,
    #[serde(deserialize_with = "deserialize_box")]
    pub margin: Vec<f32>,
    pub width: f32,
    pub height: f32,
    #[serde(rename = "window")]
    pub window_controls: bool,
    #[serde(rename = "show_line_numbers")]
    pub show_line_numbers: bool,
    pub language: Option<String>,
    pub execute_timeout_ms: u64,
    pub wrap: usize,
    #[serde(deserialize_with = "deserialize_lines")]
    pub lines: Vec<i32>,
    pub border: Border,
    pub shadow: Shadow,
    pub font: Font,
    #[serde(rename = "line_height")]
    pub line_height: f32,
    pub raster: RasterOptions,
    pub png: PngOptions,
    pub title: TitleOptions,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme: "charm".to_string(),
            background: "#171717".to_string(),
            padding: vec![20.0, 40.0, 20.0, 20.0],
            margin: vec![0.0],
            width: 0.0,
            height: 0.0,
            window_controls: false,
            show_line_numbers: false,
            language: None,
            execute_timeout_ms: 10_000,
            wrap: 0,
            lines: vec![0, -1],
            border: Border::default(),
            shadow: Shadow::default(),
            font: Font::default(),
            line_height: 1.2,
            raster: RasterOptions::default(),
            png: PngOptions::default(),
            title: TitleOptions::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Border {
    pub radius: f32,
    pub width: f32,
    pub color: String,
}

impl Default for Border {
    fn default() -> Self {
        Self {
            radius: 0.0,
            width: 0.0,
            color: "#515151".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Shadow {
    pub blur: f32,
    pub x: f32,
    pub y: f32,
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            blur: 0.0,
            x: 0.0,
            y: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Font {
    pub family: String,
    pub file: Option<String>,
    pub size: f32,
    pub ligatures: bool,
}

impl Default for Font {
    fn default() -> Self {
        Self {
            family: "JetBrains Mono".to_string(),
            file: None,
            size: 14.0,
            ligatures: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RasterOptions {
    pub scale: f32,
    pub max_pixels: u64,
    pub backend: RasterBackend,
}

impl Default for RasterOptions {
    fn default() -> Self {
        Self {
            scale: DEFAULT_RASTER_SCALE,
            max_pixels: DEFAULT_RASTER_MAX_PIXELS,
            backend: RasterBackend::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RasterBackend {
    #[default]
    Auto,
    Resvg,
    Rsvg,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TitleAlign {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TitlePathStyle {
    #[default]
    Absolute,
    Relative,
    Basename,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TitleOptions {
    pub enabled: bool,
    pub text: Option<String>,
    pub path_style: TitlePathStyle,
    pub tmux_format: String,
    pub align: TitleAlign,
    pub size: f32,
    pub color: String,
    pub opacity: f32,
    pub max_width: usize,
    pub ellipsis: String,
}

impl Default for TitleOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            text: None,
            path_style: TitlePathStyle::Absolute,
            tmux_format: "#{session_name}:#{window_index}.#{pane_index} #{pane_title}".to_string(),
            align: TitleAlign::Center,
            size: DEFAULT_TITLE_SIZE,
            color: "#C5C8C6".to_string(),
            opacity: DEFAULT_TITLE_OPACITY,
            max_width: DEFAULT_TITLE_MAX_WIDTH,
            ellipsis: "…".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PngStrip {
    None,
    #[default]
    Safe,
    All,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PngQuantPreset {
    Fast,
    Balanced,
    Best,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PngOptions {
    pub optimize: bool,
    pub level: u8,
    pub strip: PngStrip,
    pub quantize: bool,
    pub quantize_preset: Option<PngQuantPreset>,
    pub quantize_quality: u8,
    pub quantize_speed: u8,
    pub quantize_dither: f32,
}

impl Default for PngOptions {
    fn default() -> Self {
        Self {
            optimize: true,
            level: DEFAULT_PNG_OPT_LEVEL,
            strip: PngStrip::Safe,
            quantize: false,
            quantize_preset: None,
            quantize_quality: DEFAULT_PNG_QUANTIZE_QUALITY,
            quantize_speed: DEFAULT_PNG_QUANTIZE_SPEED,
            quantize_dither: DEFAULT_PNG_QUANTIZE_DITHER,
        }
    }
}

#[derive(Debug, Clone)]
pub enum InputSource {
    Text(String),
    File(PathBuf),
    Command(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Svg,
    Png,
    Webp,
}

#[derive(Debug, Clone)]
pub struct RenderRequest {
    pub input: InputSource,
    pub config: Config,
    pub format: OutputFormat,
}

#[derive(Debug, Clone)]
pub struct RenderResult {
    pub format: OutputFormat,
    pub bytes: Vec<u8>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("render error: {0}")]
    Render(String),
    #[error("execution timeout")]
    Timeout,
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn render(request: &RenderRequest) -> Result<RenderResult> {
    let bytes = match request.format {
        OutputFormat::Svg => render_svg(&request.input, &request.config)?,
        OutputFormat::Png => render_png(&request.input, &request.config)?,
        OutputFormat::Webp => render_webp(&request.input, &request.config)?,
    };
    Ok(RenderResult {
        format: request.format,
        bytes,
    })
}

pub fn render_svg(input: &InputSource, config: &Config) -> Result<Vec<u8>> {
    let loaded = load_input(input, Duration::from_millis(config.execute_timeout_ms))?;
    let is_ansi = is_ansi_input(&loaded, config);
    let line_window = &config.lines;

    let (lines, default_fg, line_offset) = if is_ansi {
        let cut = cut_text(&loaded.text, line_window);
        let mut lines = parse_ansi(&cut.text);
        if config.wrap > 0 {
            lines = wrap_ansi_lines(&lines, config.wrap);
        }
        (lines, "#C5C8C6".to_string(), cut.start)
    } else {
        let mut text = detab(&loaded.text, DEFAULT_TAB_WIDTH);
        let cut = cut_text(&text, line_window);
        text = cut.text;
        if config.wrap > 0 {
            text = wrap_text(&text, config.wrap);
        }
        let (lines, default_fg) = highlight_code(
            &text,
            loaded.path.as_deref(),
            config.language.as_deref(),
            &config.theme,
        )?;
        (lines, default_fg, cut.start)
    };

    let font_css = svg_font_face_css(config)?;
    let title_text = resolve_title_text(input, config);
    let svg = build_svg(
        &lines,
        config,
        &default_fg,
        font_css,
        line_offset,
        title_text.as_deref(),
    );
    Ok(svg.into_bytes())
}

pub fn render_png(input: &InputSource, config: &Config) -> Result<Vec<u8>> {
    let svg = render_svg(input, config)?;
    render_png_from_svg(&svg, config)
}

pub fn render_webp(input: &InputSource, config: &Config) -> Result<Vec<u8>> {
    let svg = render_svg(input, config)?;
    render_webp_from_svg(&svg, config)
}

pub fn render_png_from_svg(svg: &[u8], config: &Config) -> Result<Vec<u8>> {
    if let Some(png) = try_render_png_with_rsvg(svg, config)? {
        let png = if config.png.quantize {
            quantize_png_bytes(&png, &config.png)?
        } else {
            png
        };
        return optimize_png(png, &config.png);
    }

    let pixmap = rasterize_svg(svg, config)?;
    let png = if config.png.quantize {
        quantize_pixmap_to_png(&pixmap, &config.png)?
    } else {
        pixmap
            .encode_png()
            .map_err(|err| Error::Render(format!("png encode: {err}")))?
    };
    optimize_png(png, &config.png)
}

pub fn render_webp_from_svg(svg: &[u8], config: &Config) -> Result<Vec<u8>> {
    if matches!(config.raster.backend, RasterBackend::Rsvg) {
        return Err(Error::Render(
            "rsvg backend does not support webp output".to_string(),
        ));
    }
    let pixmap = rasterize_svg(svg, config)?;
    pixmap_to_webp(&pixmap)
}

fn try_render_png_with_rsvg(svg: &[u8], config: &Config) -> Result<Option<Vec<u8>>> {
    let backend = config.raster.backend;
    if matches!(backend, RasterBackend::Resvg) {
        return Ok(None);
    }
    let Some(bin) = RSVG_CONVERT.as_ref().cloned() else {
        if matches!(backend, RasterBackend::Rsvg) {
            return Err(Error::Render("rsvg-convert not found in PATH".to_string()));
        }
        return Ok(None);
    };
    match rsvg_convert_png(svg, config, &bin) {
        Ok(png) => Ok(Some(png)),
        Err(err) => {
            if matches!(backend, RasterBackend::Rsvg) {
                Err(err)
            } else {
                Ok(None)
            }
        }
    }
}

static RSVG_CONVERT: Lazy<Option<PathBuf>> = Lazy::new(find_rsvg_convert);

fn find_rsvg_convert() -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["rsvg-convert.exe", "rsvg-convert"]
    } else {
        &["rsvg-convert"]
    };
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        for name in names {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn rsvg_convert_png(svg: &[u8], config: &Config, bin: &Path) -> Result<Vec<u8>> {
    let (width, height) = svg_dimensions(svg)?;
    let scale = raster_scale(config, width, height)?;

    let mut cmd = Command::new(bin);
    cmd.arg("--format").arg("png");
    if (scale - 1.0).abs() > f32::EPSILON {
        cmd.arg("--zoom").arg(format!("{scale:.6}"));
    }
    cmd.arg("-");
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| Error::Render(format!("rsvg-convert spawn: {err}")))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(svg)
            .map_err(|err| Error::Render(format!("rsvg-convert stdin: {err}")))?;
    } else {
        return Err(Error::Render("rsvg-convert stdin unavailable".to_string()));
    }

    let output = child
        .wait_with_output()
        .map_err(|err| Error::Render(format!("rsvg-convert wait: {err}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let message = stderr.trim();
        if message.is_empty() {
            return Err(Error::Render("rsvg-convert failed".to_string()));
        }
        return Err(Error::Render(format!("rsvg-convert failed: {message}")));
    }
    if output.stdout.is_empty() {
        return Err(Error::Render(
            "rsvg-convert returned empty output".to_string(),
        ));
    }
    Ok(output.stdout)
}

fn svg_dimensions(svg: &[u8]) -> Result<(u32, u32)> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_data(svg, &opt)
        .map_err(|err| Error::Render(format!("usvg parse: {err}")))?;
    let size = tree.size().to_int_size();
    Ok((size.width(), size.height()))
}

fn rasterize_svg(svg: &[u8], config: &Config) -> Result<tiny_skia::Pixmap> {
    let mut opt = usvg::Options::default();
    let fontdb = build_fontdb(config)?;
    *opt.fontdb_mut() = fontdb;

    let tree = usvg::Tree::from_data(svg, &opt)
        .map_err(|err| Error::Render(format!("usvg parse: {err}")))?;
    let size = tree.size().to_int_size();
    let scale = raster_scale(config, size.width(), size.height())?;
    let width = scale_dimension(size.width(), scale)?;
    let height = scale_dimension(size.height(), scale)?;

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| Error::Render(format!("invalid pixmap size {width}x{height}")))?;
    let mut pixmap_mut = pixmap.as_mut();
    let transform = if (scale - 1.0).abs() < f32::EPSILON {
        tiny_skia::Transform::identity()
    } else {
        tiny_skia::Transform::from_scale(scale, scale)
    };
    resvg::render(&tree, transform, &mut pixmap_mut);

    Ok(pixmap)
}

fn raster_scale(config: &Config, base_width: u32, base_height: u32) -> Result<f32> {
    let mut scale = if config.width == 0.0 && config.height == 0.0 {
        config.raster.scale
    } else {
        1.0
    };
    if !scale.is_finite() || scale <= 0.0 {
        return Err(Error::Render("invalid raster scale".to_string()));
    }
    if config.raster.max_pixels > 0 {
        let base_pixels = base_width as f64 * base_height as f64;
        if base_pixels > 0.0 {
            let max_pixels = config.raster.max_pixels as f64;
            let requested_pixels = base_pixels * (scale as f64).powi(2);
            if requested_pixels > max_pixels {
                let max_scale = (max_pixels / base_pixels).sqrt() as f32;
                if max_scale.is_finite() && max_scale > 0.0 {
                    scale = scale.min(max_scale);
                }
            }
        }
    }
    Ok(scale)
}

fn scale_dimension(value: u32, scale: f32) -> Result<u32> {
    let scaled = (value as f32 * scale).round();
    if !scaled.is_finite() || scaled <= 0.0 {
        return Err(Error::Render("invalid raster scale".to_string()));
    }
    if scaled > u32::MAX as f32 {
        return Err(Error::Render("raster size overflow".to_string()));
    }
    Ok(scaled as u32)
}

fn resolve_title_text(input: &InputSource, config: &Config) -> Option<String> {
    if !config.title.enabled || !config.window_controls {
        return None;
    }
    if let Some(text) = config.title.text.as_ref() {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    let auto = match input {
        InputSource::File(path) => title_text_from_path(path, config.title.path_style),
        InputSource::Command(cmd) => format!("cmd: {}", cmd),
        InputSource::Text(_) => return None,
    };
    let sanitized = sanitize_title_text(&auto);
    if sanitized.is_empty() {
        None
    } else {
        Some(sanitized)
    }
}

fn title_text_from_path(path: &Path, style: TitlePathStyle) -> String {
    match style {
        TitlePathStyle::Basename => path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string()),
        TitlePathStyle::Relative => {
            let cwd = std::env::current_dir().ok();
            if let Some(cwd) = cwd {
                if let Ok(relative) = path.strip_prefix(&cwd) {
                    return relative.to_string_lossy().to_string();
                }
            }
            path.to_string_lossy().to_string()
        }
        TitlePathStyle::Absolute => path
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_string(),
    }
}

fn sanitize_title_text(text: &str) -> String {
    text.replace(['\n', '\r'], " ").trim().to_string()
}

fn text_width_cells(text: &str) -> usize {
    let mut width = 0usize;
    for ch in text.chars() {
        if ch == '\t' {
            width += DEFAULT_TAB_WIDTH;
        } else {
            width += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        }
    }
    width
}

fn truncate_to_cells(text: &str, max_cells: usize, ellipsis: &str) -> String {
    if max_cells == 0 {
        return String::new();
    }
    let width = text_width_cells(text);
    if width <= max_cells {
        return text.to_string();
    }
    let ellipsis_width = text_width_cells(ellipsis);
    if ellipsis_width >= max_cells {
        return ellipsis.chars().take(1).collect();
    }
    let mut out = String::new();
    let mut current = 0usize;
    for ch in text.chars() {
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if current + w > max_cells - ellipsis_width {
            break;
        }
        out.push(ch);
        current += w;
    }
    out.push_str(ellipsis);
    out
}

fn is_jetbrains_mono(family: &str) -> bool {
    family.eq_ignore_ascii_case("JetBrains Mono")
}

fn is_hack_nerd_font(family: &str) -> bool {
    family.eq_ignore_ascii_case("Hack Nerd Font")
        || family.eq_ignore_ascii_case("Hack Nerd Font Mono")
}

fn build_fontdb(config: &Config) -> Result<usvg::fontdb::Database> {
    let mut fontdb = usvg::fontdb::Database::new();
    let needs_system_fonts = config.font.file.is_none()
        && !is_jetbrains_mono(&config.font.family)
        && !is_hack_nerd_font(&config.font.family);
    if needs_system_fonts {
        fontdb.load_system_fonts();
    }
    if let Some(font_file) = &config.font.file {
        let bytes = std::fs::read(font_file)?;
        fontdb.load_font_data(bytes);
    } else if is_jetbrains_mono(&config.font.family) {
        if config.font.ligatures {
            fontdb.load_font_data(JETBRAINS_MONO_REGULAR.to_vec());
        } else {
            fontdb.load_font_data(JETBRAINS_MONO_NL.to_vec());
        }
    } else if is_hack_nerd_font(&config.font.family) {
        fontdb.load_font_data(HACK_NERD_FONT_REGULAR.to_vec());
    }
    Ok(fontdb)
}

fn pixmap_to_webp(pixmap: &tiny_skia::Pixmap) -> Result<Vec<u8>> {
    let width = pixmap.width();
    let height = pixmap.height();
    let rgba = unpremultiply_rgba(pixmap.data());
    let encoder = webp::Encoder::from_rgba(&rgba, width, height);
    let webp = encoder.encode(DEFAULT_WEBP_QUALITY);
    Ok(webp.to_vec())
}

fn unpremultiply_rgba(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    for chunk in data.chunks_exact(4) {
        let a = chunk[3] as u16;
        if a == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
            continue;
        }
        let r = ((chunk[0] as u16 * 255 + a / 2) / a) as u8;
        let g = ((chunk[1] as u16 * 255 + a / 2) / a) as u8;
        let b = ((chunk[2] as u16 * 255 + a / 2) / a) as u8;
        out.extend_from_slice(&[r, g, b, chunk[3]]);
    }
    out
}

fn quantize_pixmap_to_png(pixmap: &tiny_skia::Pixmap, config: &PngOptions) -> Result<Vec<u8>> {
    let rgba = unpremultiply_rgba(pixmap.data());
    quantize_rgba_to_png(&rgba, pixmap.width(), pixmap.height(), config)
}

fn quantize_png_bytes(png: &[u8], config: &PngOptions) -> Result<Vec<u8>> {
    let (rgba, width, height) = decode_png_rgba(png)?;
    quantize_rgba_to_png(&rgba, width, height, config)
}

fn decode_png_rgba(png: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    let mut decoder = png::Decoder::new(Cursor::new(png));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder
        .read_info()
        .map_err(|err| Error::Render(format!("png decode: {err}")))?;
    let buffer_size = reader
        .output_buffer_size()
        .ok_or_else(|| Error::Render("png decode: missing buffer size".to_string()))?;
    let mut buf = vec![0; buffer_size];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|err| Error::Render(format!("png decode: {err}")))?;
    let data = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => rgb_to_rgba(data),
        png::ColorType::GrayscaleAlpha => gray_alpha_to_rgba(data),
        png::ColorType::Grayscale => gray_to_rgba(data),
        png::ColorType::Indexed => {
            return Err(Error::Render(
                "png decode: indexed color not expanded".to_string(),
            ));
        }
    };
    Ok((rgba, info.width, info.height))
}

fn rgb_to_rgba(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 3 * 4);
    for chunk in data.chunks_exact(3) {
        out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
    }
    out
}

fn gray_to_rgba(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 4);
    for &g in data {
        out.extend_from_slice(&[g, g, g, 255]);
    }
    out
}

fn gray_alpha_to_rgba(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 2 * 4);
    for chunk in data.chunks_exact(2) {
        let g = chunk[0];
        let a = chunk[1];
        out.extend_from_slice(&[g, g, g, a]);
    }
    out
}

#[derive(Clone, Copy)]
struct QuantizeSettings {
    quality: u8,
    speed: u8,
    dither: f32,
}

impl PngQuantPreset {
    fn settings(self) -> QuantizeSettings {
        match self {
            PngQuantPreset::Fast => QuantizeSettings {
                quality: 70,
                speed: 7,
                dither: 0.5,
            },
            PngQuantPreset::Balanced => QuantizeSettings {
                quality: DEFAULT_PNG_QUANTIZE_QUALITY,
                speed: DEFAULT_PNG_QUANTIZE_SPEED,
                dither: DEFAULT_PNG_QUANTIZE_DITHER,
            },
            PngQuantPreset::Best => QuantizeSettings {
                quality: 95,
                speed: 1,
                dither: 1.0,
            },
        }
    }
}

fn quantize_settings(config: &PngOptions) -> QuantizeSettings {
    if let Some(preset) = config.quantize_preset {
        return preset.settings();
    }
    QuantizeSettings {
        quality: config.quantize_quality,
        speed: config.quantize_speed,
        dither: config.quantize_dither,
    }
}

fn quantize_rgba_to_png(
    rgba: &[u8],
    width: u32,
    height: u32,
    config: &PngOptions,
) -> Result<Vec<u8>> {
    let expected = width as usize * height as usize * 4;
    if rgba.len() != expected {
        return Err(Error::Render(
            "png quantize: invalid rgba buffer".to_string(),
        ));
    }
    let mut pixels = Vec::with_capacity(width as usize * height as usize);
    for chunk in rgba.chunks_exact(4) {
        pixels.push(imagequant::RGBA::new(
            chunk[0], chunk[1], chunk[2], chunk[3],
        ));
    }

    let mut attr = imagequant::new();
    let settings = quantize_settings(config);
    let quality = settings.quality.min(100);
    let speed = settings.speed.clamp(1, 10);
    attr.set_quality(0, quality)
        .map_err(|err| Error::Render(format!("png quantize quality: {err:?}")))?;
    attr.set_speed(speed as i32)
        .map_err(|err| Error::Render(format!("png quantize speed: {err:?}")))?;
    let mut image = attr
        .new_image(pixels, width as usize, height as usize, 0.0)
        .map_err(|err| Error::Render(format!("png quantize image: {err:?}")))?;
    let mut result = attr
        .quantize(&mut image)
        .map_err(|err| Error::Render(format!("png quantize: {err:?}")))?;
    let dither = settings.dither.clamp(0.0, 1.0);
    result
        .set_dithering_level(dither)
        .map_err(|err| Error::Render(format!("png quantize dither: {err:?}")))?;
    let (palette, indices) = result
        .remapped(&mut image)
        .map_err(|err| Error::Render(format!("png quantize remap: {err:?}")))?;
    encode_indexed_png(&palette, &indices, width, height)
}

fn encode_indexed_png(
    palette: &[imagequant::RGBA],
    indices: &[u8],
    width: u32,
    height: u32,
) -> Result<Vec<u8>> {
    if indices.len() != width as usize * height as usize {
        return Err(Error::Render(
            "png quantize: invalid index buffer".to_string(),
        ));
    }
    let mut palette_bytes = Vec::with_capacity(palette.len() * 3);
    let mut trns = Vec::with_capacity(palette.len());
    let mut has_alpha = false;
    for color in palette {
        palette_bytes.extend_from_slice(&[color.r, color.g, color.b]);
        trns.push(color.a);
        if color.a < 255 {
            has_alpha = true;
        }
    }
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, width, height);
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_palette(palette_bytes);
    if has_alpha {
        encoder.set_trns(trns);
    }
    let mut writer = encoder
        .write_header()
        .map_err(|err| Error::Render(format!("png encode: {err}")))?;
    writer
        .write_image_data(indices)
        .map_err(|err| Error::Render(format!("png encode: {err}")))?;
    drop(writer);
    Ok(out)
}

fn optimize_png(png: Vec<u8>, config: &PngOptions) -> Result<Vec<u8>> {
    if !config.optimize {
        return Ok(png);
    }
    let level = config.level.min(MAX_PNG_OPT_LEVEL);
    let mut options = oxipng::Options::from_preset(level);
    options.strip = match config.strip {
        PngStrip::None => oxipng::StripChunks::None,
        PngStrip::Safe => oxipng::StripChunks::Safe,
        PngStrip::All => oxipng::StripChunks::All,
    };
    oxipng::optimize_from_memory(&png, &options)
        .map_err(|err| Error::Render(format!("png optimize: {err}")))
}

struct LoadedInput {
    text: String,
    path: Option<PathBuf>,
    kind: InputKind,
}

#[derive(Debug, Clone, Copy)]
enum InputKind {
    Code,
    Ansi,
}

fn load_input(input: &InputSource, timeout: Duration) -> Result<LoadedInput> {
    match input {
        InputSource::Text(text) => Ok(LoadedInput {
            text: text.clone(),
            path: None,
            kind: InputKind::Code,
        }),
        InputSource::File(path) => {
            let text = std::fs::read_to_string(path)?;
            Ok(LoadedInput {
                text,
                path: Some(path.clone()),
                kind: InputKind::Code,
            })
        }
        InputSource::Command(cmd) => {
            let text = execute_command(cmd, timeout)?;
            Ok(LoadedInput {
                text,
                path: None,
                kind: InputKind::Ansi,
            })
        }
    }
}

fn is_ansi_input(loaded: &LoadedInput, config: &Config) -> bool {
    if let Some(lang) = &config.language {
        if lang.eq_ignore_ascii_case("ansi") {
            return true;
        }
    }
    if matches!(loaded.kind, InputKind::Ansi) {
        return true;
    }
    loaded.text.contains('\u{1b}')
}

fn execute_command(cmd: &str, timeout: Duration) -> Result<String> {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};
    use std::io::Read;
    use std::sync::mpsc;
    use std::thread;

    let args = shell_words::split(cmd)
        .map_err(|err| Error::InvalidInput(format!("command parse: {err}")))?;
    if args.is_empty() {
        return Err(Error::InvalidInput("empty command".to_string()));
    }

    let (cols, rows) = terminal_size::terminal_size()
        .map(|(w, h)| (w.0, h.0))
        .unwrap_or((80, 24));

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| Error::Render(format!("open pty: {err}")))?;

    let mut command = CommandBuilder::new(&args[0]);
    if args.len() > 1 {
        command.args(&args[1..]);
    }

    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|err| Error::Render(format!("spawn command: {err}")))?;
    drop(pair.slave);
    let mut killer = child.clone_killer();

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|err| Error::Render(format!("pty reader: {err}")))?;
    drop(pair.master);

    let read_handle = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = reader.read_to_end(&mut buf);
        buf
    });

    let (status_tx, status_rx) = mpsc::channel();
    thread::spawn(move || {
        let status = child.wait();
        let _ = status_tx.send(status);
    });

    let status = match status_rx.recv_timeout(timeout) {
        Ok(status) => status,
        Err(_) => {
            let _ = killer.kill();
            return Err(Error::Timeout);
        }
    };
    let output = read_handle.join().unwrap_or_default();
    let output_str = String::from_utf8_lossy(&output).to_string();

    match status {
        Ok(exit) => {
            if !exit.success() {
                return Err(Error::Render(format!("command exited with {exit}")));
            }
        }
        Err(err) => return Err(Error::Render(format!("command wait: {err}"))),
    }

    if output_str.is_empty() {
        return Err(Error::InvalidInput("no command output".to_string()));
    }

    Ok(output_str)
}

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
struct Span {
    text: String,
    style: TextStyle,
}

#[derive(Debug, Clone, Default)]
struct Line {
    spans: Vec<Span>,
}

fn highlight_code(
    text: &str,
    path: Option<&Path>,
    language: Option<&str>,
    theme_name: &str,
) -> Result<(Vec<Line>, String)> {
    static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
    static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

    let ps = &*SYNTAX_SET;
    let ts = &*THEME_SET;

    let mut theme = ts
        .themes
        .get(theme_name)
        .cloned()
        .or_else(|| {
            if theme_name.eq_ignore_ascii_case("charm") {
                Some(charm_theme())
            } else {
                None
            }
        })
        .or_else(|| ts.themes.get("base16-ocean.dark").cloned())
        .or_else(|| ts.themes.values().next().cloned())
        .ok_or_else(|| Error::Render("no themes available".to_string()))?;

    if theme_name.eq_ignore_ascii_case("charm") {
        theme = charm_theme();
    }

    let syntax = match language {
        Some(lang) => ps
            .find_syntax_by_token(lang)
            .or_else(|| ps.find_syntax_by_extension(lang))
            .unwrap_or_else(|| ps.find_syntax_plain_text()),
        None => {
            if let Some(path) = path {
                ps.find_syntax_for_file(path)
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| ps.find_syntax_plain_text())
            } else {
                let first_line = text.lines().next().unwrap_or("");
                ps.find_syntax_by_first_line(first_line)
                    .unwrap_or_else(|| ps.find_syntax_plain_text())
            }
        }
    };

    let default_fg = theme.settings.foreground.unwrap_or(Color::WHITE);
    let default_fg_hex = color_to_hex(default_fg);

    let mut highlighter = HighlightLines::new(syntax, &theme);
    let mut lines = Vec::new();

    let raw_lines: Vec<&str> = text.split('\n').collect();
    for (idx, raw) in raw_lines.iter().enumerate() {
        let mut line_with_end = raw.to_string();
        if idx + 1 < raw_lines.len() {
            line_with_end.push('\n');
        }

        let regions = highlighter
            .highlight_line(&line_with_end, ps)
            .map_err(|err| Error::Render(format!("highlight: {err}")))?;
        let mut line = Line::default();

        for (style, piece) in regions {
            let mut text_piece = piece.to_string();
            if text_piece.ends_with('\n') {
                text_piece.pop();
                if text_piece.ends_with('\r') {
                    text_piece.pop();
                }
            }
            if text_piece.is_empty() {
                continue;
            }

            let mut span_style = TextStyle::default();
            if style.foreground.a > 0 {
                span_style.fg = Some(color_to_hex(style.foreground));
            }
            if style.background.a > 0 {
                span_style.bg = Some(color_to_hex(style.background));
            }
            if style.font_style.contains(FontStyle::BOLD) {
                span_style.bold = true;
            }
            if style.font_style.contains(FontStyle::ITALIC) {
                span_style.italic = true;
            }
            if style.font_style.contains(FontStyle::UNDERLINE) {
                span_style.underline = true;
            }

            push_span(&mut line.spans, text_piece, span_style);
        }
        lines.push(line);
    }

    Ok((lines, default_fg_hex))
}

fn parse_ansi(text: &str) -> Vec<Line> {
    let mut parser = vte::Parser::new();
    let mut performer = AnsiPerformer::new();
    parser.advance(&mut performer, text.as_bytes());
    performer.into_lines()
}

struct AnsiPerformer {
    lines: Vec<Line>,
    style: TextStyle,
    col: usize,
}

impl AnsiPerformer {
    fn new() -> Self {
        Self {
            lines: vec![Line::default()],
            style: TextStyle::default(),
            col: 0,
        }
    }

    fn current_line_mut(&mut self) -> &mut Line {
        if self.lines.is_empty() {
            self.lines.push(Line::default());
        }
        self.lines.last_mut().unwrap()
    }

    fn push_char(&mut self, ch: char) {
        let width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        let style = self.style.clone();
        let line = self.current_line_mut();
        if let Some(last) = line.spans.last_mut() {
            if last.style == style {
                last.text.push(ch);
            } else {
                line.spans.push(Span {
                    text: ch.to_string(),
                    style,
                });
            }
        } else {
            line.spans.push(Span {
                text: ch.to_string(),
                style,
            });
        }
        self.col += width;
    }

    fn new_line(&mut self) {
        self.lines.push(Line::default());
        self.col = 0;
    }

    fn expand_tab(&mut self) {
        let mut count = 0;
        while !(self.col + count).is_multiple_of(ANSI_TAB_WIDTH) {
            count += 1;
        }
        if count == 0 {
            count = ANSI_TAB_WIDTH;
        }
        for _ in 0..count {
            self.push_char(' ');
        }
    }

    fn reset_style(&mut self) {
        self.style = TextStyle::default();
    }
}

impl vte::Perform for AnsiPerformer {
    fn print(&mut self, c: char) {
        self.push_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\r' => {
                self.col = 0;
            }
            b'\t' => self.expand_tab(),
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        if action != 'm' {
            return;
        }

        let mut values = params_to_vec(params);
        if values.is_empty() {
            values.push(0);
        }

        let mut i = 0;
        while i < values.len() {
            match values[i] {
                0 => self.reset_style(),
                1 => self.style.bold = true,
                3 => self.style.italic = true,
                4 => self.style.underline = true,
                9 => self.style.strike = true,
                22 => self.style.bold = false,
                23 => self.style.italic = false,
                24 => self.style.underline = false,
                29 => self.style.strike = false,
                30..=37 => self.style.fg = Some(ansi_color(values[i] as u8)),
                39 => self.style.fg = None,
                40..=47 => self.style.bg = Some(ansi_color((values[i] - 10) as u8)),
                49 => self.style.bg = None,
                90..=97 => self.style.fg = Some(ansi_color((values[i] - 60) as u8)),
                100..=107 => self.style.bg = Some(ansi_color((values[i] - 90) as u8)),
                38 => {
                    if let Some((color, consumed)) = parse_extended_color(&values[i + 1..]) {
                        self.style.fg = Some(color);
                        i += consumed;
                    }
                }
                48 => {
                    if let Some((color, consumed)) = parse_extended_color(&values[i + 1..]) {
                        self.style.bg = Some(color);
                        i += consumed;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
}

impl AnsiPerformer {
    fn into_lines(mut self) -> Vec<Line> {
        if self.lines.is_empty() {
            self.lines.push(Line::default());
        }
        self.lines
    }
}

fn params_to_vec(params: &vte::Params) -> Vec<u16> {
    let mut values = Vec::new();
    for p in params.iter() {
        if p.is_empty() {
            values.push(0);
        } else {
            values.push(p[0]);
        }
    }
    values
}

fn parse_extended_color(values: &[u16]) -> Option<(String, usize)> {
    if values.is_empty() {
        return None;
    }
    match values[0] {
        5 => {
            if values.len() >= 2 {
                Some((xterm_color(values[1] as u8), 2))
            } else {
                None
            }
        }
        2 => {
            if values.len() >= 4 {
                let r = values[1] as u8;
                let g = values[2] as u8;
                let b = values[3] as u8;
                Some((format!("#{r:02X}{g:02X}{b:02X}"), 4))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn ansi_color(code: u8) -> String {
    let palette = [
        "#282a2e", "#D74E6F", "#31BB71", "#D3E561", "#8056FF", "#ED61D7", "#04D7D7", "#C5C8C6",
        "#4B4B4B", "#FE5F86", "#00D787", "#EBFF71", "#8F69FF", "#FF7AEA", "#00FEFE", "#FFFFFF",
    ];
    let idx = match code {
        30..=37 => (code - 30) as usize,
        40..=47 => (code - 40) as usize,
        90..=97 => (code - 90 + 8) as usize,
        100..=107 => (code - 100 + 8) as usize,
        _ => code as usize,
    };
    if idx < palette.len() {
        palette[idx].to_string()
    } else {
        "#C5C8C6".to_string()
    }
}

fn xterm_color(idx: u8) -> String {
    if idx < 16 {
        return ansi_color(idx);
    }
    if idx >= 232 {
        let v = 8 + (idx - 232) * 10;
        return format!("#{v:02X}{v:02X}{v:02X}");
    }
    let idx = idx - 16;
    let r = idx / 36;
    let g = (idx % 36) / 6;
    let b = idx % 6;
    let to_comp = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
    let rr = to_comp(r);
    let gg = to_comp(g);
    let bb = to_comp(b);
    format!("#{rr:02X}{gg:02X}{bb:02X}")
}

fn color_to_hex(color: syntect::highlighting::Color) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
}

fn deserialize_box<'de, D>(deserializer: D) -> std::result::Result<Vec<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    parse_box_value(&value).map_err(serde::de::Error::custom)
}

fn parse_box_value(value: &serde_json::Value) -> std::result::Result<Vec<f32>, String> {
    match value {
        serde_json::Value::Number(n) => n
            .as_f64()
            .map(|v| vec![v as f32])
            .ok_or_else(|| "invalid number".to_string()),
        serde_json::Value::String(s) => parse_box_string(s),
        serde_json::Value::Array(arr) => {
            let mut out = Vec::new();
            for item in arr {
                match item {
                    serde_json::Value::Number(n) => {
                        out.push(n.as_f64().ok_or_else(|| "invalid number".to_string())? as f32);
                    }
                    serde_json::Value::String(s) => {
                        let parsed = parse_box_string(s)?;
                        out.extend(parsed);
                    }
                    _ => return Err("invalid array value".to_string()),
                }
            }
            if matches!(out.len(), 1 | 2 | 4) {
                Ok(out)
            } else {
                Err(format!("expected 1, 2, or 4 values, got {}", out.len()))
            }
        }
        serde_json::Value::Null => Ok(vec![0.0]),
        _ => Err("invalid box value".to_string()),
    }
}

fn parse_box_string(input: &str) -> std::result::Result<Vec<f32>, String> {
    let parts: Vec<&str> = input.split([',', ' ']).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Ok(vec![0.0]);
    }
    let mut out = Vec::new();
    for part in parts {
        let value = part
            .parse::<f32>()
            .map_err(|_| format!("invalid number {}", part))?;
        out.push(value);
    }
    if matches!(out.len(), 1 | 2 | 4) {
        Ok(out)
    } else {
        Err(format!("expected 1, 2, or 4 values, got {}", out.len()))
    }
}

fn deserialize_lines<'de, D>(deserializer: D) -> std::result::Result<Vec<i32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    parse_lines_value(&value).map_err(serde::de::Error::custom)
}

fn parse_lines_value(value: &serde_json::Value) -> std::result::Result<Vec<i32>, String> {
    match value {
        serde_json::Value::Number(n) => n
            .as_i64()
            .map(|v| vec![v as i32])
            .ok_or_else(|| "invalid number".to_string()),
        serde_json::Value::String(s) => parse_lines_string(s),
        serde_json::Value::Array(arr) => {
            let mut out = Vec::new();
            for item in arr {
                match item {
                    serde_json::Value::Number(n) => {
                        out.push(n.as_i64().ok_or_else(|| "invalid number".to_string())? as i32);
                    }
                    serde_json::Value::String(s) => {
                        let parsed = parse_lines_string(s)?;
                        out.extend(parsed);
                    }
                    _ => return Err("invalid array value".to_string()),
                }
            }
            if matches!(out.len(), 1 | 2) {
                Ok(out)
            } else {
                Err(format!("expected 1 or 2 values, got {}", out.len()))
            }
        }
        serde_json::Value::Null => Ok(vec![]),
        _ => Err("invalid lines value".to_string()),
    }
}

fn parse_lines_string(input: &str) -> std::result::Result<Vec<i32>, String> {
    let parts: Vec<&str> = input.split([',', ' ']).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for part in parts {
        let value = part
            .parse::<i32>()
            .map_err(|_| format!("invalid number {}", part))?;
        out.push(value);
    }
    if matches!(out.len(), 1 | 2) {
        Ok(out)
    } else {
        Err(format!("expected 1 or 2 values, got {}", out.len()))
    }
}

fn charm_theme() -> syntect::highlighting::Theme {
    use std::str::FromStr;
    use syntect::highlighting::{
        Color, FontStyle, ScopeSelectors, Theme, ThemeItem, ThemeSettings,
    };

    let mut theme = Theme {
        name: Some("charm".to_string()),
        author: Some("cryosnap".to_string()),
        settings: ThemeSettings {
            foreground: Some(Color {
                r: 0xC4,
                g: 0xC4,
                b: 0xC4,
                a: 0xFF,
            }),
            background: Some(Color {
                r: 0x17,
                g: 0x17,
                b: 0x17,
                a: 0xFF,
            }),
            ..ThemeSettings::default()
        },
        scopes: Vec::new(),
    };

    let mut push = |scope: &str, fg: &str, style: FontStyle| {
        let scope = ScopeSelectors::from_str(scope)
            .unwrap_or_else(|_| ScopeSelectors::from_str("text").unwrap());
        let color = hex_to_color(fg);
        theme.scopes.push(ThemeItem {
            scope,
            style: syntect::highlighting::StyleModifier {
                foreground: Some(color),
                background: None,
                font_style: Some(style),
            },
        });
    };

    push("comment", "#676767", FontStyle::empty());
    push("comment.preproc", "#FF875F", FontStyle::empty());
    push("keyword", "#00AAFF", FontStyle::empty());
    push("keyword.reserved", "#FF48DD", FontStyle::empty());
    push("keyword.namespace", "#FF5F87", FontStyle::empty());
    push("storage.type", "#635ADF", FontStyle::empty());
    push("operator", "#FF7F83", FontStyle::empty());
    push("punctuation", "#E8E8A8", FontStyle::empty());
    push("constant.numeric", "#6EEFC0", FontStyle::empty());
    push("string", "#E38356", FontStyle::empty());
    push("string.escape", "#AFFFD7", FontStyle::empty());
    push("entity.name.function", "#00DC7F", FontStyle::empty());
    push("entity.name.tag", "#B083EA", FontStyle::empty());
    push("entity.name.attribute", "#7A7AE6", FontStyle::empty());
    push(
        "entity.name.class",
        "#F1F1F1",
        FontStyle::BOLD | FontStyle::UNDERLINE,
    );
    push("entity.name.decorator", "#FFFF87", FontStyle::empty());

    theme
}

fn hex_to_color(hex: &str) -> syntect::highlighting::Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return syntect::highlighting::Color::WHITE;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    syntect::highlighting::Color { r, g, b, a: 0xFF }
}

struct CutResult {
    text: String,
    start: usize,
}

fn cut_text(text: &str, window: &[i32]) -> CutResult {
    if window.is_empty() {
        return CutResult {
            text: text.to_string(),
            start: 0,
        };
    }
    if window.len() == 1 && window[0] == 0 {
        return CutResult {
            text: text.to_string(),
            start: 0,
        };
    }
    if window.len() == 2 && window[0] == 0 && window[1] == -1 {
        return CutResult {
            text: text.to_string(),
            start: 0,
        };
    }

    let lines: Vec<&str> = text.split('\n').collect();
    let total = lines.len() as i32;
    let mut start;
    let mut end = total;

    match window.len() {
        1 => {
            if window[0] > 0 {
                start = window[0];
            } else {
                start = total + window[0];
            }
        }
        _ => {
            start = window[0];
            end = window[1];
        }
    }

    if start < 0 {
        start = 0;
    }
    if start > total {
        start = total;
    }
    end += 1;
    if end < start {
        end = start;
    }
    if end > total {
        end = total;
    }

    let start_usize = start as usize;
    let end_usize = end as usize;
    if start_usize >= lines.len() {
        return CutResult {
            text: String::new(),
            start: start_usize,
        };
    }
    CutResult {
        text: lines[start_usize..end_usize].join("\n"),
        start: start_usize,
    }
}

fn detab(text: &str, tab_width: usize) -> String {
    let mut out = String::new();
    let mut col = 0usize;
    for ch in text.chars() {
        if ch == '\t' {
            let mut count = 0;
            while !(col + count).is_multiple_of(tab_width) {
                count += 1;
            }
            if count == 0 {
                count = tab_width;
            }
            for _ in 0..count {
                out.push(' ');
            }
            col += count;
        } else {
            if ch == '\n' {
                col = 0;
            } else {
                col += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            }
            out.push(ch);
        }
    }
    out
}

fn wrap_text(text: &str, width: usize) -> String {
    if width == 0 {
        return text.to_string();
    }
    let mut out_lines = Vec::new();
    for line in text.split('\n') {
        let mut current = String::new();
        let mut current_width = 0usize;
        for ch in line.chars() {
            let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if current_width + w > width && !current.is_empty() {
                out_lines.push(current);
                current = String::new();
                current_width = 0;
            }
            current.push(ch);
            current_width += w;
            if current_width >= width {
                out_lines.push(current);
                current = String::new();
                current_width = 0;
            }
        }
        out_lines.push(current);
    }
    out_lines.join("\n")
}

fn wrap_ansi_lines(lines: &[Line], width: usize) -> Vec<Line> {
    if width == 0 {
        return lines.to_vec();
    }
    let mut out = Vec::new();
    for line in lines {
        out.extend(split_line_by_width(line, width));
    }
    out
}

fn split_line_by_width(line: &Line, width: usize) -> Vec<Line> {
    if width == 0 {
        return vec![line.clone()];
    }
    let mut out = Vec::new();
    let mut current = Line::default();
    let mut current_width = 0usize;

    for span in &line.spans {
        let mut buf = String::new();
        for ch in span.text.chars() {
            let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if current_width + w > width && !current.spans.is_empty() {
                if !buf.is_empty() {
                    current.spans.push(Span {
                        text: buf.clone(),
                        style: span.style.clone(),
                    });
                    buf.clear();
                }
                out.push(current);
                current = Line::default();
                current_width = 0;
            }
            buf.push(ch);
            current_width += w;
            if current_width >= width {
                current.spans.push(Span {
                    text: buf.clone(),
                    style: span.style.clone(),
                });
                buf.clear();
                out.push(current);
                current = Line::default();
                current_width = 0;
            }
        }
        if !buf.is_empty() {
            current.spans.push(Span {
                text: buf.clone(),
                style: span.style.clone(),
            });
        }
    }

    if !current.spans.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        out.push(Line::default());
    }
    out
}

fn push_span(spans: &mut Vec<Span>, text: String, style: TextStyle) {
    if let Some(last) = spans.last_mut() {
        if last.style == style {
            last.text.push_str(&text);
            return;
        }
    }
    spans.push(Span { text, style });
}

fn build_svg(
    lines: &[Line],
    config: &Config,
    default_fg: &str,
    font_css: Option<String>,
    line_offset: usize,
    title_text: Option<&str>,
) -> String {
    let padding = expand_box(&config.padding);
    let margin = expand_box(&config.margin);
    let mut pad_top = padding[0];
    let pad_right = padding[1];
    let pad_bottom = padding[2];
    let pad_left = padding[3];
    let margin_top = margin[0];
    let margin_right = margin[1];
    let margin_bottom = margin[2];
    let margin_left = margin[3];

    if config.window_controls {
        pad_top += WINDOW_CONTROLS_HEIGHT;
    }

    let line_height_px = config.font.size * config.line_height;
    let char_width = config.font.size / FONT_HEIGHT_TO_WIDTH_RATIO;
    let line_count = std::cmp::max(1, lines.len());

    let line_number_cells = if config.show_line_numbers {
        let digits = std::cmp::max(3, line_count.to_string().len());
        digits + 2
    } else {
        0
    };

    let mut max_cells = 0usize;
    for line in lines {
        let width = line_width_cells(line);
        max_cells = max_cells.max(width);
    }
    max_cells += line_number_cells;

    let content_width = max_cells as f32 * char_width;
    let content_height = line_count as f32 * line_height_px;

    let mut terminal_width = content_width + pad_left + pad_right;
    let mut terminal_height = content_height + pad_top + pad_bottom;
    let mut image_width = terminal_width + margin_left + margin_right;
    let mut image_height = terminal_height + margin_top + margin_bottom;

    if config.width > 0.0 {
        image_width = config.width;
        terminal_width = (image_width - margin_left - margin_right).max(0.0);
    }
    if config.height > 0.0 {
        image_height = config.height;
        terminal_height = (image_height - margin_top - margin_bottom).max(0.0);
    }

    let content_width = (terminal_width - pad_left - pad_right).max(0.0);
    let content_height = (terminal_height - pad_top - pad_bottom).max(0.0);

    let max_visible_lines = if config.height > 0.0 {
        let lines_fit = (content_height / line_height_px).floor() as usize;
        std::cmp::max(1, lines_fit)
    } else {
        line_count
    };

    let mut svg = String::new();
    svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{:.2}" height="{:.2}">"#,
        image_width, image_height
    ));
    if let Some(font_css) = font_css {
        svg.push_str(r#"<defs><style type="text/css">"#);
        svg.push_str(&font_css);
        svg.push_str("</style></defs>");
    }

    if config.shadow.blur > 0.0 || config.shadow.x != 0.0 || config.shadow.y != 0.0 {
        svg.push_str(r#"<defs><filter id="shadow" filterUnits="userSpaceOnUse">"#);
        svg.push_str(&format!(
            r#"<feGaussianBlur in="SourceAlpha" stdDeviation="{:.2}"/>"#,
            config.shadow.blur
        ));
        svg.push_str(&format!(
            r#"<feOffset dx="{:.2}" dy="{:.2}" result="offsetblur"/>"#,
            config.shadow.x, config.shadow.y
        ));
        svg.push_str(r#"<feMerge><feMergeNode/><feMergeNode in="SourceGraphic"/></feMerge>"#);
        svg.push_str("</filter></defs>");
    }

    let terminal_x = margin_left;
    let terminal_y = margin_top;
    let mut terminal_attrs = String::new();
    if config.border.radius > 0.0 {
        terminal_attrs.push_str(&format!(
            r#" rx="{:.2}" ry="{:.2}""#,
            config.border.radius, config.border.radius
        ));
    }
    if config.border.width > 0.0 {
        terminal_attrs.push_str(&format!(
            r#" stroke="{}" stroke-width="{:.2}""#,
            config.border.color, config.border.width
        ));
    }
    if config.shadow.blur > 0.0 || config.shadow.x != 0.0 || config.shadow.y != 0.0 {
        terminal_attrs.push_str(r#" filter="url(#shadow)""#);
    }

    let border_inset = config.border.width / 2.0;
    svg.push_str(&format!(
        r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"{} />"#,
        terminal_x + border_inset,
        terminal_y + border_inset,
        (terminal_width - config.border.width).max(0.0),
        (terminal_height - config.border.width).max(0.0),
        config.background,
        terminal_attrs
    ));

    svg.push_str(&format!(
        r#"<defs><clipPath id="contentClip"><rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}"/></clipPath></defs>"#,
        terminal_x + pad_left,
        terminal_y + pad_top - config.font.size,
        content_width,
        (content_height + config.font.size).max(0.0)
    ));

    if config.window_controls {
        let r = 5.5;
        let x = terminal_x + border_inset + WINDOW_CONTROLS_X_OFFSET;
        let y = terminal_y + WINDOW_CONTROLS_X_OFFSET;
        svg.push_str(&format!(
            r##"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="#FF5A54"/>"##,
            x, y, r
        ));
        svg.push_str(&format!(
            r##"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="#E6BF29"/>"##,
            x + WINDOW_CONTROLS_SPACING,
            y,
            r
        ));
        svg.push_str(&format!(
            r##"<circle cx="{:.2}" cy="{:.2}" r="{:.2}" fill="#52C12B"/>"##,
            x + WINDOW_CONTROLS_SPACING * 2.0,
            y,
            r
        ));

        if let Some(title_text) = title_text {
            let title = sanitize_title_text(title_text);
            if !title.is_empty() {
                let title_size = if config.title.size > 0.0 {
                    config.title.size
                } else {
                    (config.font.size - 2.0).max(8.0)
                };
                let char_width = title_size / FONT_HEIGHT_TO_WIDTH_RATIO;
                let controls_right = x + WINDOW_CONTROLS_SPACING * 2.0 + r;
                let title_margin = WINDOW_CONTROLS_X_OFFSET;
                let left_reserved = (controls_right - terminal_x) + title_margin;
                let right_reserved = title_margin;
                let available_px = match config.title.align {
                    TitleAlign::Center => terminal_width - 2.0 * left_reserved,
                    _ => terminal_width - left_reserved - right_reserved,
                };

                if available_px > 0.0 {
                    let max_cells_from_width =
                        (available_px / char_width).floor().max(0.0) as usize;
                    let mut max_cells = max_cells_from_width;
                    if config.title.max_width > 0 {
                        max_cells = max_cells.min(config.title.max_width);
                    }
                    let truncated = truncate_to_cells(&title, max_cells, &config.title.ellipsis);
                    if !truncated.is_empty() {
                        let (title_x, anchor) = match config.title.align {
                            TitleAlign::Left => (terminal_x + left_reserved, "start"),
                            TitleAlign::Center => (terminal_x + terminal_width / 2.0, "middle"),
                            TitleAlign::Right => {
                                (terminal_x + terminal_width - right_reserved, "end")
                            }
                        };
                        let title_y = terminal_y + WINDOW_CONTROLS_X_OFFSET + (title_size * 0.35);
                        let opacity = config.title.opacity.clamp(0.0, 1.0);
                        let opacity_attr = if opacity < 1.0 {
                            format!(r#" fill-opacity="{:.2}""#, opacity)
                        } else {
                            String::new()
                        };
                        svg.push_str(&format!(
                            r#"<text x="{:.2}" y="{:.2}" fill="{}" font-family="{}" font-size="{:.2}px" text-anchor="{}"{}>{}</text>"#,
                            title_x,
                            title_y,
                            escape_attr(&config.title.color),
                            escape_attr(&config.font.family),
                            title_size,
                            anchor,
                            opacity_attr,
                            escape_text(&truncated)
                        ));
                    }
                }
            }
        }
    }

    svg.push_str(&format!(
        r#"<g font-family="{}" font-size="{:.2}px" clip-path="url(#contentClip)">"#,
        escape_attr(&config.font.family),
        config.font.size
    ));
    let mut bg_layer = String::new();
    let mut text_layer = String::new();

    let line_number_width_px = line_number_cells as f32 * char_width;
    for (idx, line) in lines.iter().take(max_visible_lines).enumerate() {
        let line_idx = idx as f32;
        let y = terminal_y + pad_top + line_height_px * (line_idx + 1.0);
        let base_x = terminal_x + pad_left;

        if config.show_line_numbers {
            let number_text = format!(
                "{:>width$}  ",
                idx + 1 + line_offset,
                width = line_number_cells - 2
            );
            text_layer.push_str(&format!(
                r##"<text x="{:.2}" y="{:.2}" fill="#777777" xml:space="preserve">{}</text>"##,
                base_x,
                y,
                escape_text(&number_text)
            ));
        }

        let text_x = base_x + line_number_width_px;
        text_layer.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" fill="{}" xml:space="preserve">"#,
            text_x, y, default_fg
        ));

        let mut cursor_x = text_x;
        for span in &line.spans {
            let text = &span.text;
            let width_px = span_width_px(text, char_width);
            if let Some(bg) = &span.style.bg {
                let rect_y = y - config.font.size;
                bg_layer.push_str(&format!(
                    r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="{}"/>"#,
                    cursor_x, rect_y, width_px, line_height_px, bg
                ));
            }

            let mut attrs = String::new();
            if let Some(fg) = &span.style.fg {
                attrs.push_str(&format!(r#" fill="{}""#, fg));
            }
            if span.style.bold {
                attrs.push_str(r#" font-weight="bold""#);
            }
            if span.style.italic {
                attrs.push_str(r#" font-style="italic""#);
            }
            if span.style.underline || span.style.strike {
                let mut deco = Vec::new();
                if span.style.underline {
                    deco.push("underline");
                }
                if span.style.strike {
                    deco.push("line-through");
                }
                attrs.push_str(&format!(r#" text-decoration="{}""#, deco.join(" ")));
            }

            text_layer.push_str(&format!(
                r#"<tspan xml:space="preserve"{}>{}</tspan>"#,
                attrs,
                escape_text(text)
            ));
            cursor_x += width_px;
        }
        text_layer.push_str("</text>");
    }

    svg.push_str(&bg_layer);
    svg.push_str(&text_layer);
    svg.push_str("</g></svg>");
    svg
}

fn expand_box(values: &[f32]) -> [f32; 4] {
    match values.len() {
        1 => [values[0], values[0], values[0], values[0]],
        2 => [values[0], values[1], values[0], values[1]],
        4 => [values[0], values[1], values[2], values[3]],
        _ => [0.0, 0.0, 0.0, 0.0],
    }
}

fn line_width_cells(line: &Line) -> usize {
    let mut width = 0usize;
    for span in &line.spans {
        for ch in span.text.chars() {
            if ch == '\t' {
                let mut count = 0;
                while !(width + count).is_multiple_of(DEFAULT_TAB_WIDTH) {
                    count += 1;
                }
                if count == 0 {
                    count = DEFAULT_TAB_WIDTH;
                }
                width += count;
            } else {
                width += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            }
        }
    }
    width
}

fn span_width_px(text: &str, char_width: f32) -> f32 {
    let mut width = 0usize;
    for ch in text.chars() {
        if ch == '\t' {
            let mut count = 0;
            while !(width + count).is_multiple_of(DEFAULT_TAB_WIDTH) {
                count += 1;
            }
            if count == 0 {
                count = DEFAULT_TAB_WIDTH;
            }
            width += count;
        } else {
            width += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        }
    }
    width as f32 * char_width
}

fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(text: &str) -> String {
    escape_text(text).replace('"', "&quot;")
}

fn svg_font_face_css(config: &Config) -> Result<Option<String>> {
    let (data, format, mime) = if let Some(font_file) = &config.font.file {
        let bytes = std::fs::read(font_file)?;
        let ext = Path::new(font_file)
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let (format, mime) = match ext.as_str() {
            "ttf" => ("truetype", "font/ttf"),
            "woff2" => ("woff2", "font/woff2"),
            "woff" => ("woff", "font/woff"),
            _ => ("truetype", "font/ttf"),
        };
        (bytes, format.to_string(), mime.to_string())
    } else if is_jetbrains_mono(&config.font.family) {
        if config.font.ligatures {
            (
                JETBRAINS_MONO_REGULAR.to_vec(),
                "truetype".to_string(),
                "font/ttf".to_string(),
            )
        } else {
            (
                JETBRAINS_MONO_NL.to_vec(),
                "truetype".to_string(),
                "font/ttf".to_string(),
            )
        }
    } else if is_hack_nerd_font(&config.font.family) {
        (
            HACK_NERD_FONT_REGULAR.to_vec(),
            "truetype".to_string(),
            "font/ttf".to_string(),
        )
    } else {
        return Ok(None);
    };

    let encoded = base64::engine::general_purpose::STANDARD.encode(data);
    Ok(Some(format!(
        "@font-face {{ font-family: '{}'; src: url(data:{};base64,{}) format('{}'); }}",
        escape_attr(&config.font.family),
        mime,
        encoded,
        format
    )))
}

static JETBRAINS_MONO_REGULAR: &[u8] = include_bytes!("../assets/JetBrainsMono-Regular.ttf");
static JETBRAINS_MONO_NL: &[u8] = include_bytes!("../assets/JetBrainsMonoNL-Regular.ttf");
static HACK_NERD_FONT_REGULAR: &[u8] = include_bytes!("../assets/HackNerdFont-Regular.ttf");

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    #[test]
    fn deserialize_box_values() {
        let cfg: Config = serde_json::from_str(r#"{"padding":"10,20","margin":[1,2,3,4]}"#)
            .expect("parse config");
        assert_eq!(cfg.padding, vec![10.0, 20.0]);
        assert_eq!(cfg.margin, vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn deserialize_lines_values() {
        let cfg: Config = serde_json::from_str(r#"{"lines":"2,4"}"#).expect("parse config");
        assert_eq!(cfg.lines, vec![2, 4]);
    }

    #[test]
    fn cut_text_window() {
        let input = "a\nb\nc\nd";
        let result = cut_text(input, &[1, 2]);
        assert_eq!(result.text, "b\nc");
        assert_eq!(result.start, 1);
    }

    #[test]
    fn detab_expands() {
        let input = "a\tb";
        let out = detab(input, 4);
        assert_eq!(out, "a   b");
    }

    #[test]
    fn wrap_text_basic() {
        let input = "abcd";
        let out = wrap_text(input, 3);
        assert_eq!(out, "abc\nd");
    }

    #[test]
    fn parse_ansi_colors() {
        let input = "A\x1b[31mB\x1b[0mC";
        let lines = parse_ansi(input);
        assert_eq!(lines.len(), 1);
        let spans = &lines[0].spans;
        assert!(spans
            .iter()
            .any(|s| s.text == "B" && s.style.fg == Some("#D74E6F".to_string())));
    }

    #[test]
    fn wrap_ansi_lines_basic() {
        let line = Line {
            spans: vec![Span {
                text: "abcdef".to_string(),
                style: TextStyle::default(),
            }],
        };
        let out = wrap_ansi_lines(&[line], 3);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].spans[0].text, "abc");
        assert_eq!(out[1].spans[0].text, "def");
    }

    #[test]
    fn build_svg_includes_border_shadow() {
        let line = Line {
            spans: vec![Span {
                text: "hi".to_string(),
                style: TextStyle::default(),
            }],
        };
        let mut cfg = Config::default();
        cfg.font.family = "Test".to_string();
        cfg.border.radius = 4.0;
        cfg.border.width = 1.0;
        cfg.shadow.blur = 6.0;
        cfg.window_controls = true;
        cfg.show_line_numbers = true;
        let svg = build_svg(&[line], &cfg, "#FFFFFF", None, 0, None);
        assert!(svg.contains("filter id=\"shadow\""));
        assert!(svg.contains("clipPath"));
        assert!(svg.contains("font-family=\"Test\""));
        assert!(svg.contains("circle"));
    }

    #[test]
    fn render_svg_basic() {
        let mut cfg = Config::default();
        cfg.font.family = "Test".to_string();
        let request = RenderRequest {
            input: InputSource::Text("fn main() {}".to_string()),
            config: cfg,
            format: OutputFormat::Svg,
        };
        let result = render(&request).expect("render svg");
        let svg = String::from_utf8(result.bytes).expect("utf8");
        assert!(svg.contains("<svg"));
    }

    #[test]
    fn svg_font_face_css_respects_family() {
        let mut cfg = Config::default();
        cfg.font.family = "JetBrains Mono".to_string();
        let css = svg_font_face_css(&cfg).expect("css");
        assert!(css.is_some());

        cfg.font.family = "Custom".to_string();
        let css = svg_font_face_css(&cfg).expect("css");
        assert!(css.is_none());
    }

    #[test]
    fn svg_font_face_css_supports_hack_nerd_font() {
        let mut cfg = Config::default();
        cfg.font.family = "Hack Nerd Font".to_string();
        let css = svg_font_face_css(&cfg).expect("css");
        assert!(css.is_some());
    }

    #[test]
    fn title_truncates_with_ellipsis() {
        let out = truncate_to_cells("abcdef", 4, "…");
        assert_eq!(out, "abc…");
    }

    #[test]
    fn title_uses_absolute_path() {
        let mut cfg = Config {
            window_controls: true,
            ..Config::default()
        };
        cfg.title.enabled = true;
        cfg.title.path_style = TitlePathStyle::Absolute;
        let path = std::env::temp_dir().join(format!("cryosnap-title-{}.txt", std::process::id()));
        std::fs::write(&path, "hi").expect("write temp");
        let input = InputSource::File(path.clone());
        let title = resolve_title_text(&input, &cfg).expect("title");
        assert!(Path::new(&title).is_absolute());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn resolve_title_text_disabled_returns_none() {
        let cfg = Config {
            window_controls: true,
            title: TitleOptions {
                enabled: false,
                ..TitleOptions::default()
            },
            ..Config::default()
        };
        let input = InputSource::Text("hi".to_string());
        assert!(resolve_title_text(&input, &cfg).is_none());
    }

    #[test]
    fn resolve_title_text_window_controls_off_returns_none() {
        let cfg = Config {
            window_controls: false,
            title: TitleOptions {
                enabled: true,
                text: Some("Title".to_string()),
                ..TitleOptions::default()
            },
            ..Config::default()
        };
        let input = InputSource::Text("hi".to_string());
        assert!(resolve_title_text(&input, &cfg).is_none());
    }

    #[test]
    fn resolve_title_text_from_command() {
        let cfg = Config {
            window_controls: true,
            ..Config::default()
        };
        let input = InputSource::Command("echo hi".to_string());
        let title = resolve_title_text(&input, &cfg).expect("title");
        assert!(title.contains("cmd: echo hi"));
    }

    #[test]
    fn title_text_from_path_basename_and_relative() {
        let _lock = cwd_lock().lock().expect("lock");
        let root = std::env::temp_dir().join(format!("cryosnap-title-test-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let path = root.join("sample.txt");
        std::fs::write(&path, "hi").expect("write temp");

        let basename = title_text_from_path(&path, TitlePathStyle::Basename);
        assert_eq!(basename, "sample.txt");

        let cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(&root).expect("chdir");
        let resolved = std::env::current_dir().expect("cwd").join("sample.txt");
        let relative = title_text_from_path(&resolved, TitlePathStyle::Relative);
        assert_eq!(relative, "sample.txt");
        std::env::set_current_dir(cwd).expect("restore");

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn sanitize_title_text_removes_newlines() {
        let out = sanitize_title_text("hello\nworld\rtest");
        assert_eq!(out, "hello world test");
    }

    #[test]
    fn truncate_to_cells_edge_cases() {
        assert_eq!(truncate_to_cells("abcdef", 0, "…"), "");
        assert_eq!(truncate_to_cells("abcdef", 1, "..."), ".");
    }

    #[test]
    fn unpremultiply_rgba_handles_zero_alpha() {
        let data = [10u8, 20, 30, 0, 100, 50, 25, 128];
        let out = unpremultiply_rgba(&data);
        assert_eq!(&out[..4], &[0, 0, 0, 0]);
        assert_eq!(out.len(), 8);
    }

    #[test]
    fn encode_indexed_png_rejects_invalid_length() {
        let palette = vec![imagequant::RGBA::new(0, 0, 0, 255)];
        let indices = vec![0, 0];
        let err = encode_indexed_png(&palette, &indices, 1, 1).unwrap_err();
        assert!(err.to_string().contains("invalid index buffer"));
    }

    #[test]
    fn render_webp_from_svg_rejects_rsvg_backend() {
        let mut cfg = Config::default();
        cfg.raster.backend = RasterBackend::Rsvg;
        let svg = br#"<svg width="10" height="10" xmlns="http://www.w3.org/2000/svg"></svg>"#;
        let err = render_webp_from_svg(svg, &cfg).unwrap_err();
        assert!(err.to_string().contains("rsvg backend"));
    }

    #[test]
    fn is_ansi_input_detects_escape() {
        let loaded = LoadedInput {
            text: "hi\x1b[31m".to_string(),
            path: None,
            kind: InputKind::Code,
        };
        let cfg = Config::default();
        assert!(is_ansi_input(&loaded, &cfg));
    }

    #[test]
    fn load_input_file_reads_text() {
        let root = std::env::temp_dir().join(format!("cryosnap-load-input-{}", std::process::id()));
        std::fs::create_dir_all(&root).expect("create dir");
        let path = root.join("input.txt");
        std::fs::write(&path, "hello").expect("write");
        let input = InputSource::File(path.clone());
        let loaded = load_input(&input, Duration::from_millis(1000)).expect("load");
        assert_eq!(loaded.text, "hello");
        assert_eq!(loaded.path, Some(path));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn execute_command_rejects_empty() {
        let err = execute_command("   ", Duration::from_millis(1000)).unwrap_err();
        assert!(err.to_string().contains("empty command"));
    }

    #[test]
    fn render_png_basic() {
        let mut cfg = Config::default();
        cfg.font.family = "Test".to_string();
        let request = RenderRequest {
            input: InputSource::Text("hi".to_string()),
            config: cfg,
            format: OutputFormat::Png,
        };
        let result = render(&request).expect("render png");
        assert!(result.bytes.starts_with(b"\x89PNG"));
    }

    #[test]
    fn render_png_optimize_disabled() {
        let mut cfg = Config::default();
        cfg.font.family = "Test".to_string();
        cfg.png.optimize = false;
        let request = RenderRequest {
            input: InputSource::Text("hi".to_string()),
            config: cfg,
            format: OutputFormat::Png,
        };
        let result = render(&request).expect("render png");
        assert!(result.bytes.starts_with(b"\x89PNG"));
    }

    #[test]
    fn render_png_quantize_basic() {
        let mut cfg = Config::default();
        cfg.font.family = "Test".to_string();
        cfg.png.quantize = true;
        cfg.png.optimize = false;
        let request = RenderRequest {
            input: InputSource::Text("hi".to_string()),
            config: cfg,
            format: OutputFormat::Png,
        };
        let result = render(&request).expect("render png");
        assert!(result.bytes.starts_with(b"\x89PNG"));
    }

    #[test]
    fn decode_png_rgba_roundtrip() {
        let rgba = [255u8, 0, 0, 255, 0, 255, 0, 128];
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("header");
            writer.write_image_data(&rgba).expect("write");
        }
        let (decoded, width, height) = decode_png_rgba(&bytes).expect("decode");
        assert_eq!(width, 2);
        assert_eq!(height, 1);
        assert_eq!(decoded, rgba);
    }

    #[test]
    fn decode_png_rgb_expands_alpha() {
        let rgb = [10u8, 20, 30, 40, 50, 60];
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
            encoder.set_color(png::ColorType::Rgb);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("header");
            writer.write_image_data(&rgb).expect("write");
        }
        let (decoded, width, height) = decode_png_rgba(&bytes).expect("decode");
        assert_eq!(width, 2);
        assert_eq!(height, 1);
        assert_eq!(decoded, vec![10u8, 20, 30, 255, 40, 50, 60, 255]);
    }

    #[test]
    fn quantize_preset_overrides_values() {
        let mut cfg = Config::default();
        cfg.png.quantize_preset = Some(PngQuantPreset::Fast);
        cfg.png.quantize_quality = 99;
        cfg.png.quantize_speed = 1;
        cfg.png.quantize_dither = 0.0;
        let settings = quantize_settings(&cfg.png);
        assert_eq!(settings.quality, 70);
        assert_eq!(settings.speed, 7);
        assert!((settings.dither - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn quantize_png_bytes_basic() {
        let rgba = [0u8, 0, 0, 255, 255, 255, 255, 255];
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("header");
            writer.write_image_data(&rgba).expect("write");
        }
        let png = quantize_png_bytes(&bytes, &PngOptions::default()).expect("quantize");
        assert!(png.starts_with(b"\x89PNG"));
    }

    #[test]
    fn render_svg_supports_nerd_font_glyph() {
        let mut cfg = Config::default();
        cfg.font.family = "Hack Nerd Font".to_string();
        let request = RenderRequest {
            input: InputSource::Text("\u{f121}".to_string()),
            config: cfg,
            format: OutputFormat::Svg,
        };
        let result = render(&request).expect("render svg");
        let svg = String::from_utf8(result.bytes).expect("utf8");
        assert!(svg.contains('\u{f121}'));
    }

    #[test]
    fn render_svg_respects_lines_offset() {
        let mut cfg = Config::default();
        cfg.font.family = "Test".to_string();
        cfg.show_line_numbers = true;
        cfg.lines = vec![1, 1];
        let request = RenderRequest {
            input: InputSource::Text("a\nb\nc".to_string()),
            config: cfg,
            format: OutputFormat::Svg,
        };
        let result = render(&request).expect("render svg");
        let svg = String::from_utf8(result.bytes).expect("utf8");
        assert!(svg.contains(">  2  </text>"));
        assert!(svg.contains(">b</tspan>"));
        assert!(!svg.contains(">a</tspan>"));
    }

    #[test]
    fn render_webp_basic() {
        let mut cfg = Config::default();
        cfg.font.family = "Test".to_string();
        let request = RenderRequest {
            input: InputSource::Text("hi".to_string()),
            config: cfg,
            format: OutputFormat::Webp,
        };
        let result = render(&request).expect("render webp");
        assert!(result.bytes.starts_with(b"RIFF"));
        assert!(result.bytes.len() > 12);
        assert_eq!(&result.bytes[8..12], b"WEBP");
    }

    #[test]
    fn raster_scale_defaults() {
        let cfg = Config::default();
        let scale = raster_scale(&cfg, 100, 100).expect("scale");
        assert_eq!(scale, DEFAULT_RASTER_SCALE);
    }

    #[test]
    fn raster_scale_with_dimensions() {
        let cfg = Config {
            width: 800.0,
            ..Config::default()
        };
        let scale = raster_scale(&cfg, 100, 100).expect("scale");
        assert_eq!(scale, 1.0);
        let cfg = Config {
            height: 600.0,
            ..Config::default()
        };
        let scale = raster_scale(&cfg, 100, 100).expect("scale");
        assert_eq!(scale, 1.0);
    }

    #[test]
    fn raster_scale_clamps_pixels() {
        let mut cfg = Config::default();
        cfg.raster.max_pixels = 1_000;
        let scale = raster_scale(&cfg, 100, 100).expect("scale");
        assert!(scale < cfg.raster.scale);
    }

    #[test]
    fn scale_dimension_rejects_zero() {
        let result = scale_dimension(1, 0.0);
        assert!(matches!(result, Err(Error::Render(_))));
    }

    #[cfg(unix)]
    #[test]
    fn execute_command_echo() {
        let output =
            execute_command("printf 'hello'", Duration::from_millis(2000)).expect("execute");
        assert!(output.contains("hello"));
    }

    #[cfg(unix)]
    #[test]
    fn execute_command_timeout() {
        let result = execute_command("sleep 2", Duration::from_millis(10));
        assert!(matches!(result, Err(Error::Timeout)));
    }

    #[cfg(unix)]
    #[test]
    fn execute_command_failure() {
        let result = execute_command("false", Duration::from_millis(2000));
        assert!(matches!(result, Err(Error::Render(_))));
    }

    #[cfg(unix)]
    #[test]
    fn execute_command_no_output() {
        let result = execute_command("printf ''", Duration::from_millis(2000));
        assert!(matches!(result, Err(Error::InvalidInput(_))));
    }

    fn cwd_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }
}
