use clap::{Parser, ValueEnum};
use cryosnap_core::{
    CjkRegion, FontSystemFallback, PngQuantPreset, PngStrip, RasterBackend, TitleAlign,
    TitlePathStyle,
};

#[derive(Parser, Debug)]
#[command(
    name = "cryosnap",
    about = "Generate images of code and terminal output.",
    version
)]
pub(crate) struct Args {
    /// Input file path. Use "-" to read from stdin.
    pub(crate) input: Option<String>,

    /// Output file path (.svg/.png/.webp). Supports out.{svg,png,webp}.
    /// If omitted, writes to stdout (or cryosnap.png when stdout is a TTY).
    #[arg(short, long)]
    pub(crate) output: Option<std::path::PathBuf>,

    /// Output format (svg, png, or webp).
    #[arg(long, value_enum)]
    pub(crate) format: Option<FormatArg>,

    /// JSON config file path (default/base/full/user or custom file).
    #[arg(short, long)]
    pub(crate) config: Option<String>,

    /// Use interactive mode to configure settings.
    #[arg(short, long)]
    pub(crate) interactive: bool,

    /// Background color (e.g. #171717).
    #[arg(short = 'b', long)]
    pub(crate) background: Option<String>,

    /// Padding (1,2,4 values).
    #[arg(short = 'p', long)]
    pub(crate) padding: Option<String>,

    /// Margin (1,2,4 values).
    #[arg(short = 'm', long)]
    pub(crate) margin: Option<String>,

    /// Width of output image.
    #[arg(short = 'W', long)]
    pub(crate) width: Option<f32>,

    /// Height of output image.
    #[arg(short = 'H', long)]
    pub(crate) height: Option<f32>,

    /// Theme name for syntax highlighting.
    #[arg(short = 't', long)]
    pub(crate) theme: Option<String>,

    /// Language name for syntax highlighting.
    #[arg(short = 'l', long)]
    pub(crate) language: Option<String>,

    /// Wrap lines at a specific width.
    #[arg(short = 'w', long)]
    pub(crate) wrap: Option<usize>,

    /// Lines to capture (start,end).
    #[arg(long)]
    pub(crate) lines: Option<String>,

    /// Show window controls.
    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    pub(crate) window: Option<bool>,

    /// Show line numbers.
    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    pub(crate) show_line_numbers: Option<bool>,

    /// Border radius.
    #[arg(short = 'r', long = "border.radius")]
    pub(crate) border_radius: Option<f32>,

    /// Border width.
    #[arg(long = "border.width")]
    pub(crate) border_width: Option<f32>,

    /// Border color.
    #[arg(long = "border.color")]
    pub(crate) border_color: Option<String>,

    /// Shadow blur.
    #[arg(long = "shadow.blur")]
    pub(crate) shadow_blur: Option<f32>,

    /// Shadow offset X.
    #[arg(long = "shadow.x")]
    pub(crate) shadow_x: Option<f32>,

    /// Shadow offset Y.
    #[arg(long = "shadow.y")]
    pub(crate) shadow_y: Option<f32>,

    /// Font family.
    #[arg(long = "font.family")]
    pub(crate) font_family: Option<String>,

    /// Font file path.
    #[arg(long = "font.file")]
    pub(crate) font_file: Option<String>,

    /// Font fallback families (comma-separated).
    #[arg(long = "font.fallbacks", value_name = "LIST")]
    pub(crate) font_fallbacks: Option<String>,

    /// Font directories (comma-separated). Defaults to ~/.cryosnap/fonts.
    #[arg(long = "font.dirs", value_name = "LIST")]
    pub(crate) font_dirs: Option<String>,

    /// CJK region preference (auto, sc, tc, hk, jp, kr).
    #[arg(long = "font.cjk-region", value_enum, alias = "font.cjk.region")]
    pub(crate) font_cjk_region: Option<FontCjkRegionArg>,

    /// Auto-download missing fonts.
    #[arg(
        long = "font.auto-download",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    pub(crate) font_auto_download: Option<bool>,

    /// Force refresh downloaded fonts (always check latest when downloading).
    #[arg(
        long = "font.force-update",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    pub(crate) font_force_update: Option<bool>,

