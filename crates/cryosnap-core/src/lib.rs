use base64::Engine;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use unicode_script::{Script, UnicodeScript};

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
const AUTO_FALLBACK_NF: &[&str] = &["Symbols Nerd Font Mono"];
const AUTO_FALLBACK_CJK: &[&str] = &[
    "Noto Sans Mono CJK SC",
    "Noto Sans Mono CJK TC",
    "Noto Sans Mono CJK HK",
    "Noto Sans Mono CJK JP",
    "Noto Sans Mono CJK KR",
    "Noto Sans CJK SC",
    "Noto Sans CJK TC",
    "Noto Sans CJK HK",
    "Noto Sans CJK JP",
    "Noto Sans CJK KR",
    "Source Han Sans SC",
    "Source Han Sans TC",
    "Source Han Sans HK",
    "Source Han Sans JP",
    "Source Han Sans KR",
    "PingFang SC",
    "PingFang TC",
    "PingFang HK",
    "Hiragino Sans GB",
    "Hiragino Sans",
    "Apple SD Gothic Neo",
    "Microsoft YaHei",
    "Microsoft JhengHei",
    "SimSun",
    "MS Gothic",
    "Meiryo",
    "Yu Gothic",
    "Malgun Gothic",
    "WenQuanYi Micro Hei",
    "WenQuanYi Zen Hei",
];
const AUTO_FALLBACK_CJK_SC: &[&str] = &[
    "Noto Sans Mono CJK SC",
    "Noto Sans CJK SC",
    "Source Han Sans SC",
    "PingFang SC",
    "Microsoft YaHei",
    "SimSun",
    "WenQuanYi Micro Hei",
    "WenQuanYi Zen Hei",
];
const AUTO_FALLBACK_CJK_TC: &[&str] = &[
    "Noto Sans Mono CJK TC",
    "Noto Sans CJK TC",
    "Source Han Sans TC",
    "PingFang TC",
    "Microsoft JhengHei",
];
const AUTO_FALLBACK_CJK_HK: &[&str] = &[
    "Noto Sans Mono CJK HK",
    "Noto Sans CJK HK",
    "Source Han Sans HK",
    "PingFang HK",
    "Microsoft JhengHei",
];
const AUTO_FALLBACK_CJK_JP: &[&str] = &[
    "Noto Sans Mono CJK JP",
    "Noto Sans CJK JP",
    "Source Han Sans JP",
    "Hiragino Sans",
    "Yu Gothic",
    "MS Gothic",
    "Meiryo",
];
const AUTO_FALLBACK_CJK_KR: &[&str] = &[
    "Noto Sans Mono CJK KR",
    "Noto Sans CJK KR",
    "Source Han Sans KR",
    "Apple SD Gothic Neo",
    "Malgun Gothic",
];
const AUTO_FALLBACK_GLOBAL: &[&str] = &[
    "Noto Sans",
    "Noto Sans Mono",
    "Segoe UI",
    "Arial Unicode MS",
];
const AUTO_FALLBACK_EMOJI: &[&str] = &["Apple Color Emoji", "Segoe UI Emoji", "Noto Color Emoji"];
const NOTOFONTS_STATE_URL: &str =
    "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/state.json";
const NOTOFONTS_FILES_REPO: &str = "notofonts/notofonts.github.io";
const NOTO_EMOJI_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/googlefonts/noto-emoji/main/fonts/NotoColorEmoji.ttf",
    "https://raw.githubusercontent.com/notofonts/noto-emoji/main/fonts/NotoColorEmoji.ttf",
];
const NOTO_CJK_SC_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/OTF/SimplifiedChinese/NotoSansCJKsc-Regular.otf",
    "https://raw.githubusercontent.com/googlefonts/noto-cjk/main/Sans/OTF/SimplifiedChinese/NotoSansCJKsc-Regular.otf",
];
const NOTO_CJK_TC_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/OTF/TraditionalChinese/NotoSansCJKtc-Regular.otf",
    "https://raw.githubusercontent.com/googlefonts/noto-cjk/main/Sans/OTF/TraditionalChinese/NotoSansCJKtc-Regular.otf",
];
const NOTO_CJK_HK_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/OTF/HongKong/NotoSansCJKhk-Regular.otf",
    "https://raw.githubusercontent.com/googlefonts/noto-cjk/main/Sans/OTF/HongKong/NotoSansCJKhk-Regular.otf",
];
const NOTO_CJK_JP_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/OTF/Japanese/NotoSansCJKjp-Regular.otf",
    "https://raw.githubusercontent.com/googlefonts/noto-cjk/main/Sans/OTF/Japanese/NotoSansCJKjp-Regular.otf",
];
const NOTO_CJK_KR_URLS: &[&str] = &[
    "https://raw.githubusercontent.com/notofonts/noto-cjk/main/Sans/OTF/Korean/NotoSansCJKkr-Regular.otf",
    "https://raw.githubusercontent.com/googlefonts/noto-cjk/main/Sans/OTF/Korean/NotoSansCJKkr-Regular.otf",
];
const DEFAULT_GITHUB_PROXIES: &[&str] = &["https://fastgit.cc/", "https://ghfast.top/"];

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
    pub fallbacks: Vec<String>,
    #[serde(rename = "system_fallback")]
    pub system_fallback: FontSystemFallback,
    #[serde(rename = "auto_download")]
    pub auto_download: bool,
    #[serde(rename = "force_update")]
    pub force_update: bool,
    #[serde(rename = "cjk_region")]
    pub cjk_region: CjkRegion,
    #[serde(rename = "dirs")]
    pub dirs: Vec<String>,
}

