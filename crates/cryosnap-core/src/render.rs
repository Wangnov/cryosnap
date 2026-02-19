use crate::ansi::{parse_ansi, wrap_ansi_lines};
use crate::fonts::{
    build_font_families, build_font_plan, build_fontdb, collect_font_fallback_needs,
    ensure_fonts_available, load_app_font_families, needs_system_fonts, resolve_script_font_plan,
    scan_text_fallbacks, FontFallbackNeeds, FontPlan, ScriptFontPlan,
};
use crate::input::{is_ansi_input, load_input};
use crate::layout::scale_dimension;
use crate::png::{optimize_png, pixmap_to_webp, quantize_pixmap_to_png, quantize_png_bytes};
use crate::svg::{build_svg, svg_font_face_css};
use crate::syntax::highlight_code;
use crate::text::{cut_text, detab, wrap_text};
use crate::{
    Config, Error, FontSystemFallback, InputSource, OutputFormat, RasterBackend, RenderRequest,
    RenderResult, Result, TitlePathStyle, DEFAULT_TAB_WIDTH,
};
use once_cell::sync::Lazy;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

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

#[derive(Debug, Clone)]
pub struct PlannedSvg {
    pub bytes: Vec<u8>,
    pub needs_system_fonts: bool,
}

pub fn render_svg_planned(input: &InputSource, config: &Config) -> Result<PlannedSvg> {
    let rendered = render_svg_with_plan(input, config)?;
    Ok(PlannedSvg {
        bytes: rendered.bytes,
        needs_system_fonts: rendered.font_plan.needs_system_fonts,
    })
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

pub(crate) fn raster_scale(config: &Config, base_width: u32, base_height: u32) -> Result<f32> {
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

pub(crate) fn resolve_title_text(input: &InputSource, config: &Config) -> Option<String> {
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

pub(crate) fn title_text_from_path(path: &Path, style: TitlePathStyle) -> String {
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

pub(crate) fn sanitize_title_text(text: &str) -> String {
    text.replace(['\n', '\r'], " ").trim().to_string()
}