    /// System font fallback mode (auto, always, never).
    #[arg(
        long = "font.system-fallback",
        value_enum,
        alias = "font.system_fallback"
    )]
    pub(crate) font_system_fallback: Option<FontSystemFallbackArg>,

    /// Font size.
    #[arg(long = "font.size")]
    pub(crate) font_size: Option<f32>,

    /// Line height.
    #[arg(long = "line-height")]
    pub(crate) line_height: Option<f32>,

    /// Raster scale when width/height not specified.
    #[arg(long = "raster.scale")]
    pub(crate) raster_scale: Option<f32>,

    /// Maximum raster pixels to cap memory usage (0 disables).
    #[arg(long = "raster.max-pixels")]
    pub(crate) raster_max_pixels: Option<u64>,

    /// Raster backend (auto, resvg, rsvg).
    #[arg(long = "raster.backend", value_enum)]
    pub(crate) raster_backend: Option<RasterBackendArg>,

    /// Enable title bar text when window controls are shown.
    #[arg(
        long = "title",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    pub(crate) title: Option<bool>,

    /// Title text override.
    #[arg(long = "title.text")]
    pub(crate) title_text: Option<String>,

    /// Title path style for file inputs (absolute, relative, basename).
    #[arg(long = "title.path-style", value_enum)]
    pub(crate) title_path_style: Option<TitlePathStyleArg>,

    /// tmux title format string.
    #[arg(long = "title.tmux-format")]
    pub(crate) title_tmux_format: Option<String>,

    /// Title alignment (left, center, right).
    #[arg(long = "title.align", value_enum)]
    pub(crate) title_align: Option<TitleAlignArg>,

    /// Title font size.
    #[arg(long = "title.size")]
    pub(crate) title_size: Option<f32>,

    /// Title color.
    #[arg(long = "title.color")]
    pub(crate) title_color: Option<String>,

    /// Title opacity (0-1).
    #[arg(long = "title.opacity")]
    pub(crate) title_opacity: Option<f32>,

    /// Title max width (cells).
    #[arg(long = "title.max-width")]
    pub(crate) title_max_width: Option<usize>,

    /// Title ellipsis string.
    #[arg(long = "title.ellipsis")]
    pub(crate) title_ellipsis: Option<String>,

    /// Enable font ligatures.
    #[arg(
        long = "font.ligatures",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    pub(crate) font_ligatures: Option<bool>,

    /// Optimize PNG output (lossless).
    #[arg(
        long = "png-opt",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    pub(crate) png_opt: Option<bool>,

    /// PNG optimization preset (0-6).
    #[arg(long = "png-opt-level")]
    pub(crate) png_opt_level: Option<u8>,

    /// PNG metadata strip mode (none, safe, all).
    #[arg(long = "png-strip", value_enum)]
    pub(crate) png_strip: Option<PngStripArg>,

    /// Quantize PNG output (lossy, libimagequant).
    #[arg(
        long = "png-quant",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    pub(crate) png_quant: Option<bool>,

    /// PNG quantize quality (0-100).
    #[arg(long = "png-quant-quality")]
    pub(crate) png_quant_quality: Option<u8>,

    /// PNG quantize speed (1-10).
    #[arg(long = "png-quant-speed")]
    pub(crate) png_quant_speed: Option<u8>,

    /// PNG quantize dithering level (0-1).
    #[arg(long = "png-quant-dither")]
    pub(crate) png_quant_dither: Option<f32>,

    /// PNG quantize preset (fast, balanced, best).
    #[arg(long = "png-quant-preset", value_enum)]
    pub(crate) png_quant_preset: Option<PngQuantPresetArg>,

    /// Execute timeout (e.g. 500ms, 2s).
    #[arg(long = "execute.timeout")]
    pub(crate) execute_timeout: Option<String>,

    /// Execute command and capture output.
    #[arg(short = 'x', long)]
    pub(crate) execute: Option<String>,

    /// Capture output from tmux capture-pane.
    #[arg(long)]
    pub(crate) tmux: bool,

    /// Raw args passed to `tmux capture-pane` (e.g. "-t %3 -S -200 -E 100 -J").
    #[arg(long = "tmux-args", value_name = "ARGS", allow_hyphen_values = true)]
    pub(crate) tmux_args: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum FormatArg {
    Svg,
    Png,
    Webp,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum PngStripArg {
    None,
    Safe,
    All,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum RasterBackendArg {
    Auto,
    Resvg,
    Rsvg,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum FontSystemFallbackArg {
    Auto,
    Always,
    Never,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum FontCjkRegionArg {
    Auto,
    Sc,
    Tc,
    Hk,
    Jp,
    Kr,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum PngQuantPresetArg {
    Fast,
    Balanced,
    Best,
}

impl From<PngStripArg> for PngStrip {
    fn from(value: PngStripArg) -> Self {
        match value {
            PngStripArg::None => PngStrip::None,
            PngStripArg::Safe => PngStrip::Safe,
            PngStripArg::All => PngStrip::All,
        }
    }
}

impl From<RasterBackendArg> for RasterBackend {
    fn from(value: RasterBackendArg) -> Self {
        match value {
            RasterBackendArg::Auto => RasterBackend::Auto,
            RasterBackendArg::Resvg => RasterBackend::Resvg,
            RasterBackendArg::Rsvg => RasterBackend::Rsvg,
        }
    }
}

impl From<FontSystemFallbackArg> for FontSystemFallback {
    fn from(value: FontSystemFallbackArg) -> Self {
        match value {
            FontSystemFallbackArg::Auto => FontSystemFallback::Auto,
            FontSystemFallbackArg::Always => FontSystemFallback::Always,
            FontSystemFallbackArg::Never => FontSystemFallback::Never,
        }
    }
}

impl From<FontCjkRegionArg> for CjkRegion {
    fn from(value: FontCjkRegionArg) -> Self {
        match value {
            FontCjkRegionArg::Auto => CjkRegion::Auto,
            FontCjkRegionArg::Sc => CjkRegion::Sc,
            FontCjkRegionArg::Tc => CjkRegion::Tc,
            FontCjkRegionArg::Hk => CjkRegion::Hk,
            FontCjkRegionArg::Jp => CjkRegion::Jp,
            FontCjkRegionArg::Kr => CjkRegion::Kr,
        }
    }
}

impl From<PngQuantPresetArg> for PngQuantPreset {
    fn from(value: PngQuantPresetArg) -> Self {
        match value {
            PngQuantPresetArg::Fast => PngQuantPreset::Fast,
            PngQuantPresetArg::Balanced => PngQuantPreset::Balanced,
            PngQuantPresetArg::Best => PngQuantPreset::Best,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum TitleAlignArg {
    Left,
    Center,
    Right,
}

impl From<TitleAlignArg> for TitleAlign {
    fn from(value: TitleAlignArg) -> Self {
        match value {
            TitleAlignArg::Left => TitleAlign::Left,
            TitleAlignArg::Center => TitleAlign::Center,
            TitleAlignArg::Right => TitleAlign::Right,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum TitlePathStyleArg {
    Absolute,
    Relative,
    Basename,
}

impl From<TitlePathStyleArg> for TitlePathStyle {
    fn from(value: TitlePathStyleArg) -> Self {
        match value {
            TitlePathStyleArg::Absolute => TitlePathStyle::Absolute,
            TitlePathStyleArg::Relative => TitlePathStyle::Relative,
            TitlePathStyleArg::Basename => TitlePathStyle::Basename,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arg_enum_conversions_cover_variants() {
        assert!(matches!(PngStrip::from(PngStripArg::None), PngStrip::None));
        assert!(matches!(PngStrip::from(PngStripArg::Safe), PngStrip::Safe));
        assert!(matches!(PngStrip::from(PngStripArg::All), PngStrip::All));

        assert!(matches!(
            RasterBackend::from(RasterBackendArg::Auto),
            RasterBackend::Auto
        ));
        assert!(matches!(
            RasterBackend::from(RasterBackendArg::Resvg),
            RasterBackend::Resvg
        ));
        assert!(matches!(
            RasterBackend::from(RasterBackendArg::Rsvg),
            RasterBackend::Rsvg
        ));

        assert!(matches!(
            FontSystemFallback::from(FontSystemFallbackArg::Auto),
            FontSystemFallback::Auto
        ));
        assert!(matches!(
            FontSystemFallback::from(FontSystemFallbackArg::Always),
            FontSystemFallback::Always
        ));
        assert!(matches!(
            FontSystemFallback::from(FontSystemFallbackArg::Never),
            FontSystemFallback::Never
        ));

        assert!(matches!(
            CjkRegion::from(FontCjkRegionArg::Auto),
            CjkRegion::Auto
        ));
        assert!(matches!(
            CjkRegion::from(FontCjkRegionArg::Sc),
            CjkRegion::Sc
        ));
        assert!(matches!(
            CjkRegion::from(FontCjkRegionArg::Tc),
            CjkRegion::Tc
        ));
        assert!(matches!(
            CjkRegion::from(FontCjkRegionArg::Hk),
            CjkRegion::Hk
        ));
        assert!(matches!(
            CjkRegion::from(FontCjkRegionArg::Jp),
            CjkRegion::Jp
        ));
        assert!(matches!(
            CjkRegion::from(FontCjkRegionArg::Kr),
            CjkRegion::Kr
        ));

        assert!(matches!(
            PngQuantPreset::from(PngQuantPresetArg::Fast),
            PngQuantPreset::Fast
        ));
        assert!(matches!(
            PngQuantPreset::from(PngQuantPresetArg::Balanced),
            PngQuantPreset::Balanced
        ));
        assert!(matches!(
            PngQuantPreset::from(PngQuantPresetArg::Best),
            PngQuantPreset::Best
        ));

        assert!(matches!(
            TitleAlign::from(TitleAlignArg::Left),
            TitleAlign::Left
        ));
        assert!(matches!(
            TitleAlign::from(TitleAlignArg::Center),
            TitleAlign::Center
        ));
        assert!(matches!(
            TitleAlign::from(TitleAlignArg::Right),
            TitleAlign::Right
        ));

        assert!(matches!(
            TitlePathStyle::from(TitlePathStyleArg::Absolute),
            TitlePathStyle::Absolute
        ));
        assert!(matches!(
            TitlePathStyle::from(TitlePathStyleArg::Relative),
            TitlePathStyle::Relative
        ));
        assert!(matches!(
            TitlePathStyle::from(TitlePathStyleArg::Basename),
            TitlePathStyle::Basename
        ));
    }
}