impl Default for Font {
    fn default() -> Self {
        Self {
            family: "monospace".to_string(),
            file: None,
            size: 14.0,
            ligatures: true,
            fallbacks: Vec::new(),
            system_fallback: FontSystemFallback::default(),
            auto_download: true,
            force_update: false,
            cjk_region: CjkRegion::default(),
            dirs: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FontSystemFallback {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CjkRegion {
    #[default]
    Auto,
    Sc,
    Tc,
    Hk,
    Jp,
    Kr,
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
    Ok(render_svg_with_plan(input, config)?.bytes)
}

struct RenderedSvg {
    bytes: Vec<u8>,
    font_plan: FontPlan,
}

fn render_svg_with_plan(input: &InputSource, config: &Config) -> Result<RenderedSvg> {
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

    let title_text = resolve_title_text(input, config);
    let needs = collect_font_fallback_needs(&lines, title_text.as_deref());
    let script_plan = resolve_script_font_plan(config, &needs);
    let script_plan = match script_plan {
        Ok(plan) => plan,
        Err(err) => {
            eprintln!("cryosnap: font plan failed: {}", err);
            ScriptFontPlan::default()
        }
    };
    let _ = ensure_fonts_available(config, &needs, &script_plan);
    let app_families = load_app_font_families(config).unwrap_or_default();
    let font_plan = build_font_plan(config, &needs, &app_families, &script_plan.families);
    let font_css = svg_font_face_css(config)?;
    let svg = build_svg(
        &lines,
        config,
        &default_fg,
        font_css,
        line_offset,
        title_text.as_deref(),
        &font_plan.font_family,
    );
    Ok(RenderedSvg {
        bytes: svg.into_bytes(),
        font_plan,
    })
}

pub fn render_png(input: &InputSource, config: &Config) -> Result<Vec<u8>> {
    let rendered = render_svg_with_plan(input, config)?;
    render_png_from_svg_with_plan(
        &rendered.bytes,
        config,
        rendered.font_plan.needs_system_fonts,
    )
}

pub fn render_webp(input: &InputSource, config: &Config) -> Result<Vec<u8>> {
    let rendered = render_svg_with_plan(input, config)?;
    render_webp_from_svg_with_plan(
        &rendered.bytes,
        config,
        rendered.font_plan.needs_system_fonts,
    )
}

pub fn render_png_from_svg(svg: &[u8], config: &Config) -> Result<Vec<u8>> {
    let needs = font_needs_from_svg(svg, config);
    render_png_from_svg_with_plan(svg, config, needs.needs_system_fonts)
}

fn render_png_from_svg_with_plan(
    svg: &[u8],
    config: &Config,
    needs_system_fonts: bool,
) -> Result<Vec<u8>> {
    if let Some(png) = try_render_png_with_rsvg(svg, config)? {
        let png = if config.png.quantize {
            quantize_png_bytes(&png, &config.png)?
        } else {
            png
        };
        return optimize_png(png, &config.png);
    }

    let pixmap = rasterize_svg(svg, config, needs_system_fonts)?;
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
    let needs = font_needs_from_svg(svg, config);
    render_webp_from_svg_with_plan(svg, config, needs.needs_system_fonts)
}

fn render_webp_from_svg_with_plan(
    svg: &[u8],
    config: &Config,
    needs_system_fonts: bool,
) -> Result<Vec<u8>> {
    if matches!(config.raster.backend, RasterBackend::Rsvg) {
        return Err(Error::Render(
            "rsvg backend does not support webp output".to_string(),
        ));
    }
    let pixmap = rasterize_svg(svg, config, needs_system_fonts)?;
    pixmap_to_webp(&pixmap)
}

#[derive(Debug, Default, Clone, Copy)]
struct SvgFontNeeds {
    needs_system_fonts: bool,
}

fn font_needs_from_svg(svg: &[u8], config: &Config) -> SvgFontNeeds {
    let mut needs = FontFallbackNeeds::default();
    let svg_text = std::str::from_utf8(svg).ok();
    if let Some(text) = svg_text {
        scan_text_fallbacks(text, &mut needs);
    }
    let script_plan = resolve_script_font_plan(config, &needs);
    let script_plan = match script_plan {
        Ok(plan) => plan,
        Err(err) => {
            eprintln!("cryosnap: font plan failed: {}", err);
            ScriptFontPlan::default()
        }
    };
    let _ = ensure_fonts_available(config, &needs, &script_plan);
    let app_families = load_app_font_families(config).unwrap_or_default();
    let families = build_font_families(config, &needs, &script_plan.families);
    let mut needs_system_fonts = needs_system_fonts(config, &app_families, &families);
    if svg_text.is_none() && matches!(config.font.system_fallback, FontSystemFallback::Auto) {
        needs_system_fonts = true;
    }

    SvgFontNeeds { needs_system_fonts }
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

fn rasterize_svg(
    svg: &[u8],
    config: &Config,
    needs_system_fonts: bool,
) -> Result<tiny_skia::Pixmap> {
    let mut opt = usvg::Options::default();
    let fontdb = build_fontdb(config, needs_system_fonts)?;
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

#[derive(Debug, Default, Clone)]
struct FontFallbackNeeds {
    needs_unicode: bool,
    needs_nf: bool,
    needs_cjk: bool,
    needs_emoji: bool,
    scripts: HashSet<Script>,
}

#[derive(Debug, Clone)]
struct FontPlan {
    font_family: String,
    needs_system_fonts: bool,
}

#[derive(Debug, Default, Clone)]
struct ScriptFontPlan {
    families: Vec<String>,
    downloads: Vec<ScriptDownload>,
}

#[derive(Debug, Clone)]
struct ScriptDownload {
    family: String,
    repo: String,
    file_path: String,
    filename: String,
    tag: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum FontStylePreference {
    Sans,
    Serif,
}

fn is_private_use(ch: char) -> bool {
    let cp = ch as u32;
    (0xE000..=0xF8FF).contains(&cp)
        || (0xF0000..=0xFFFFD).contains(&cp)
        || (0x100000..=0x10FFFD).contains(&cp)
}

fn is_cjk(ch: char) -> bool {
    let cp = ch as u32;
    matches!(
        cp,
        0x4E00..=0x9FFF
            | 0x3400..=0x4DBF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0x2CEB0..=0x2EBEF
            | 0x2F800..=0x2FA1F
            | 0x3040..=0x309F
            | 0x30A0..=0x30FF
            | 0x31F0..=0x31FF
            | 0x1100..=0x11FF
            | 0x3130..=0x318F
            | 0xAC00..=0xD7AF
            | 0x3100..=0x312F
            | 0x31A0..=0x31BF
    )
}

fn is_emoji(ch: char) -> bool {
    let cp = ch as u32;
    matches!(
        cp,
        0x2300..=0x23FF
            | 0x2600..=0x27BF
            | 0x2B00..=0x2BFF
            | 0x1F000..=0x1FAFF
    )
}

fn scan_text_fallbacks(text: &str, needs: &mut FontFallbackNeeds) {
    for ch in text.chars() {
        if ch > '\u{7f}' {
            needs.needs_unicode = true;
        }
        if is_private_use(ch) {
            needs.needs_nf = true;
        }
        if is_cjk(ch) {
            needs.needs_cjk = true;
        }
        if is_emoji(ch) {
            needs.needs_emoji = true;
        }
        if ch > '\u{7f}' {
            let script = ch.script();
            if !matches!(script, Script::Common | Script::Inherited | Script::Unknown) {
                needs.scripts.insert(script);
            }
        }
    }
}

fn collect_font_fallback_needs(lines: &[Line], title_text: Option<&str>) -> FontFallbackNeeds {
    let mut needs = FontFallbackNeeds::default();
    for line in lines {
        for span in &line.spans {
            scan_text_fallbacks(&span.text, &mut needs);
        }
    }
    if let Some(title) = title_text {
        scan_text_fallbacks(title, &mut needs);
    }
    needs
}

fn push_family(out: &mut Vec<String>, seen: &mut HashSet<String>, name: &str) {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return;
    }
    let key = trimmed.to_ascii_lowercase();
    if seen.insert(key) {
        out.push(trimmed.to_string());
    }
}

fn family_key(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

fn is_generic_family(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "serif" | "sans-serif" | "sans" | "monospace" | "cursive" | "fantasy"
    )
}

fn fallback_style_preference(config: &Config) -> FontStylePreference {
    let family = config.font.family.trim().to_ascii_lowercase();
    if matches!(family.as_str(), "serif") || family.contains("serif") {
        FontStylePreference::Serif
    } else {
        FontStylePreference::Sans
    }
}

fn normalize_repo_key(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn build_repo_key_index(state: &NotofontsState) -> HashMap<String, String> {
    let mut index = HashMap::new();
    for key in state.0.keys() {
        index.insert(normalize_repo_key(key), key.clone());
    }
    index
}

fn is_cjk_script(script: Script) -> bool {
    matches!(
        script,
        Script::Han | Script::Hiragana | Script::Katakana | Script::Hangul | Script::Bopomofo
    )
}

fn script_repo_key(script: Script, index: &HashMap<String, String>) -> Option<String> {
    match script {
        Script::Common | Script::Inherited | Script::Unknown => return None,
        Script::Latin | Script::Greek | Script::Cyrillic => {
            return Some("latin-greek-cyrillic".to_string())
        }
        _ => {}
    }
    if is_cjk_script(script) {
        return None;
    }
    let name = script.full_name();
    let normalized = normalize_repo_key(name);
    index.get(&normalized).cloned()
}

fn parse_cjk_region_from_locale(value: &str) -> Option<CjkRegion> {
    let raw = value.trim();
    if raw.is_empty() {
        return None;
    }
    let mut base = raw.to_ascii_lowercase();
    if let Some(pos) = base.find(['.', '@']) {
        base.truncate(pos);
    }
    let normalized = base.replace('-', "_");
    let parts = normalized
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    match parts[0] {
        "ja" => return Some(CjkRegion::Jp),
        "ko" => return Some(CjkRegion::Kr),
        "zh" => {
            if parts.iter().any(|part| matches!(*part, "hk" | "mo")) {
                return Some(CjkRegion::Hk);
            }
            if parts.contains(&"tw") {
                return Some(CjkRegion::Tc);
            }
            if parts.contains(&"hant") {
                return Some(CjkRegion::Tc);
            }
            if parts.iter().any(|part| matches!(*part, "cn" | "sg")) {
                return Some(CjkRegion::Sc);
            }
            if parts.contains(&"hans") {
                return Some(CjkRegion::Sc);
            }
        }
        _ => {}
    }
    None
}

fn locale_cjk_region() -> Option<CjkRegion> {
    for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Ok(value) = env::var(key) {
            if let Some(region) = parse_cjk_region_from_locale(&value) {
                return Some(region);
            }
        }
    }
    None
}

fn push_cjk_region(out: &mut Vec<CjkRegion>, seen: &mut HashSet<CjkRegion>, region: CjkRegion) {
    if seen.insert(region) {
        out.push(region);
    }
}

fn collect_cjk_regions(config: &Config, needs: &FontFallbackNeeds) -> Vec<CjkRegion> {
    if !needs.needs_cjk {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    if needs.scripts.contains(&Script::Hiragana) || needs.scripts.contains(&Script::Katakana) {
        push_cjk_region(&mut out, &mut seen, CjkRegion::Jp);
    }
    if needs.scripts.contains(&Script::Hangul) {
        push_cjk_region(&mut out, &mut seen, CjkRegion::Kr);
    }
    if needs.scripts.contains(&Script::Bopomofo) {
        push_cjk_region(&mut out, &mut seen, CjkRegion::Tc);
    }

    if needs.scripts.contains(&Script::Han) || out.is_empty() {
        let region = match config.font.cjk_region {
            CjkRegion::Auto => locale_cjk_region().unwrap_or(CjkRegion::Sc),
            other => other,
        };
        push_cjk_region(&mut out, &mut seen, region);
    }

    out
}

fn cjk_region_families(region: CjkRegion) -> &'static [&'static str] {
    match region {
        CjkRegion::Sc => AUTO_FALLBACK_CJK_SC,
        CjkRegion::Tc => AUTO_FALLBACK_CJK_TC,
        CjkRegion::Hk => AUTO_FALLBACK_CJK_HK,
        CjkRegion::Jp => AUTO_FALLBACK_CJK_JP,
        CjkRegion::Kr => AUTO_FALLBACK_CJK_KR,
        CjkRegion::Auto => AUTO_FALLBACK_CJK_SC,
    }
}

fn cjk_region_urls(region: CjkRegion) -> &'static [&'static str] {
    match region {
        CjkRegion::Sc => NOTO_CJK_SC_URLS,
        CjkRegion::Tc => NOTO_CJK_TC_URLS,
        CjkRegion::Hk => NOTO_CJK_HK_URLS,
        CjkRegion::Jp => NOTO_CJK_JP_URLS,
        CjkRegion::Kr => NOTO_CJK_KR_URLS,
        CjkRegion::Auto => NOTO_CJK_SC_URLS,
    }
}

fn cjk_region_filename(region: CjkRegion) -> &'static str {
    match region {
        CjkRegion::Sc => "NotoSansCJKsc-Regular.otf",
        CjkRegion::Tc => "NotoSansCJKtc-Regular.otf",
        CjkRegion::Hk => "NotoSansCJKhk-Regular.otf",
        CjkRegion::Jp => "NotoSansCJKjp-Regular.otf",
        CjkRegion::Kr => "NotoSansCJKkr-Regular.otf",
        CjkRegion::Auto => "NotoSansCJKsc-Regular.otf",
    }
}

fn choose_family_name(
    families: &HashMap<String, NotofontsFamily>,
    style: FontStylePreference,
) -> Option<String> {
    let candidates = families
        .iter()
        .filter(|(_, info)| info.latest_release.is_some() || !info.files.is_empty())
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }
    let mut best: Option<(i32, &String)> = None;
    for name in &candidates {
        let score = score_family_name(name, style);
        match best {
            None => best = Some((score, name)),
            Some((best_score, best_name)) => {
                if score > best_score
                    || (score == best_score && name.len() < best_name.len())
                    || (score == best_score && name.len() == best_name.len() && name < best_name)
                {
                    best = Some((score, name));
                }
            }
        }
    }
    best.map(|(_, name)| name.clone())
}

fn tag_from_release_url(url: &str) -> Option<String> {
    url.rsplit('/').next().map(|v| v.to_string())
}

fn score_family_name(name: &str, style: FontStylePreference) -> i32 {
    let lower = name.to_ascii_lowercase();
    let mut score = 0;
    match style {
        FontStylePreference::Sans => {
            if lower.contains("noto sans") {
                score += 300;
            } else if lower.contains("sans") {
                score += 200;
            }
            if lower.contains("kufi") {
                score += 120;
            }
        }
        FontStylePreference::Serif => {
            if lower.contains("noto serif") {
                score += 300;
            } else if lower.contains("serif") {
                score += 200;
            }
            if lower.contains("naskh") {
                score += 120;
            }
        }
    }
    if lower.contains("supplement") {
        score -= 200;
    }
    if lower.contains("looped") {
        score -= 120;
    }
    if lower.contains("display") {
        score -= 40;
    }
    if lower.contains("ui") {
        score -= 20;
    }
    score
}

fn repo_from_release_url(url: &str) -> Option<String> {
    let suffix = url.split("github.com/").nth(1)?;
    let mut parts = suffix.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(format!("{owner}/{repo}"))
}

fn score_font_path(path: &str) -> Option<i32> {
    let lower = path.to_ascii_lowercase();
    let ext_score = if lower.ends_with(".ttf") {
        100
    } else if lower.ends_with(".otf") {
        80
    } else {
        return None;
    };
    let mut score = ext_score;
    if lower.contains("/full/") {
        score += 60;
    }
    if lower.contains("/hinted/") {
        score += 45;
    }
    if lower.contains("/googlefonts/") {
        score += 30;
    }
    if lower.contains("/unhinted/") {
        score += 10;
    }
    if lower.contains("regular") {
        score += 200;
    }
    if lower.contains("italic") {
        score -= 120;
    }
    if lower.contains("variable") || lower.contains('[') {
        score -= 20;
    }
    if lower.contains("slim") {
        score -= 10;
    }
    Some(score)
}

fn pick_best_font_file(files: &[String]) -> Option<String> {
    let mut best: Option<(i32, &String)> = None;
    for file in files {
        let Some(score) = score_font_path(file) else {
            continue;
        };
        match best {
            None => best = Some((score, file)),
            Some((best_score, best_file)) => {
                if score > best_score || (score == best_score && file.len() < best_file.len()) {
                    best = Some((score, file));
                }
            }
        }
    }
    best.map(|(_, file)| file.clone())
}

fn resolve_script_font_plan(config: &Config, needs: &FontFallbackNeeds) -> Result<ScriptFontPlan> {
    if needs.scripts.is_empty() || !needs.needs_unicode {
        return Ok(ScriptFontPlan::default());
    }
    let state = load_notofonts_state(force_update_enabled(config))?;
    let index = build_repo_key_index(&state);
    let style = fallback_style_preference(config);
    let mut families = Vec::new();
    let mut downloads = Vec::new();
    let mut seen_repo = HashSet::new();
    let mut seen_family = HashSet::new();
    let mut seen_download = HashSet::new();

    let mut scripts = needs.scripts.iter().copied().collect::<Vec<_>>();
    scripts.sort_by_key(|script| script.full_name().to_string());
    for script in scripts {
        let Some(repo_key) = script_repo_key(script, &index) else {
            continue;
        };
        if !seen_repo.insert(repo_key.clone()) {
            continue;
        }
        let Some(repo) = state.0.get(&repo_key) else {
            continue;
        };
        let Some(family) = choose_family_name(&repo.families, style) else {
            continue;
        };
        let Some(family_info) = repo.families.get(&family) else {
            continue;
        };
        let Some(file_path) = pick_best_font_file(&family_info.files) else {
            continue;
        };
        let repo_name = if file_path.starts_with("fonts/") {
            NOTOFONTS_FILES_REPO.to_string()
        } else {
            family_info
                .latest_release
                .as_ref()
                .and_then(|release| repo_from_release_url(&release.url))
                .unwrap_or_else(|| format!("notofonts/{repo_key}"))
        };
        let raw_name = Path::new(&file_path)
            .file_name()
            .and_then(|value| value.to_str());
        let Some(raw_name) = raw_name else {
            continue;
        };
        let filename = format!("{}__{}", repo_name.replace('/', "_"), raw_name);
        let tag = if repo_name == NOTOFONTS_FILES_REPO {
            None
        } else {
            family_info
                .latest_release
                .as_ref()
                .and_then(|release| tag_from_release_url(&release.url))
        };
        if seen_family.insert(family.clone()) {
            families.push(family.clone());
        }
        let download_key = format!("{repo_name}|{file_path}");
        if seen_download.insert(download_key) {
            downloads.push(ScriptDownload {
                family,
                repo: repo_name,
                file_path,
                filename,
                tag,
            });
        }
    }

    Ok(ScriptFontPlan {
        families,
        downloads,
    })
}

fn build_font_families(
    config: &Config,
    needs: &FontFallbackNeeds,
    script_families: &[String],
) -> Vec<String> {
    let mut families = Vec::new();
    let mut seen = HashSet::new();
    push_family(&mut families, &mut seen, &config.font.family);
    for name in &config.font.fallbacks {
        push_family(&mut families, &mut seen, name);
    }
    for name in script_families {
        push_family(&mut families, &mut seen, name);
    }

    if needs.needs_nf {
        for name in AUTO_FALLBACK_NF {
            push_family(&mut families, &mut seen, name);
        }
    }
    if needs.needs_cjk {
        for name in AUTO_FALLBACK_CJK {
            push_family(&mut families, &mut seen, name);
        }
    }
    if needs.needs_unicode {
        for name in AUTO_FALLBACK_GLOBAL {
            push_family(&mut families, &mut seen, name);
        }
    }
    if needs.needs_emoji {
        for name in AUTO_FALLBACK_EMOJI {
            push_family(&mut families, &mut seen, name);
        }
    }

    families
}

fn family_requires_system(name: &str, app_families: &HashSet<String>) -> bool {
    if is_generic_family(name) {
        return true;
    }
    let key = family_key(name);
    !app_families.contains(&key)
}

fn needs_system_fonts(
    config: &Config,
    app_families: &HashSet<String>,
    families: &[String],
) -> bool {
    match config.font.system_fallback {
        FontSystemFallback::Never => return false,
        FontSystemFallback::Always => return true,
        FontSystemFallback::Auto => {}
    }
    let mut needs = false;
    if config.font.file.is_none() && family_requires_system(&config.font.family, app_families) {
        needs = true;
    }
    if !needs {
        for name in families {
            if config.font.file.is_some() && name.eq_ignore_ascii_case(&config.font.family) {
                continue;
            }
            if family_requires_system(name, app_families) {
                needs = true;
                break;
            }
        }
    }
    needs
}

fn build_font_plan(
    config: &Config,
    needs: &FontFallbackNeeds,
    app_families: &HashSet<String>,
    script_families: &[String],
) -> FontPlan {
    let families = build_font_families(config, needs, script_families);
    let font_family = families.join(", ");
    let needs_system_fonts = needs_system_fonts(config, app_families, &families);
    FontPlan {
        font_family,
        needs_system_fonts,
    }
}

fn build_fontdb(config: &Config, needs_system_fonts: bool) -> Result<usvg::fontdb::Database> {
    let mut fontdb = usvg::fontdb::Database::new();
    if let Some(font_file) = &config.font.file {
        let bytes = std::fs::read(font_file)?;
        fontdb.load_font_data(bytes);
    }
    for dir in resolve_font_dirs(config)? {
        if dir.is_dir() {
            fontdb.load_fonts_dir(dir);
        }
    }
    if needs_system_fonts {
        fontdb.load_system_fonts();
    }
    Ok(fontdb)
}

fn resolve_font_dirs(config: &Config) -> Result<Vec<PathBuf>> {
    let raw = env::var("CRYOSNAP_FONT_DIRS").ok();
    if let Some(raw) = raw {
        return parse_font_dir_list(&raw);
    }
    if !config.font.dirs.is_empty() {
        return Ok(config
            .font
            .dirs
            .iter()
            .filter_map(|value| expand_home_dir(value))
            .collect());
    }
    Ok(vec![default_font_dir()?])
}

fn parse_font_dir_list(raw: &str) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for part in raw.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(path) = expand_home_dir(trimmed) {
            out.push(path);
        }
    }
    Ok(out)
}

fn expand_home_dir(value: &str) -> Option<PathBuf> {
    if value == "~" || value.starts_with("~/") || value.starts_with("~\\") {
        let home = home_dir()?;
        let rest = value.trim_start_matches('~');
        return Some(if rest.is_empty() {
            home
        } else {
            home.join(rest.trim_start_matches(['/', '\\']))
        });
    }
    Some(PathBuf::from(value))
}

fn default_font_dir() -> Result<PathBuf> {
    Ok(default_app_dir()?.join("fonts"))
}

fn default_app_dir() -> Result<PathBuf> {
    if let Ok(path) = env::var("CRYOSNAP_HOME") {
        return Ok(PathBuf::from(path));
    }
    let home = home_dir()
        .ok_or_else(|| Error::InvalidInput("unable to resolve home directory".to_string()))?;
    Ok(home.join(".cryosnap"))
}

fn home_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        if let Some(path) = env::var_os("USERPROFILE") {
            return Some(PathBuf::from(path));
        }
        if let (Some(drive), Some(path)) = (env::var_os("HOMEDRIVE"), env::var_os("HOMEPATH")) {
            return Some(PathBuf::from(drive).join(path));
        }
        None
    } else {
        env::var_os("HOME").map(PathBuf::from)
    }
}

