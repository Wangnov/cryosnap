use serde::{Deserialize, Serialize};

use crate::{
    DEFAULT_PNG_OPT_LEVEL, DEFAULT_PNG_QUANTIZE_DITHER, DEFAULT_PNG_QUANTIZE_QUALITY,
    DEFAULT_PNG_QUANTIZE_SPEED, DEFAULT_RASTER_MAX_PIXELS, DEFAULT_RASTER_SCALE,
    DEFAULT_TITLE_MAX_WIDTH, DEFAULT_TITLE_OPACITY, DEFAULT_TITLE_SIZE,
};

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
            backend: RasterBackend::Auto,
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
