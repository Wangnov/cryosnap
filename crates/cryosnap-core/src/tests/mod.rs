extern crate png as png_crate;

use super::*;
use crate::ansi::*;
use crate::fonts::dirs::*;
use crate::fonts::*;
use crate::input::*;
use crate::layout::*;
use crate::png::*;
use crate::render::{raster_scale, resolve_title_text, sanitize_title_text, title_text_from_path};
use crate::svg::*;
use crate::syntax::*;
use crate::text::*;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;
use unicode_script::Script;

#[test]
fn deserialize_box_values() {
    let cfg: Config =
        serde_json::from_str(r#"{"padding":"10,20","margin":[1,2,3,4]}"#).expect("parse config");
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
    let path = std::env::temp_dir().join(format!("cryosnap-font-test-{}.ttf", std::process::id()));
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
                url:
                    "https://github.com/notofonts/devanagari/releases/tag/NotoSansDevanagari-v2.006"
                        .to_string(),
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
                url:
                    "https://github.com/notofonts/tamil/releases/tag/NotoSansTamilSupplement-v2.006"
                        .to_string(),
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
        let mut encoder = png_crate::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png_crate::ColorType::Rgba);
        encoder.set_depth(png_crate::BitDepth::Eight);
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
        let mut encoder = png_crate::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png_crate::ColorType::Rgb);
        encoder.set_depth(png_crate::BitDepth::Eight);
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
        let mut encoder = png_crate::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png_crate::ColorType::Rgba);
        encoder.set_depth(png_crate::BitDepth::Eight);
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
        let mut encoder = png_crate::Encoder::new(&mut bytes, 2, 1);
        encoder.set_color(png_crate::ColorType::Grayscale);
        encoder.set_depth(png_crate::BitDepth::Eight);
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
        let mut encoder = png_crate::Encoder::new(&mut bytes, 1, 1);
        encoder.set_color(png_crate::ColorType::GrayscaleAlpha);
        encoder.set_depth(png_crate::BitDepth::Eight);
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
        let mut encoder = png_crate::Encoder::new(&mut bytes, 1, 1);
        encoder.set_color(png_crate::ColorType::Indexed);
        encoder.set_depth(png_crate::BitDepth::Eight);
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
        let mut encoder = png_crate::Encoder::new(&mut bytes, 1, 1);
        encoder.set_color(png_crate::ColorType::Rgba);
        encoder.set_depth(png_crate::BitDepth::Eight);
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
            files: vec!["fonts/NotoSansDevanagari/ttf/NotoSansDevanagari-Regular.ttf".to_string()],
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
                url:
                    "https://github.com/notofonts/devanagari/releases/tag/NotoSansDevanagari-v2.006"
                        .to_string(),
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
            file_path: "fonts/NotoSansDevanagari/ttf/NotoSansDevanagari-Regular.ttf".to_string(),
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
    let downloaded = download_notofonts_file(&download, &temp, false).expect("skip");
    assert!(!downloaded);
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
    let err = execute_command("definitely_not_a_cmd_123", Duration::from_millis(1000)).unwrap_err();
    assert!(matches!(err, Error::Render(_)));
}

#[cfg(unix)]
#[test]
fn execute_command_echo() {
    let output = execute_command("printf 'hello'", Duration::from_millis(2000)).expect("execute");
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