fn collect_font_families(db: &usvg::fontdb::Database) -> HashSet<String> {
    let mut families = HashSet::new();
    for face in db.faces() {
        for (family, _) in &face.families {
            families.insert(family_key(family));
        }
    }
    families
}

fn load_app_font_families(config: &Config) -> Result<HashSet<String>> {
    let mut fontdb = usvg::fontdb::Database::new();
    for dir in resolve_font_dirs(config)? {
        if dir.is_dir() {
            fontdb.load_fonts_dir(dir);
        }
    }
    Ok(collect_font_families(&fontdb))
}

fn load_system_font_families() -> HashSet<String> {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    collect_font_families(&fontdb)
}

fn auto_download_enabled(config: &Config) -> bool {
    if let Ok(value) = env::var("CRYOSNAP_FONT_AUTO_DOWNLOAD") {
        let value = value.trim().to_ascii_lowercase();
        return !(value == "0" || value == "false" || value == "no" || value == "off");
    }
    config.font.auto_download
}

fn force_update_enabled(config: &Config) -> bool {
    if let Ok(value) = env::var("CRYOSNAP_FONT_FORCE_UPDATE") {
        let value = value.trim().to_ascii_lowercase();
        return !(value == "0" || value == "false" || value == "no" || value == "off");
    }
    config.font.force_update
}

#[derive(Debug, Clone, Deserialize)]
struct NotofontsState(HashMap<String, NotofontsRepo>);

#[derive(Debug, Clone, Deserialize)]
struct NotofontsRepo {
    #[serde(default)]
    families: HashMap<String, NotofontsFamily>,
}

#[derive(Debug, Clone, Deserialize)]
struct NotofontsFamily {
    #[serde(default)]
    latest_release: Option<NotofontsRelease>,
    #[serde(default)]
    files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct NotofontsRelease {
    url: String,
}

static NOTOFONTS_STATE: Lazy<Mutex<Option<Arc<NotofontsState>>>> = Lazy::new(|| Mutex::new(None));
static HTTP_AGENT: Lazy<ureq::Agent> = Lazy::new(|| {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(600))
        .build()
});

fn github_proxy_candidates() -> Vec<String> {
    if let Ok(value) = env::var("CRYOSNAP_GITHUB_PROXY") {
        let parts = value
            .split(',')
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .collect::<Vec<_>>();
        if !parts.is_empty() {
            return parts;
        }
    }
    DEFAULT_GITHUB_PROXIES
        .iter()
        .map(|v| v.to_string())
        .collect()
}

fn cache_dir() -> Result<PathBuf> {
    Ok(default_app_dir()?.join("cache"))
}

fn apply_github_proxy(url: &str, proxy: &str) -> String {
    let mut base = proxy.trim().to_string();
    if !base.ends_with('/') {
        base.push('/');
    }
    format!("{base}{url}")
}

enum FetchOutcome {
    Ok(Box<ureq::Response>, Option<String>),
    NotModified,
}

fn build_github_candidates() -> Vec<Option<String>> {
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    candidates.push(None);
    for proxy in github_proxy_candidates() {
        if seen.insert(proxy.clone()) {
            candidates.push(Some(proxy));
        }
    }
    candidates
}

fn looks_like_json(bytes: &[u8]) -> bool {
    for &b in bytes {
        if !b.is_ascii_whitespace() {
            return b == b'{' || b == b'[';
        }
    }
    false
}

fn fetch_with_candidates(url: &str, headers: &[(&str, &str)]) -> Result<FetchOutcome> {
    let candidates = build_github_candidates();

    let mut last_error: Option<String> = None;
    for proxy_opt in candidates {
        let target = match &proxy_opt {
            Some(proxy) => apply_github_proxy(url, proxy),
            None => url.to_string(),
        };
        let mut req = HTTP_AGENT
            .get(&target)
            .set("User-Agent", "cryosnap/auto-font");
        for (key, value) in headers {
            req = req.set(key, value);
        }
        match req.call() {
            Ok(resp) => {
                if resp.status() == 304 {
                    return Ok(FetchOutcome::NotModified);
                }
                return Ok(FetchOutcome::Ok(Box::new(resp), proxy_opt.clone()));
            }
            Err(ureq::Error::Status(304, _)) => return Ok(FetchOutcome::NotModified),
            Err(err) => {
                last_error = Some(format!("{err}"));
                continue;
            }
        }
    }
    Err(Error::Render(format!(
        "download failed: {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    )))
}

fn fetch_bytes_with_cache(url: &str, cache_name: &str, force_update: bool) -> Result<Vec<u8>> {
    let cache_dir = cache_dir()?;
    let data_path = cache_dir.join(cache_name);
    let etag_path = cache_dir.join(format!("{cache_name}.etag"));
    let mut headers: Vec<(&str, &str)> = Vec::new();
    let mut etag_holder: Option<String> = None;
    if data_path.exists() && !force_update {
        if let Ok(etag) = fs::read_to_string(&etag_path) {
            let tag = etag.trim().to_string();
            if !tag.is_empty() {
                etag_holder = Some(tag);
            }
        }
    }
    if let Some(tag) = etag_holder.as_ref() {
        headers.push(("If-None-Match", tag.as_str()));
    }
    let candidates = build_github_candidates();
    let mut last_error: Option<String> = None;
    for proxy_opt in candidates {
        let target = match &proxy_opt {
            Some(proxy) => apply_github_proxy(url, proxy),
            None => url.to_string(),
        };
        let mut req = HTTP_AGENT
            .get(&target)
            .set("User-Agent", "cryosnap/auto-font");
        for (key, value) in &headers {
            req = req.set(key, value);
        }
        match req.call() {
            Ok(resp) => {
                if resp.status() == 304 {
                    if data_path.exists() {
                        let cached = fs::read(&data_path)?;
                        if looks_like_json(&cached) {
                            return Ok(cached);
                        }
                        let _ = fs::remove_file(&data_path);
                        let _ = fs::remove_file(&etag_path);
                    }
                    last_error = Some("font state cache missing".to_string());
                    continue;
                }
                let etag_value = resp.header("ETag").map(|v| v.to_string());
                let mut reader = resp.into_reader();
                let mut buf = Vec::new();
                reader.read_to_end(&mut buf)?;
                if !looks_like_json(&buf) {
                    last_error = Some("invalid response".to_string());
                    continue;
                }
                if let Some(parent) = data_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&data_path, &buf)?;
                if let Some(etag) = etag_value {
                    let _ = fs::write(&etag_path, etag.as_bytes());
                }
                return Ok(buf);
            }
            Err(ureq::Error::Status(304, _)) => {
                if data_path.exists() {
                    let cached = fs::read(&data_path)?;
                    if looks_like_json(&cached) {
                        return Ok(cached);
                    }
                    let _ = fs::remove_file(&data_path);
                    let _ = fs::remove_file(&etag_path);
                }
                last_error = Some("font state cache missing".to_string());
                continue;
            }
            Err(err) => {
                last_error = Some(format!("{err}"));
                continue;
            }
        }
    }
    if data_path.exists() {
        let cached = fs::read(&data_path)?;
        if looks_like_json(&cached) {
            return Ok(cached);
        }
        let _ = fs::remove_file(&data_path);
        let _ = fs::remove_file(&etag_path);
    }
    Err(Error::Render(format!(
        "download failed: {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    )))
}

fn download_url_with_etag(url: &str, target: &Path, force_update: bool) -> Result<bool> {
    let etag_path = target.with_extension(format!(
        "{}.etag",
        target
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("font")
    ));
    let mut headers: Vec<(&str, &str)> = Vec::new();
    let mut etag_holder: Option<String> = None;
    if target.exists() && !force_update {
        if let Ok(etag) = fs::read_to_string(&etag_path) {
            let tag = etag.trim().to_string();
            if !tag.is_empty() {
                etag_holder = Some(tag);
            }
        }
    }
    if let Some(tag) = etag_holder.as_ref() {
        headers.push(("If-None-Match", tag.as_str()));
    }
    match fetch_with_candidates(url, &headers)? {
        FetchOutcome::Ok(resp, _proxy) => {
            let etag_value = resp.header("ETag").map(|v| v.to_string());
            let mut reader = resp.into_reader();
            let temp = target.with_extension("download");
            let mut file = fs::File::create(&temp)?;
            std::io::copy(&mut reader, &mut file)?;
            file.sync_all()?;
            fs::rename(&temp, target)?;
            if let Some(etag) = etag_value {
                let _ = fs::write(&etag_path, etag.as_bytes());
            }
            Ok(true)
        }
        FetchOutcome::NotModified => Ok(false),
    }
}

fn load_notofonts_state(force_update: bool) -> Result<Arc<NotofontsState>> {
    if !force_update {
        if let Ok(guard) = NOTOFONTS_STATE.lock() {
            if let Some(state) = guard.as_ref() {
                return Ok(state.clone());
            }
        }
    }
    let bytes = fetch_bytes_with_cache(NOTOFONTS_STATE_URL, "notofonts_state.json", force_update)?;
    let state: NotofontsState = serde_json::from_slice(&bytes)
        .map_err(|err| Error::Render(format!("font state parse: {err}")))?;
    let state = Arc::new(state);
    if !force_update {
        if let Ok(mut guard) = NOTOFONTS_STATE.lock() {
            *guard = Some(state.clone());
        }
    }
    Ok(state)
}

struct FontPackage {
    id: &'static str,
    family: &'static str,
    filename: &'static str,
    url: &'static str,
    download_sha256: &'static str,
    file_sha256: &'static str,
    archive_entry: Option<&'static str>,
}

const FONT_PACKAGE_NF: FontPackage = FontPackage {
    id: "symbols-nerd-font-mono",
    family: "Symbols Nerd Font Mono",
    filename: "SymbolsNerdFontMono-Regular.ttf",
    url:
        "https://github.com/ryanoasis/nerd-fonts/releases/download/v3.2.1/NerdFontsSymbolsOnly.zip",
    download_sha256: "bc59c2ea74d022a6262ff9e372fde5c36cd5ae3f82a567941489ecfab4f03d66",
    file_sha256: "6f7e339af33bde250a4d7360a3176ab1ffe4e99c00eef0d71b4c322364c595f3",
    archive_entry: Some("SymbolsNerdFontMono-Regular.ttf"),
};

fn ensure_fonts_available(
    config: &Config,
    needs: &FontFallbackNeeds,
    script_plan: &ScriptFontPlan,
) -> Result<()> {
    if !auto_download_enabled(config) {
        return Ok(());
    }
    let force_update = force_update_enabled(config);
    if !needs.needs_nf && !needs.needs_cjk && !needs.needs_emoji && script_plan.downloads.is_empty()
    {
        return Ok(());
    }
    let font_dirs = resolve_font_dirs(config)?;
    let Some(primary_dir) = font_dirs.first() else {
        return Ok(());
    };
    let app_families = load_app_font_families(config).unwrap_or_default();
    let allow_system = !matches!(config.font.system_fallback, FontSystemFallback::Never);
    let system_families = if allow_system {
        load_system_font_families()
    } else {
        HashSet::new()
    };

    fs::create_dir_all(primary_dir)?;

    for download in &script_plan.downloads {
        let app_has = app_families.contains(&family_key(&download.family));
        let system_has = allow_system && system_families.contains(&family_key(&download.family));
        if system_has && !app_has {
            continue;
        }
        if app_has {
            if !force_update {
                continue;
            }
            let target = primary_dir.join(&download.filename);
            if !target.exists() {
                continue;
            }
        }
        if let Err(err) = download_notofonts_file(download, primary_dir, force_update) {
            eprintln!(
                "cryosnap: font download failed for {}: {}",
                download.family, err
            );
        }
    }

    if needs.needs_nf
        && !any_family_present(&[FONT_PACKAGE_NF.family], &app_families)
        && !(allow_system && any_family_present(&[FONT_PACKAGE_NF.family], &system_families))
    {
        if let Err(err) = download_font_package(&FONT_PACKAGE_NF, primary_dir) {
            eprintln!(
                "cryosnap: font download failed for {}: {}",
                FONT_PACKAGE_NF.id, err
            );
        }
    }

    if needs.needs_cjk {
        let cjk_regions = collect_cjk_regions(config, needs);
        for region in cjk_regions {
            let families = cjk_region_families(region);
            let app_has = any_family_present(families, &app_families);
            let system_has = allow_system && any_family_present(families, &system_families);
            if system_has && !app_has {
                continue;
            }
            let filename = cjk_region_filename(region);
            let target = primary_dir.join(filename);
            if app_has {
                if !force_update {
                    continue;
                }
                if !target.exists() {
                    continue;
                }
            }
            if let Err(err) =
                download_raw_font(cjk_region_urls(region), primary_dir, filename, force_update)
            {
                eprintln!("cryosnap: font download failed for cjk: {}", err);
            }
        }
    }

    if needs.needs_emoji {
        let app_has = any_family_present(AUTO_FALLBACK_EMOJI, &app_families);
        let system_has = allow_system && any_family_present(AUTO_FALLBACK_EMOJI, &system_families);
        if !system_has || app_has {
            let filename = "NotoColorEmoji.ttf";
            let target = primary_dir.join(filename);
            if !app_has || (force_update && target.exists()) {
                if let Err(err) =
                    download_raw_font(NOTO_EMOJI_URLS, primary_dir, filename, force_update)
                {
                    eprintln!("cryosnap: font download failed for emoji: {}", err);
                }
            }
        }
    }

    Ok(())
}

fn any_family_present(families: &[&str], set: &HashSet<String>) -> bool {
    families.iter().any(|name| set.contains(&family_key(name)))
}

fn download_raw_font(urls: &[&str], dir: &Path, filename: &str, force_update: bool) -> Result<()> {
    let target = dir.join(filename);
    let mut last_error: Option<Error> = None;
    for url in urls {
        match download_url_with_etag(url, &target, force_update) {
            Ok(_) => return Ok(()),
            Err(err) => last_error = Some(err),
        }
    }
    Err(last_error
        .unwrap_or_else(|| Error::Render("font download failed: no available urls".to_string())))
}

fn download_notofonts_file(
    download: &ScriptDownload,
    dir: &Path,
    force_update: bool,
) -> Result<()> {
    let target = dir.join(&download.filename);
    if !force_update && target.exists() {
        return Ok(());
    }
    let mut refs = vec!["main".to_string(), "master".to_string()];
    if let Some(tag) = &download.tag {
        if !tag.is_empty() {
            refs.push(tag.clone());
        }
    }
    let mut last_error: Option<Error> = None;
    for reference in refs {
        let url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}",
            download.repo, reference, download.file_path
        );
        match download_url_with_etag(&url, &target, force_update) {
            Ok(_) => return Ok(()),
            Err(err) => last_error = Some(err),
        }
    }
    Err(last_error
        .unwrap_or_else(|| Error::Render("font download failed: no available refs".to_string())))
}

fn download_url_to_file(url: &str, target: &Path) -> Result<()> {
    match fetch_with_candidates(url, &[])? {
        FetchOutcome::Ok(resp, _proxy) => {
            let mut reader = resp.into_reader();
            let mut file = fs::File::create(target)?;
            std::io::copy(&mut reader, &mut file)?;
            file.sync_all()?;
            Ok(())
        }
        FetchOutcome::NotModified => Ok(()),
    }
}

fn download_zip_with_candidates(url: &str, target: &Path) -> Result<()> {
    let candidates = build_github_candidates();
    let mut last_error: Option<String> = None;
    for proxy_opt in candidates {
        let target_url = match &proxy_opt {
            Some(proxy) => apply_github_proxy(url, proxy),
            None => url.to_string(),
        };
        let req = HTTP_AGENT
            .get(&target_url)
            .set("User-Agent", "cryosnap/auto-font");
        match req.call() {
            Ok(resp) => {
                let temp = target.with_extension("download");
                let mut reader = resp.into_reader();
                let mut file = fs::File::create(&temp)?;
                std::io::copy(&mut reader, &mut file)?;
                file.sync_all()?;
                if let Err(err) = validate_zip_archive(&temp) {
                    last_error = Some(err.to_string());
                    let _ = fs::remove_file(&temp);
                    continue;
                }
                fs::rename(&temp, target)?;
                return Ok(());
            }
            Err(ureq::Error::Status(status, _)) => {
                last_error = Some(format!("status {status}"));
                continue;
            }
            Err(err) => {
                last_error = Some(format!("{err}"));
                continue;
            }
        }
    }
    Err(Error::Render(format!(
        "download failed: {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    )))
}

fn validate_zip_archive(path: &Path) -> Result<()> {
    let file = fs::File::open(path)?;
    match zip::ZipArchive::new(file) {
        Ok(_) => Ok(()),
        Err(err) => Err(Error::Render(format!("zip read: {err}"))),
    }
}

fn download_font_package(pkg: &FontPackage, dir: &Path) -> Result<()> {
    let target = dir.join(pkg.filename);
    if verify_sha256(&target, pkg.file_sha256)? {
        return Ok(());
    }
    let temp = dir.join(format!("{}.download", pkg.filename));
    if pkg.archive_entry.is_some() {
        download_zip_with_candidates(pkg.url, &temp)?;
    } else {
        download_url_to_file(pkg.url, &temp)?;
    }
    if !verify_sha256(&temp, pkg.download_sha256)? {
        let _ = fs::remove_file(&temp);
        return Err(Error::Render(format!(
            "font checksum mismatch for {}",
            pkg.id
        )));
    }
    if let Some(entry) = pkg.archive_entry {
        extract_zip_entry(&temp, entry, &target)?;
        let _ = fs::remove_file(&temp);
    } else {
        fs::rename(&temp, &target)?;
    }
    if !verify_sha256(&target, pkg.file_sha256)? {
        return Err(Error::Render(format!(
            "font checksum mismatch for {}",
            pkg.id
        )));
    }
    Ok(())
}

fn extract_zip_entry(archive_path: &Path, entry: &str, target: &Path) -> Result<()> {
    let file = fs::File::open(archive_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|err| Error::Render(format!("zip read: {err}")))?;
    let mut entry_file = archive
        .by_name(entry)
        .map_err(|err| Error::Render(format!("zip entry {entry}: {err}")))?;
    let mut out = fs::File::create(target)?;
    std::io::copy(&mut entry_file, &mut out)?;
    out.sync_all()?;
    Ok(())
}

fn verify_sha256(path: &Path, expected: &str) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    if expected.trim().is_empty() {
        return Ok(true);
    }
    let actual = sha256_hex(path)?;
    Ok(actual.eq_ignore_ascii_case(expected.trim()))
}

fn sha256_hex(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{:02x}", byte));
    }
    Ok(out)
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
    font_family: &str,
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
                            escape_attr(font_family),
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
        escape_attr(font_family),
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
    let mut rules = Vec::new();
    let mut push_rule = |family: &str, data: Vec<u8>, format: &str, mime: &str| {
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        rules.push(format!(
            "@font-face {{ font-family: '{}'; src: url(data:{};base64,{}) format('{}'); }}",
            escape_attr(family),
            mime,
            encoded,
            format
        ));
    };
    if let Some(font_file) = &config.font.file {
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
        push_rule(&config.font.family, bytes, format, mime);
    }

    if rules.is_empty() {
        Ok(None)
    } else {
        Ok(Some(rules.join("")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Mutex, OnceLock};
    use std::thread;

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
        let svg = build_svg(&[line], &cfg, "#FFFFFF", None, 0, None, &cfg.font.family);
        assert!(svg.contains("filter id=\"shadow\""));
        assert!(svg.contains("clipPath"));
        assert!(svg.contains("font-family=\"Test\""));
        assert!(svg.contains("circle"));
    }

    #[test]
    fn build_svg_renders_title_and_styles() {
        let styled = Span {
            text: "ab\tcd".to_string(),
            style: TextStyle {
                fg: Some("#ff0000".to_string()),
                bg: Some("#00ff00".to_string()),
                bold: true,
                italic: true,
                underline: true,
                strike: true,
            },
        };
        let plain = Span {
            text: " ef".to_string(),
            style: TextStyle::default(),
        };
        let line = Line {
            spans: vec![styled, plain],
        };
        let mut cfg = Config::default();
        cfg.font.family = "Test".to_string();
        cfg.font.size = 16.0;
        cfg.line_height = 1.2;
        cfg.padding = vec![8.0];
        cfg.margin = vec![4.0];
        cfg.width = 320.0;
        cfg.height = 200.0;
        cfg.window_controls = true;
        cfg.show_line_numbers = true;
        cfg.shadow.blur = 4.0;
        cfg.shadow.x = 1.0;
        cfg.shadow.y = 2.0;
        cfg.border.radius = 3.0;
        cfg.border.width = 1.0;
        cfg.title.enabled = true;
        cfg.title.text = Some("Center Title Example".to_string());
        cfg.title.align = TitleAlign::Center;
        cfg.title.opacity = 0.5;
        cfg.title.max_width = 8;
        cfg.title.size = 0.0;
        cfg.title.ellipsis = "..".to_string();

        let svg_center = build_svg(
            &[line.clone()],
            &cfg,
            "#FFFFFF",
            Some("/*css*/".to_string()),
            3,
            cfg.title.text.as_deref(),
            &cfg.font.family,
        );
        assert!(svg_center.contains("<defs><style type=\"text/css\">"));
        assert!(svg_center.contains("text-anchor=\"middle\""));
        assert!(svg_center.contains("fill-opacity="));
        assert!(svg_center.contains("text-decoration=\"underline line-through\""));
        assert!(svg_center.contains("font-weight=\"bold\""));
        assert!(svg_center.contains("font-style=\"italic\""));

        let mut cfg_right = cfg.clone();
        cfg_right.title.text = Some("Right Title".to_string());
        cfg_right.title.align = TitleAlign::Right;
        cfg_right.title.opacity = 1.0;
        let svg_right = build_svg(
            &[line.clone()],
            &cfg_right,
            "#FFFFFF",
            None,
            0,
            cfg_right.title.text.as_deref(),
            &cfg_right.font.family,
        );
        assert!(svg_right.contains("text-anchor=\"end\""));

        let mut cfg_left = cfg.clone();
        cfg_left.title.text = Some("Left Title".to_string());
        cfg_left.title.align = TitleAlign::Left;
        let svg_left = build_svg(
            &[line],
            &cfg_left,
            "#FFFFFF",
            None,
            0,
            cfg_left.title.text.as_deref(),
            &cfg_left.font.family,
        );
        assert!(svg_left.contains("text-anchor=\"start\""));
    }

    #[test]
    fn expand_box_invalid_length_defaults() {
        let out = expand_box(&[1.0, 2.0, 3.0]);
        assert_eq!(out, [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn line_width_cells_handles_tab_at_start() {
        let line = Line {
            spans: vec![Span {
                text: "\t".to_string(),
                style: TextStyle::default(),
            }],
        };
        assert_eq!(line_width_cells(&line), DEFAULT_TAB_WIDTH);
        let width_px = span_width_px("\t", 8.0);
        assert_eq!(width_px, DEFAULT_TAB_WIDTH as f32 * 8.0);
    }

    #[test]
    fn render_svg_with_ansi_wrap() {
        let mut cfg = Config::default();
        cfg.font.family = "Test".to_string();
        cfg.wrap = 4;
        let input = "\x1b[31mhello world\x1b[0m";
        let request = RenderRequest {
            input: InputSource::Text(input.to_string()),
            config: cfg,
            format: OutputFormat::Svg,
        };
        let result = render(&request).expect("render svg");
        let svg = String::from_utf8(result.bytes).expect("utf8");
        assert!(svg.contains("<svg"));
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
        let path =
            std::env::temp_dir().join(format!("cryosnap-font-test-{}.ttf", std::process::id()));
        std::fs::write(&path, b"font").expect("write temp font");
        cfg.font.family = "Custom".to_string();
        cfg.font.file = Some(path.to_string_lossy().to_string());
        let css = svg_font_face_css(&cfg).expect("css");
        assert!(css.is_some());

        cfg.font.file = None;
        let css = svg_font_face_css(&cfg).expect("css");
        assert!(css.is_none());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn svg_font_face_css_woff_format() {
        let mut cfg = Config::default();
        let temp = temp_dir("woff-font");
        let path = temp.join("test.woff");
        std::fs::write(&path, b"font").expect("write temp font");
        cfg.font.family = "Custom".to_string();
        cfg.font.file = Some(path.to_string_lossy().to_string());
        let css = svg_font_face_css(&cfg).expect("css").expect("some");
        assert!(css.contains("format('woff')"));
        assert!(css.contains("font/woff"));
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn svg_font_face_css_unknown_ext_defaults() {
        let mut cfg = Config::default();
        let temp = temp_dir("font-unknown");
        let path = temp.join("test.abc");
        std::fs::write(&path, b"font").expect("write temp font");
        cfg.font.family = "Custom".to_string();
        cfg.font.file = Some(path.to_string_lossy().to_string());
        let css = svg_font_face_css(&cfg).expect("css").expect("some");
        assert!(css.contains("format('truetype')"));
        assert!(css.contains("font/ttf"));
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir_all(temp);
    }

    #[test]
    fn normalize_repo_key_strips_punct() {
        assert_eq!(normalize_repo_key("N'Ko"), "nko");
        assert_eq!(normalize_repo_key("Sign-Writing"), "signwriting");
        assert_eq!(normalize_repo_key("Old Hungarian"), "oldhungarian");
    }

    #[test]
    fn script_repo_key_maps_latin() {
        let mut repos = HashMap::new();
        repos.insert(
            "latin-greek-cyrillic".to_string(),
            NotofontsRepo {
                families: HashMap::new(),
            },
        );
        let state = NotofontsState(repos);
        let index = build_repo_key_index(&state);
        assert_eq!(
            script_repo_key(Script::Latin, &index).as_deref(),
            Some("latin-greek-cyrillic")
        );
    }

    #[test]
    fn tag_from_release_url_extracts() {
        let url = "https://github.com/notofonts/devanagari/releases/tag/NotoSansDevanagari-v2.006";
        assert_eq!(
            tag_from_release_url(url).as_deref(),
            Some("NotoSansDevanagari-v2.006")
        );
    }

    #[test]
    fn repo_from_release_url_extracts() {
        let url = "https://github.com/notofonts/devanagari/releases/tag/NotoSansDevanagari-v2.006";
        assert_eq!(
            repo_from_release_url(url).as_deref(),
            Some("notofonts/devanagari")
        );
    }

    #[test]
    fn choose_family_prefers_sans() {
        let mut families = HashMap::new();
        families.insert(
            "Noto Sans Devanagari".to_string(),
            NotofontsFamily {
                latest_release: Some(NotofontsRelease {
                    url: "https://github.com/notofonts/devanagari/releases/tag/NotoSansDevanagari-v2.006".to_string(),
                }),
                files: Vec::new(),
            },
        );
        families.insert(
            "Noto Serif Devanagari".to_string(),
            NotofontsFamily {
                latest_release: Some(NotofontsRelease {
                    url: "https://github.com/notofonts/devanagari/releases/tag/NotoSerifDevanagari-v2.006".to_string(),
                }),
                files: Vec::new(),
            },
        );
        let picked = choose_family_name(&families, FontStylePreference::Sans).expect("family");
        assert_eq!(picked, "Noto Sans Devanagari");
    }

    #[test]
    fn choose_family_avoids_supplement_when_possible() {
        let mut families = HashMap::new();
        families.insert(
            "Noto Sans Tamil".to_string(),
            NotofontsFamily {
                latest_release: Some(NotofontsRelease {
                    url: "https://github.com/notofonts/tamil/releases/tag/NotoSansTamil-v2.006"
                        .to_string(),
                }),
                files: Vec::new(),
            },
        );
        families.insert(
            "Noto Sans Tamil Supplement".to_string(),
            NotofontsFamily {
                latest_release: Some(NotofontsRelease {
                    url: "https://github.com/notofonts/tamil/releases/tag/NotoSansTamilSupplement-v2.006".to_string(),
                }),
                files: Vec::new(),
            },
        );
        let picked = choose_family_name(&families, FontStylePreference::Sans).expect("family");
        assert_eq!(picked, "Noto Sans Tamil");
    }

    #[test]
    fn choose_family_avoids_looped_when_possible() {
        let mut families = HashMap::new();
        families.insert(
            "Noto Sans Thai".to_string(),
            NotofontsFamily {
                latest_release: Some(NotofontsRelease {
                    url: "https://github.com/notofonts/thai/releases/tag/NotoSansThai-v2.006"
                        .to_string(),
                }),
                files: Vec::new(),
            },
        );
        families.insert(
            "Noto Sans Thai Looped".to_string(),
            NotofontsFamily {
                latest_release: Some(NotofontsRelease {
                    url: "https://github.com/notofonts/thai/releases/tag/NotoSansThaiLooped-v2.006"
                        .to_string(),
                }),
                files: Vec::new(),
            },
        );
        let picked = choose_family_name(&families, FontStylePreference::Sans).expect("family");
        assert_eq!(picked, "Noto Sans Thai");
    }

    #[test]
    fn score_font_path_prefers_regular() {
        let regular = score_font_path("fonts/NotoSans/ttf/NotoSans-Regular.ttf").unwrap();
        let bold = score_font_path("fonts/NotoSans/ttf/NotoSans-Bold.ttf").unwrap();
        assert!(regular > bold);
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
    fn quantize_pixmap_to_png_basic() {
        let mut pixmap = tiny_skia::Pixmap::new(2, 1).expect("pixmap");
        pixmap.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
        let png = quantize_pixmap_to_png(&pixmap, &PngOptions::default()).expect("quantize");
        assert!(png.starts_with(b"\x89PNG"));
    }

    #[test]
    fn render_svg_includes_nf_fallback_when_needed() {
        let cfg = Config::default();
        let request = RenderRequest {
            input: InputSource::Text("\u{f121}".to_string()),
            config: cfg,
            format: OutputFormat::Svg,
        };
        let result = render(&request).expect("render svg");
        let svg = String::from_utf8(result.bytes).expect("utf8");
        assert!(svg.contains("Symbols Nerd Font Mono"));
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

    #[test]
    fn parse_cjk_region_from_locale_variants() {
        assert_eq!(
            parse_cjk_region_from_locale("zh_CN.UTF-8"),
            Some(CjkRegion::Sc)
        );
        assert_eq!(parse_cjk_region_from_locale("zh_TW"), Some(CjkRegion::Tc));
        assert_eq!(parse_cjk_region_from_locale("zh-Hant"), Some(CjkRegion::Tc));
        assert_eq!(parse_cjk_region_from_locale("zh_Hans"), Some(CjkRegion::Sc));
        assert_eq!(parse_cjk_region_from_locale("zh_HK"), Some(CjkRegion::Hk));
        assert_eq!(parse_cjk_region_from_locale("ja_JP"), Some(CjkRegion::Jp));
        assert_eq!(parse_cjk_region_from_locale("ko_KR"), Some(CjkRegion::Kr));
        assert_eq!(parse_cjk_region_from_locale("en_US"), None);
        assert_eq!(parse_cjk_region_from_locale(""), None);
    }

    #[test]
    fn locale_cjk_region_returns_none_without_env() {
        let _lock = env_lock().lock().expect("lock");
        let prev_lc_all = std::env::var("LC_ALL").ok();
        let prev_lc_ctype = std::env::var("LC_CTYPE").ok();
        let prev_lang = std::env::var("LANG").ok();

        std::env::remove_var("LC_ALL");
        std::env::remove_var("LC_CTYPE");
        std::env::remove_var("LANG");
        assert_eq!(locale_cjk_region(), None);

        restore_env_var("LC_ALL", prev_lc_all);
        restore_env_var("LC_CTYPE", prev_lc_ctype);
        restore_env_var("LANG", prev_lang);
    }

    #[test]
    fn script_repo_key_skips_common_and_cjk() {
        let index = HashMap::new();
        assert!(script_repo_key(Script::Common, &index).is_none());
        assert!(script_repo_key(Script::Han, &index).is_none());
    }

    #[test]
    fn collect_cjk_regions_includes_scripts_and_config() {
        let mut cfg = Config::default();
        cfg.font.cjk_region = CjkRegion::Hk;
        let mut needs = FontFallbackNeeds::default();
        needs.needs_cjk = true;
        needs.scripts.insert(Script::Hiragana);
        needs.scripts.insert(Script::Hangul);
        needs.scripts.insert(Script::Bopomofo);
        needs.scripts.insert(Script::Han);
        let regions = collect_cjk_regions(&cfg, &needs);
        assert_eq!(
            regions,
            vec![CjkRegion::Jp, CjkRegion::Kr, CjkRegion::Tc, CjkRegion::Hk]
        );
    }

    #[test]
    fn cjk_region_helpers_cover_all_variants() {
        assert_eq!(cjk_region_families(CjkRegion::Sc), AUTO_FALLBACK_CJK_SC);
        assert_eq!(cjk_region_families(CjkRegion::Tc), AUTO_FALLBACK_CJK_TC);
        assert_eq!(cjk_region_families(CjkRegion::Hk), AUTO_FALLBACK_CJK_HK);
        assert_eq!(cjk_region_families(CjkRegion::Jp), AUTO_FALLBACK_CJK_JP);
        assert_eq!(cjk_region_families(CjkRegion::Kr), AUTO_FALLBACK_CJK_KR);
        assert_eq!(cjk_region_families(CjkRegion::Auto), AUTO_FALLBACK_CJK_SC);

        assert_eq!(cjk_region_urls(CjkRegion::Sc), NOTO_CJK_SC_URLS);
        assert_eq!(cjk_region_urls(CjkRegion::Tc), NOTO_CJK_TC_URLS);
        assert_eq!(cjk_region_urls(CjkRegion::Hk), NOTO_CJK_HK_URLS);
        assert_eq!(cjk_region_urls(CjkRegion::Jp), NOTO_CJK_JP_URLS);
        assert_eq!(cjk_region_urls(CjkRegion::Kr), NOTO_CJK_KR_URLS);
        assert_eq!(cjk_region_urls(CjkRegion::Auto), NOTO_CJK_SC_URLS);

        assert_eq!(
            cjk_region_filename(CjkRegion::Sc),
            "NotoSansCJKsc-Regular.otf"
        );
        assert_eq!(
            cjk_region_filename(CjkRegion::Tc),
            "NotoSansCJKtc-Regular.otf"
        );
        assert_eq!(
            cjk_region_filename(CjkRegion::Hk),
            "NotoSansCJKhk-Regular.otf"
        );
        assert_eq!(
            cjk_region_filename(CjkRegion::Jp),
            "NotoSansCJKjp-Regular.otf"
        );
        assert_eq!(
            cjk_region_filename(CjkRegion::Kr),
            "NotoSansCJKkr-Regular.otf"
        );
        assert_eq!(
            cjk_region_filename(CjkRegion::Auto),
            "NotoSansCJKsc-Regular.otf"
        );
    }

    #[test]
    fn locale_cjk_region_prefers_env_order() {
        let _lock = env_lock().lock().expect("lock");
        let prev_lc_all = std::env::var("LC_ALL").ok();
        let prev_lc_ctype = std::env::var("LC_CTYPE").ok();
        let prev_lang = std::env::var("LANG").ok();

        std::env::set_var("LC_ALL", "zh_TW.UTF-8");
        std::env::set_var("LC_CTYPE", "ja_JP.UTF-8");
        std::env::set_var("LANG", "zh_CN.UTF-8");
        assert_eq!(locale_cjk_region(), Some(CjkRegion::Tc));

        restore_env_var("LC_ALL", prev_lc_all);
        restore_env_var("LC_CTYPE", prev_lc_ctype);
        restore_env_var("LANG", prev_lang);
    }

    #[test]
    fn expand_home_dir_expands_tilde() {
        let _lock = env_lock().lock().expect("lock");
        let temp = temp_dir("home");
        let prev_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &temp);

        assert_eq!(expand_home_dir("~"), Some(temp.clone()));
        assert_eq!(expand_home_dir("~/fonts"), Some(temp.join("fonts")));
        assert_eq!(
            expand_home_dir("/tmp/fonts"),
            Some(std::path::PathBuf::from("/tmp/fonts"))
        );

        restore_env_var("HOME", prev_home);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn parse_font_dir_list_ignores_empty() {
        let _lock = env_lock().lock().expect("lock");
        let temp = temp_dir("font-dirs");
        let prev_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &temp);
        let out = parse_font_dir_list(" ,~/fonts, /tmp ,,").expect("parse");
        assert_eq!(out.len(), 2);
        assert_eq!(out[0], temp.join("fonts"));
        assert_eq!(out[1], std::path::PathBuf::from("/tmp"));
        restore_env_var("HOME", prev_home);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn resolve_font_dirs_prefers_env() {
        let _lock = env_lock().lock().expect("lock");
        let prev_env = std::env::var("CRYOSNAP_FONT_DIRS").ok();
        std::env::set_var("CRYOSNAP_FONT_DIRS", "/a,/b");
        let cfg = Config::default();
        let dirs = resolve_font_dirs(&cfg).expect("dirs");
        assert_eq!(
            dirs,
            vec![
                std::path::PathBuf::from("/a"),
                std::path::PathBuf::from("/b")
            ]
        );
        restore_env_var("CRYOSNAP_FONT_DIRS", prev_env);
    }

    #[test]
    fn resolve_font_dirs_uses_config_dirs() {
        let _lock = env_lock().lock().expect("lock");
        let prev_env = std::env::var("CRYOSNAP_FONT_DIRS").ok();
        std::env::remove_var("CRYOSNAP_FONT_DIRS");
        let temp = temp_dir("font-dirs-config");
        let prev_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", &temp);
        let mut cfg = Config::default();
        cfg.font.dirs = vec!["~/fonts".to_string(), "/opt/fonts".to_string()];
        let dirs = resolve_font_dirs(&cfg).expect("dirs");
        assert_eq!(dirs[0], temp.join("fonts"));
        assert_eq!(dirs[1], std::path::PathBuf::from("/opt/fonts"));
        restore_env_var("HOME", prev_home);
        restore_env_var("CRYOSNAP_FONT_DIRS", prev_env);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn needs_system_fonts_respects_modes() {
        let mut cfg = Config::default();
        let mut app_families = HashSet::new();
        let families = vec!["monospace".to_string()];

        cfg.font.system_fallback = FontSystemFallback::Never;
        assert!(!needs_system_fonts(&cfg, &app_families, &families));

        cfg.font.system_fallback = FontSystemFallback::Always;
        assert!(needs_system_fonts(&cfg, &app_families, &families));

        cfg.font.system_fallback = FontSystemFallback::Auto;
        cfg.font.family = "Custom".to_string();
        assert!(needs_system_fonts(&cfg, &app_families, &families));

        cfg.font.file = Some("embedded.ttf".to_string());
        cfg.font.family = "Embedded".to_string();
        app_families.insert(family_key("Embedded"));
        let families = vec!["Embedded".to_string(), "Missing".to_string()];
        assert!(needs_system_fonts(&cfg, &app_families, &families));
    }

    #[test]
    fn build_fontdb_loads_font_file_and_system_fonts() {
        let temp = temp_dir("fontdb");
        let font_path = copy_asset_font("JetBrainsMono-Regular.ttf", &temp);
        let mut cfg = Config::default();
        cfg.font.file = Some(font_path.to_string_lossy().to_string());
        cfg.font.dirs = vec![temp.to_string_lossy().to_string()];

        let fontdb = build_fontdb(&cfg, true).expect("fontdb");
        assert!(fontdb.faces().next().is_some());
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn default_font_dir_uses_cryosnap_home() {
        let _lock = env_lock().lock().expect("lock");
        let temp = temp_dir("cryosnap-home");
        let prev_home = std::env::var("CRYOSNAP_HOME").ok();
        std::env::set_var("CRYOSNAP_HOME", &temp);
        let dir = default_font_dir().expect("font dir");
        assert_eq!(dir, temp.join("fonts"));
        restore_env_var("CRYOSNAP_HOME", prev_home);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn github_proxy_candidates_from_env() {
        let _lock = env_lock().lock().expect("lock");
        let prev_env = std::env::var("CRYOSNAP_GITHUB_PROXY").ok();
        std::env::set_var("CRYOSNAP_GITHUB_PROXY", "https://a, https://b");
        let candidates = github_proxy_candidates();
        assert_eq!(candidates, vec!["https://a", "https://b"]);
        restore_env_var("CRYOSNAP_GITHUB_PROXY", prev_env);
    }

    #[test]
    fn apply_github_proxy_adds_slash() {
        let url = "https://github.com/notofonts/devanagari";
        assert_eq!(
            apply_github_proxy(url, "https://proxy.example"),
            "https://proxy.example/https://github.com/notofonts/devanagari"
        );
        assert_eq!(
            apply_github_proxy(url, "https://proxy.example/"),
            "https://proxy.example/https://github.com/notofonts/devanagari"
        );
    }

    #[test]
    fn build_github_candidates_dedupes() {
        let _lock = env_lock().lock().expect("lock");
        let prev_env = std::env::var("CRYOSNAP_GITHUB_PROXY").ok();
        std::env::set_var("CRYOSNAP_GITHUB_PROXY", "https://a, https://a, https://b");
        let candidates = build_github_candidates();
        assert_eq!(
            candidates,
            vec![
                None,
                Some("https://a".to_string()),
                Some("https://b".to_string())
            ]
        );
        restore_env_var("CRYOSNAP_GITHUB_PROXY", prev_env);
    }

    #[test]
    fn looks_like_json_detects_payloads() {
        assert!(looks_like_json(br#"{ "ok": true }"#));
        assert!(looks_like_json(br#"   [1,2,3]"#));
        assert!(!looks_like_json(br#"<html></html>"#));
    }

    #[test]
    fn fetch_bytes_with_cache_uses_etag() {
        let _lock = env_lock().lock().expect("lock");
        let temp = temp_dir("cache");
        let prev_home = std::env::var("CRYOSNAP_HOME").ok();
        std::env::set_var("CRYOSNAP_HOME", &temp);
        let body = br#"{ "ok": true }"#.to_vec();
        let server = spawn_etag_server(body.clone(), "W/\"abc\"", 2);
        let url = server.url("/state.json");
        let first = fetch_bytes_with_cache(&url, "state.json", false).expect("fetch");
        assert_eq!(first, body);
        let second = fetch_bytes_with_cache(&url, "state.json", false).expect("fetch");
        assert_eq!(second, body);
        server.join();
        restore_env_var("CRYOSNAP_HOME", prev_home);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn download_url_with_etag_not_modified() {
        let temp = temp_dir("etag");
        let body = b"fontdata".to_vec();
        let server = spawn_sequence_server(vec![
            ServerResponse {
                status: 200,
                headers: vec![("ETag".to_string(), "\"etag1\"".to_string())],
                body: body.clone(),
            },
            ServerResponse {
                status: 304,
                headers: Vec::new(),
                body: Vec::new(),
            },
        ]);
        let url = server.url("/font.ttf");
        let target = temp.join("font.ttf");
        let downloaded = download_url_with_etag(&url, &target, false).expect("download");
        assert!(downloaded);
        let before = std::fs::read(&target).expect("read");
        let _ = download_url_with_etag(&url, &target, false).expect("download");
        let after = std::fs::read(&target).expect("read");
        assert_eq!(after, before);
        assert_eq!(after, body);
        server.join();
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn verify_sha256_handles_expected() {
        let temp = temp_dir("sha");
        let path = temp.join("data.bin");
        std::fs::write(&path, b"hello").expect("write");
        let digest = sha256_hex(&path).expect("sha");
        assert!(verify_sha256(&path, &digest).expect("verify"));
        assert!(verify_sha256(&path, "").expect("verify"));
        assert!(!verify_sha256(&path, "deadbeef").expect("verify"));
        let missing = temp.join("missing.bin");
        assert!(!verify_sha256(&missing, &digest).expect("verify"));
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn validate_zip_and_extract_entry() {
        let temp = temp_dir("zip");
        let zip_path = temp.join("test.zip");
        let out_path = temp.join("out.txt");
        {
            let file = std::fs::File::create(&zip_path).expect("zip");
            let mut writer = zip::ZipWriter::new(file);
            let options = zip::write::FileOptions::default();
            writer.start_file("data.txt", options).expect("start");
            writer.write_all(b"hello").expect("write");
            writer.finish().expect("finish");
        }
        validate_zip_archive(&zip_path).expect("validate");
        extract_zip_entry(&zip_path, "data.txt", &out_path).expect("extract");
        let out = std::fs::read_to_string(&out_path).expect("read");
        assert_eq!(out, "hello");

        let bad_path = temp.join("bad.zip");
        std::fs::write(&bad_path, b"notzip").expect("write");
        assert!(validate_zip_archive(&bad_path).is_err());
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn parse_ansi_extended_colors_and_tabs() {
        let input = "A\x1b[1;38;5;196mB\x1b[0m\tC";
        let lines = parse_ansi(input);
        assert_eq!(lines.len(), 1);
        let spans = &lines[0].spans;
        assert!(spans.iter().any(|s| s.text.contains('B') && s.style.bold));
        assert!(spans
            .iter()
            .any(|s| s.text.contains('B') && s.style.fg == Some("#FF0000".to_string())));
        assert!(spans.iter().any(|s| s.text.contains('C')));
    }

    #[test]
    fn parse_ansi_rgb_color_and_reset() {
        let input = "A\x1b[38;2;1;2;3mB\x1b[0mC";
        let lines = parse_ansi(input);
        let spans = &lines[0].spans;
        assert!(spans
            .iter()
            .any(|s| s.text == "B" && s.style.fg == Some("#010203".to_string())));
        assert!(spans.iter().any(|s| s.text == "C" && s.style.fg.is_none()));
    }

    #[test]
    fn ansi_color_fallbacks() {
        assert_eq!(ansi_color(200), "#C5C8C6");
        assert_eq!(xterm_color(7), ansi_color(7));
        assert_eq!(xterm_color(232), "#080808");
        assert_eq!(xterm_color(231), "#FFFFFF");
        assert_eq!(xterm_color(21), "#0000FF");
    }

    #[test]
    fn decode_png_grayscale_variants() {
        let gray = [128u8, 64];
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 2, 1);
            encoder.set_color(png::ColorType::Grayscale);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("header");
            writer.write_image_data(&gray).expect("write");
        }
        let (decoded, width, height) = decode_png_rgba(&bytes).expect("decode");
        assert_eq!(width, 2);
        assert_eq!(height, 1);
        assert_eq!(decoded, vec![128, 128, 128, 255, 64, 64, 64, 255]);

        let graya = [10u8, 200];
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
            encoder.set_color(png::ColorType::GrayscaleAlpha);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("header");
            writer.write_image_data(&graya).expect("write");
        }
        let (decoded, _, _) = decode_png_rgba(&bytes).expect("decode");
        assert_eq!(decoded, vec![10, 10, 10, 200]);
    }

    #[test]
    fn decode_png_indexed_expands() {
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
            encoder.set_color(png::ColorType::Indexed);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_palette(vec![0, 0, 0]);
            let mut writer = encoder.write_header().expect("header");
            writer.write_image_data(&[0]).expect("write");
        }
        let (decoded, width, height) = decode_png_rgba(&bytes).expect("decode");
        assert_eq!(width, 1);
        assert_eq!(height, 1);
        assert_eq!(decoded, vec![0, 0, 0, 255]);
    }

    #[test]
    fn quantize_rgba_to_png_rejects_invalid_len() {
        let rgba = [0u8, 0, 0];
        let err = quantize_rgba_to_png(&rgba, 2, 2, &PngOptions::default()).unwrap_err();
        assert!(err.to_string().contains("invalid rgba"));
    }

    #[test]
    fn optimize_png_handles_strip_options() {
        let rgba = [0u8, 0, 0, 255];
        let mut bytes = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().expect("header");
            writer.write_image_data(&rgba).expect("write");
        }
        for strip in [PngStrip::None, PngStrip::Safe, PngStrip::All] {
            let mut cfg = PngOptions::default();
            cfg.optimize = true;
            cfg.strip = strip;
            let out = optimize_png(bytes.clone(), &cfg).expect("optimize");
            assert!(out.starts_with(b"\x89PNG"));
        }
    }

    #[test]
    fn cut_text_window_defaults_and_negative() {
        let input = "a\nb\nc";
        let out = cut_text(input, &[]);
        assert_eq!(out.text, input);
        assert_eq!(out.start, 0);

        let out = cut_text(input, &[0]);
        assert_eq!(out.text, input);
        assert_eq!(out.start, 0);

        let out = cut_text(input, &[0, -1]);
        assert_eq!(out.text, input);
        assert_eq!(out.start, 0);

        let out = cut_text(input, &[-1]);
        assert_eq!(out.text, "c");
        assert_eq!(out.start, 2);
    }

    #[test]
    fn cut_text_window_out_of_range_returns_empty() {
        let input = "a\nb\nc";
        let out = cut_text(input, &[10]);
        assert!(out.text.is_empty());
        assert_eq!(out.start, 3);
    }

    #[test]
    fn split_line_by_width_empty_line_returns_default() {
        let line = Line::default();
        let out = split_line_by_width(&line, 4);
        assert_eq!(out.len(), 1);
        assert!(out[0].spans.is_empty());
    }

    #[test]
    fn split_line_by_width_wraps_spans() {
        let line = Line {
            spans: vec![Span {
                text: "abcd".to_string(),
                style: TextStyle::default(),
            }],
        };
        let out = split_line_by_width(&line, 2);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].spans[0].text, "ab");
        assert_eq!(out[1].spans[0].text, "cd");
    }

    #[test]
    fn highlight_code_with_language_and_path() {
        let temp = temp_dir("highlight");
        let path = temp.join("sample.rs");
        let text = "fn main() {\n    println!(\"hi\");\n}\n";
        std::fs::write(&path, text).expect("write");

        let (lines, fg) = highlight_code(text, Some(&path), None, "charm").expect("highlight");
        assert!(!lines.is_empty());
        assert!(fg.starts_with('#'));

        let (lines, fg) = highlight_code(text, None, Some("rs"), "charm").expect("highlight");
        assert!(!lines.is_empty());
        assert!(fg.starts_with('#'));

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn parse_ansi_styles_and_resets() {
        let input = "\x1b[1;3;4;9;38;2;1;2;3;48;5;120mX\x1b[22;23;24;29;39;49mY";
        let lines = parse_ansi(input);
        let spans = &lines[0].spans;
        let span_x = spans.iter().find(|s| s.text.contains('X')).expect("X");
        assert!(span_x.style.bold);
        assert!(span_x.style.italic);
        assert!(span_x.style.underline);
        assert!(span_x.style.strike);
        assert_eq!(span_x.style.fg, Some("#010203".to_string()));
        assert!(span_x.style.bg.is_some());
        let span_y = spans.iter().find(|s| s.text.contains('Y')).expect("Y");
        assert!(!span_y.style.bold);
        assert!(!span_y.style.italic);
        assert!(!span_y.style.underline);
        assert!(!span_y.style.strike);
        assert!(span_y.style.fg.is_none());
        assert!(span_y.style.bg.is_none());
    }

    #[test]
    fn score_family_name_variants() {
        let sans = score_family_name("Noto Sans Kufi", FontStylePreference::Sans);
        let sans_ui = score_family_name("Noto Sans UI", FontStylePreference::Sans);
        let sans_supp = score_family_name("Noto Sans Supplement", FontStylePreference::Sans);
        assert!(sans > sans_ui);
        assert!(sans > sans_supp);

        let serif = score_family_name("Noto Serif Naskh", FontStylePreference::Serif);
        let serif_plain = score_family_name("Noto Serif", FontStylePreference::Serif);
        assert!(serif > serif_plain);
    }

    #[test]
    fn score_font_path_variants() {
        let regular = score_font_path("fonts/NotoSans/ttf/NotoSans-Regular.ttf").unwrap();
        let italic = score_font_path("fonts/NotoSans/ttf/NotoSans-Italic.ttf").unwrap();
        let full = score_font_path("fonts/NotoSans/full/NotoSans-Regular.ttf").unwrap();
        let hinted = score_font_path("fonts/NotoSans/hinted/NotoSans-Regular.ttf").unwrap();
        let otf = score_font_path("fonts/NotoSans/otf/NotoSans-Regular.otf").unwrap();
        assert!(regular > italic);
        assert!(full > hinted);
        assert!(regular > otf);
        assert!(score_font_path("fonts/NotoSans/README.txt").is_none());
    }

    #[test]
    fn pick_best_font_file_prefers_regular() {
        let files = vec![
            "fonts/NotoSans/ttf/NotoSans-Italic.ttf".to_string(),
            "fonts/NotoSans/full/NotoSans-Regular.ttf".to_string(),
        ];
        let best = pick_best_font_file(&files).expect("best");
        assert!(best.contains("Regular"));
    }

    #[test]
    fn resolve_script_font_plan_files_repo() {
        let _lock = state_lock().lock().expect("lock");
        let mut families = HashMap::new();
        families.insert(
            "Noto Sans Devanagari".to_string(),
            NotofontsFamily {
                latest_release: None,
                files: vec![
                    "fonts/NotoSansDevanagari/ttf/NotoSansDevanagari-Regular.ttf".to_string(),
                ],
            },
        );
        let mut repos = HashMap::new();
        repos.insert("devanagari".to_string(), NotofontsRepo { families });
        let state = Arc::new(NotofontsState(repos));
        let prev = set_notofonts_state(Some(state));

        let mut needs = FontFallbackNeeds::default();
        needs.needs_unicode = true;
        needs.scripts.insert(Script::Devanagari);
        let plan = resolve_script_font_plan(&Config::default(), &needs).expect("plan");
        assert_eq!(plan.families, vec!["Noto Sans Devanagari".to_string()]);
        assert_eq!(plan.downloads.len(), 1);
        let download = &plan.downloads[0];
        assert_eq!(download.repo, NOTOFONTS_FILES_REPO);
        assert!(download.tag.is_none());

        set_notofonts_state(prev);
    }

    #[test]
    fn resolve_script_font_plan_release_repo() {
        let _lock = state_lock().lock().expect("lock");
        let mut families = HashMap::new();
        families.insert(
            "Noto Sans Devanagari".to_string(),
            NotofontsFamily {
                latest_release: Some(NotofontsRelease {
                    url: "https://github.com/notofonts/devanagari/releases/tag/NotoSansDevanagari-v2.006".to_string(),
                }),
                files: vec!["NotoSansDevanagari-Regular.ttf".to_string()],
            },
        );
        families.insert(
            "Noto Serif Devanagari".to_string(),
            NotofontsFamily {
                latest_release: Some(NotofontsRelease {
                    url: "https://github.com/notofonts/devanagari/releases/tag/NotoSerifDevanagari-v2.006".to_string(),
                }),
                files: vec!["NotoSerifDevanagari-Regular.ttf".to_string()],
            },
        );
        let mut repos = HashMap::new();
        repos.insert("devanagari".to_string(), NotofontsRepo { families });
        let state = Arc::new(NotofontsState(repos));
        let prev = set_notofonts_state(Some(state));

        let mut config = Config::default();
        config.font.family = "serif".to_string();
        let mut needs = FontFallbackNeeds::default();
        needs.needs_unicode = true;
        needs.scripts.insert(Script::Devanagari);
        let plan = resolve_script_font_plan(&config, &needs).expect("plan");
        assert_eq!(plan.families, vec!["Noto Serif Devanagari".to_string()]);
        let download = &plan.downloads[0];
        assert_eq!(download.repo, "notofonts/devanagari");
        assert_eq!(download.tag.as_deref(), Some("NotoSerifDevanagari-v2.006"));

        set_notofonts_state(prev);
    }

    #[test]
    fn resolve_script_font_plan_dedupes_scripts() {
        let _lock = state_lock().lock().expect("lock");
        let mut families = HashMap::new();
        families.insert(
            "Noto Sans".to_string(),
            NotofontsFamily {
                latest_release: None,
                files: vec!["fonts/NotoSans/ttf/NotoSans-Regular.ttf".to_string()],
            },
        );
        let mut repos = HashMap::new();
        repos.insert(
            "latin-greek-cyrillic".to_string(),
            NotofontsRepo { families },
        );
        let state = Arc::new(NotofontsState(repos));
        let prev = set_notofonts_state(Some(state));

        let mut needs = FontFallbackNeeds::default();
        needs.needs_unicode = true;
        needs.scripts.insert(Script::Latin);
        needs.scripts.insert(Script::Greek);
        needs.scripts.insert(Script::Unknown);

        let plan = resolve_script_font_plan(&Config::default(), &needs).expect("plan");
        assert_eq!(plan.families.len(), 1);
        assert_eq!(plan.downloads.len(), 1);

        set_notofonts_state(prev);
    }

    #[test]
    fn ensure_fonts_available_respects_auto_download_env() {
        let _lock = env_lock().lock().expect("lock");
        let prev = std::env::var("CRYOSNAP_FONT_AUTO_DOWNLOAD").ok();
        std::env::set_var("CRYOSNAP_FONT_AUTO_DOWNLOAD", "0");
        let cfg = Config::default();
        let mut needs = FontFallbackNeeds::default();
        needs.needs_nf = true;
        let plan = ScriptFontPlan::default();
        ensure_fonts_available(&cfg, &needs, &plan).expect("ensure");
        restore_env_var("CRYOSNAP_FONT_AUTO_DOWNLOAD", prev);
    }

    #[test]
    fn ensure_fonts_available_skips_downloads_when_fonts_present() {
        let _lock = env_lock().lock().expect("lock");
        let temp = temp_dir("fonts-present");
        let _font = copy_asset_font("SymbolsNerdFontMono-Regular.ttf", &temp);
        let prev_force = std::env::var("CRYOSNAP_FONT_FORCE_UPDATE").ok();
        std::env::set_var("CRYOSNAP_FONT_FORCE_UPDATE", "1");

        let mut cfg = Config::default();
        cfg.font.dirs = vec![temp.to_string_lossy().to_string()];
        cfg.font.system_fallback = FontSystemFallback::Never;
        cfg.font.auto_download = true;

        let mut needs = FontFallbackNeeds::default();
        needs.needs_nf = true;
        let plan = ScriptFontPlan {
            downloads: vec![ScriptDownload {
                family: "Symbols Nerd Font Mono".to_string(),
                repo: "notofonts/devanagari".to_string(),
                file_path: "fonts/NotoSansDevanagari/ttf/NotoSansDevanagari-Regular.ttf"
                    .to_string(),
                filename: "missing.ttf".to_string(),
                tag: None,
            }],
            families: Vec::new(),
        };
        ensure_fonts_available(&cfg, &needs, &plan).expect("ensure");

        restore_env_var("CRYOSNAP_FONT_FORCE_UPDATE", prev_force);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn fetch_bytes_with_cache_falls_back_to_proxy() {
        let _lock = env_lock().lock().expect("lock");
        let temp = temp_dir("cache-proxy");
        let prev_home = std::env::var("CRYOSNAP_HOME").ok();
        let prev_proxy = std::env::var("CRYOSNAP_GITHUB_PROXY").ok();
        std::env::set_var("CRYOSNAP_HOME", &temp);

        let server = spawn_sequence_server(vec![
            ServerResponse {
                status: 200,
                headers: vec![("ETag".to_string(), "W/\"bad\"".to_string())],
                body: b"oops".to_vec(),
            },
            ServerResponse {
                status: 200,
                headers: vec![("ETag".to_string(), "W/\"good\"".to_string())],
                body: br#"{ "ok": true }"#.to_vec(),
            },
        ]);
        std::env::set_var("CRYOSNAP_GITHUB_PROXY", format!("http://{}", server.addr));
        let url = server.url("/state.json");
        let bytes = fetch_bytes_with_cache(&url, "state.json", false).expect("fetch");
        assert!(bytes.starts_with(b"{"));

        server.join();
        restore_env_var("CRYOSNAP_HOME", prev_home);
        restore_env_var("CRYOSNAP_GITHUB_PROXY", prev_proxy);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn fetch_bytes_with_cache_refreshes_invalid_cached_json() {
        let _lock = env_lock().lock().expect("lock");
        let temp = temp_dir("cache-invalid-json");
        let prev_home = std::env::var("CRYOSNAP_HOME").ok();
        let prev_proxy = std::env::var("CRYOSNAP_GITHUB_PROXY").ok();
        std::env::set_var("CRYOSNAP_HOME", &temp);

        let cache_dir = temp.join("cache");
        std::fs::create_dir_all(&cache_dir).expect("cache dir");
        std::fs::write(cache_dir.join("state.json"), b"oops").expect("cache");
        std::fs::write(cache_dir.join("state.json.etag"), "W/\"bad\"").expect("etag");

        let server = spawn_sequence_server(vec![
            ServerResponse {
                status: 304,
                headers: Vec::new(),
                body: Vec::new(),
            },
            ServerResponse {
                status: 200,
                headers: vec![("ETag".to_string(), "W/\"good\"".to_string())],
                body: br#"{ "ok": true }"#.to_vec(),
            },
        ]);
        std::env::set_var("CRYOSNAP_GITHUB_PROXY", format!("http://{}", server.addr));
        let url = server.url("/state.json");
        let bytes = fetch_bytes_with_cache(&url, "state.json", false).expect("fetch");
        let text = std::str::from_utf8(&bytes).expect("utf8");
        assert!(text.contains("\"ok\""));

        server.join();
        restore_env_var("CRYOSNAP_HOME", prev_home);
        restore_env_var("CRYOSNAP_GITHUB_PROXY", prev_proxy);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn encode_indexed_png_with_alpha_palette() {
        let palette = vec![
            imagequant::RGBA::new(0, 0, 0, 255),
            imagequant::RGBA::new(255, 0, 0, 128),
        ];
        let indices = vec![0, 1];
        let png = encode_indexed_png(&palette, &indices, 2, 1).expect("encode");
        assert!(png.starts_with(b"\x89PNG"));
    }

    #[test]
    fn fetch_with_candidates_handles_not_modified() {
        let server = spawn_sequence_server(vec![ServerResponse {
            status: 304,
            headers: Vec::new(),
            body: Vec::new(),
        }]);
        let url = server.url("/state.json");
        let outcome = fetch_with_candidates(&url, &[]).expect("fetch");
        assert!(matches!(outcome, FetchOutcome::NotModified));
        server.join();
    }

    #[test]
    fn download_notofonts_file_skips_when_exists() {
        let temp = temp_dir("notofonts");
        let filename = "dummy.ttf";
        let target = temp.join(filename);
        std::fs::write(&target, b"font").expect("write");
        let download = ScriptDownload {
            family: "Dummy".to_string(),
            repo: "notofonts/devanagari".to_string(),
            file_path: "fonts/NotoSansDevanagari/ttf/NotoSansDevanagari-Regular.ttf".to_string(),
            filename: filename.to_string(),
            tag: None,
        };
        download_notofonts_file(&download, &temp, false).expect("skip");
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[cfg(unix)]
    #[test]
    fn execute_command_parse_error() {
        let err = execute_command("'", Duration::from_millis(1000)).unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
    }

    #[cfg(unix)]
    #[test]
    fn execute_command_missing_binary() {
        let err =
            execute_command("definitely_not_a_cmd_123", Duration::from_millis(1000)).unwrap_err();
        assert!(matches!(err, Error::Render(_)));
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

    struct TestServer {
        addr: String,
        handle: thread::JoinHandle<()>,
    }

    impl TestServer {
        fn url(&self, path: &str) -> String {
            format!("http://{}{}", self.addr, path)
        }

        fn join(self) {
            self.handle.join().expect("server thread");
        }
    }

    fn spawn_etag_server(body: Vec<u8>, etag: &str, expected: usize) -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let body = body.clone();
        let etag = etag.to_string();
        let handle = thread::spawn(move || {
            for _ in 0..expected {
                let (mut stream, _) = listener.accept().expect("accept");
                let request = read_request(&mut stream);
                let match_tag = header_value(&request, "If-None-Match");
                if match_tag.as_deref() == Some(etag.as_str()) {
                    write_response(&mut stream, 304, &[], None);
                } else {
                    let headers = [
                        ("Content-Type", "application/json"),
                        ("ETag", etag.as_str()),
                    ];
                    write_response(&mut stream, 200, &headers, Some(&body));
                }
            }
        });
        TestServer {
            addr: addr.to_string(),
            handle,
        }
    }

    struct ServerResponse {
        status: u16,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    }

    fn spawn_sequence_server(responses: Vec<ServerResponse>) -> TestServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr");
        let handle = thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().expect("accept");
                let _ = read_request(&mut stream);
                let header_refs: Vec<(&str, &str)> = response
                    .headers
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect();
                let body = if response.body.is_empty() {
                    None
                } else {
                    Some(response.body.as_slice())
                };
                write_response(&mut stream, response.status, &header_refs, body);
            }
        });
        TestServer {
            addr: addr.to_string(),
            handle,
        }
    }

    fn read_request(stream: &mut TcpStream) -> String {
        let mut buf = [0u8; 4096];
        let mut data = Vec::new();
        loop {
            let read = stream.read(&mut buf).unwrap_or(0);
            if read == 0 {
                break;
            }
            data.extend_from_slice(&buf[..read]);
            if data.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        String::from_utf8_lossy(&data).to_string()
    }

    fn header_value(request: &str, key: &str) -> Option<String> {
        let key = key.to_ascii_lowercase();
        request.lines().find_map(|line| {
            let trimmed = line.trim_end_matches('\r');
            if trimmed.to_ascii_lowercase().starts_with(&format!("{key}:")) {
                trimmed.splitn(2, ':').nth(1).map(|v| v.trim().to_string())
            } else {
                None
            }
        })
    }

    fn write_response(
        stream: &mut TcpStream,
        status: u16,
        headers: &[(&str, &str)],
        body: Option<&[u8]>,
    ) {
        let status_text = match status {
            200 => "OK",
            304 => "Not Modified",
            400 => "Bad Request",
            _ => "OK",
        };
        let body_len = body.map_or(0, |b| b.len());
        let mut response = format!("HTTP/1.1 {} {}\r\n", status, status_text);
        response.push_str(&format!("Content-Length: {body_len}\r\n"));
        response.push_str("Connection: close\r\n");
        for (k, v) in headers {
            response.push_str(&format!("{k}: {v}\r\n"));
        }
        response.push_str("\r\n");
        stream.write_all(response.as_bytes()).expect("write");
        if let Some(body) = body {
            stream.write_all(body).expect("write body");
        }
    }

    fn restore_env_var(key: &str, value: Option<String>) {
        match value {
            Some(val) => std::env::set_var(key, val),
            None => std::env::remove_var(key),
        }
    }

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!("cryosnap-{}-{}-{}", prefix, std::process::id(), id));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create dir");
        path
    }

    fn asset_path(name: &str) -> std::path::PathBuf {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest.join("..").join("..").join("assets").join(name)
    }

    fn copy_asset_font(name: &str, dir: &std::path::Path) -> std::path::PathBuf {
        let src = asset_path(name);
        let dest = dir.join(name);
        std::fs::copy(&src, &dest).expect("copy font");
        dest
    }

    fn set_notofonts_state(state: Option<Arc<NotofontsState>>) -> Option<Arc<NotofontsState>> {
        let mut guard = NOTOFONTS_STATE.lock().expect("state lock");
        let prev = guard.clone();
        *guard = state;
        prev
    }

    fn cwd_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn state_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }
}
