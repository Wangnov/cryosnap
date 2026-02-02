use clap::{CommandFactory, Parser, ValueEnum};
use cryosnap_core::{
    Config, InputSource, OutputFormat, PngQuantPreset, PngStrip, RasterBackend, RenderRequest,
    TitleAlign, TitlePathStyle,
};
use dialoguer::{Confirm, Input, Select};
use directories::ProjectDirs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "cryosnap",
    about = "Generate images of code and terminal output.",
    version
)]
struct Args {
    /// Input file path. Use "-" to read from stdin.
    input: Option<String>,

    /// Output file path (.svg/.png/.webp). Supports out.{svg,png,webp}.
    /// If omitted, writes to stdout (or cryosnap.png when stdout is a TTY).
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format (svg, png, or webp).
    #[arg(long, value_enum)]
    format: Option<FormatArg>,

    /// JSON config file path (default/base/full/user or custom file).
    #[arg(short, long)]
    config: Option<String>,

    /// Use interactive mode to configure settings.
    #[arg(short, long)]
    interactive: bool,

    /// Background color (e.g. #171717).
    #[arg(short = 'b', long)]
    background: Option<String>,

    /// Padding (1,2,4 values).
    #[arg(short = 'p', long)]
    padding: Option<String>,

    /// Margin (1,2,4 values).
    #[arg(short = 'm', long)]
    margin: Option<String>,

    /// Width of output image.
    #[arg(short = 'W', long)]
    width: Option<f32>,

    /// Height of output image.
    #[arg(short = 'H', long)]
    height: Option<f32>,

    /// Theme name for syntax highlighting.
    #[arg(short = 't', long)]
    theme: Option<String>,

    /// Language name for syntax highlighting.
    #[arg(short = 'l', long)]
    language: Option<String>,

    /// Wrap lines at a specific width.
    #[arg(short = 'w', long)]
    wrap: Option<usize>,

    /// Lines to capture (start,end).
    #[arg(long)]
    lines: Option<String>,

    /// Show window controls.
    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    window: Option<bool>,

    /// Show line numbers.
    #[arg(
        long,
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    show_line_numbers: Option<bool>,

    /// Border radius.
    #[arg(short = 'r', long = "border.radius")]
    border_radius: Option<f32>,

    /// Border width.
    #[arg(long = "border.width")]
    border_width: Option<f32>,

    /// Border color.
    #[arg(long = "border.color")]
    border_color: Option<String>,

    /// Shadow blur.
    #[arg(long = "shadow.blur")]
    shadow_blur: Option<f32>,

    /// Shadow offset X.
    #[arg(long = "shadow.x")]
    shadow_x: Option<f32>,

    /// Shadow offset Y.
    #[arg(long = "shadow.y")]
    shadow_y: Option<f32>,

    /// Font family.
    #[arg(long = "font.family")]
    font_family: Option<String>,

    /// Font file path.
    #[arg(long = "font.file")]
    font_file: Option<String>,

    /// Font size.
    #[arg(long = "font.size")]
    font_size: Option<f32>,

    /// Line height.
    #[arg(long = "line-height")]
    line_height: Option<f32>,

    /// Raster scale when width/height not specified.
    #[arg(long = "raster.scale")]
    raster_scale: Option<f32>,

    /// Maximum raster pixels to cap memory usage (0 disables).
    #[arg(long = "raster.max-pixels")]
    raster_max_pixels: Option<u64>,

    /// Raster backend (auto, resvg, rsvg).
    #[arg(long = "raster.backend", value_enum)]
    raster_backend: Option<RasterBackendArg>,

    /// Enable title bar text when window controls are shown.
    #[arg(
        long = "title",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    title: Option<bool>,

    /// Title text override.
    #[arg(long = "title.text")]
    title_text: Option<String>,

    /// Title path style for file inputs (absolute, relative, basename).
    #[arg(long = "title.path-style", value_enum)]
    title_path_style: Option<TitlePathStyleArg>,

    /// tmux title format string.
    #[arg(long = "title.tmux-format")]
    title_tmux_format: Option<String>,

    /// Title alignment (left, center, right).
    #[arg(long = "title.align", value_enum)]
    title_align: Option<TitleAlignArg>,

    /// Title font size.
    #[arg(long = "title.size")]
    title_size: Option<f32>,

    /// Title color.
    #[arg(long = "title.color")]
    title_color: Option<String>,

    /// Title opacity (0-1).
    #[arg(long = "title.opacity")]
    title_opacity: Option<f32>,

    /// Title max width (cells).
    #[arg(long = "title.max-width")]
    title_max_width: Option<usize>,

    /// Title ellipsis string.
    #[arg(long = "title.ellipsis")]
    title_ellipsis: Option<String>,

    /// Enable font ligatures.
    #[arg(
        long = "font.ligatures",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    font_ligatures: Option<bool>,

    /// Optimize PNG output (lossless).
    #[arg(
        long = "png-opt",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    png_opt: Option<bool>,

    /// PNG optimization preset (0-6).
    #[arg(long = "png-opt-level")]
    png_opt_level: Option<u8>,

    /// PNG metadata strip mode (none, safe, all).
    #[arg(long = "png-strip", value_enum)]
    png_strip: Option<PngStripArg>,

    /// Quantize PNG output (lossy, libimagequant).
    #[arg(
        long = "png-quant",
        num_args = 0..=1,
        default_missing_value = "true",
        value_parser = clap::builder::BoolishValueParser::new()
    )]
    png_quant: Option<bool>,

    /// PNG quantize quality (0-100).
    #[arg(long = "png-quant-quality")]
    png_quant_quality: Option<u8>,

    /// PNG quantize speed (1-10).
    #[arg(long = "png-quant-speed")]
    png_quant_speed: Option<u8>,

    /// PNG quantize dithering level (0-1).
    #[arg(long = "png-quant-dither")]
    png_quant_dither: Option<f32>,

    /// PNG quantize preset (fast, balanced, best).
    #[arg(long = "png-quant-preset", value_enum)]
    png_quant_preset: Option<PngQuantPresetArg>,

    /// Execute timeout (e.g. 500ms, 2s).
    #[arg(long = "execute.timeout")]
    execute_timeout: Option<String>,

    /// Execute command and capture output.
    #[arg(short = 'x', long)]
    execute: Option<String>,

    /// Capture output from tmux capture-pane.
    #[arg(long)]
    tmux: bool,

    /// Raw args passed to `tmux capture-pane` (e.g. "-t %3 -S -200 -E 100 -J").
    #[arg(long = "tmux-args", value_name = "ARGS", allow_hyphen_values = true)]
    tmux_args: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArg {
    Svg,
    Png,
    Webp,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PngStripArg {
    None,
    Safe,
    All,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum RasterBackendArg {
    Auto,
    Resvg,
    Rsvg,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PngQuantPresetArg {
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
enum TitleAlignArg {
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
enum TitlePathStyleArg {
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

fn main() {
    if let Err(err) = run() {
        eprintln!("ERROR: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    run_with(
        args,
        atty::is(atty::Stream::Stdin),
        atty::is(atty::Stream::Stdout),
        None,
    )
}

fn run_with(
    args: Args,
    stdin_is_tty: bool,
    stdout_is_tty: bool,
    stdin_override: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut config, is_default_config) = load_config(args.config.as_deref())?;
    let mut quantize_set = false;
    if let Some(background) = args.background {
        config.background = background;
    }
    if let Some(padding) = args.padding {
        config.padding = parse_box(&padding)?;
    }
    if let Some(margin) = args.margin {
        config.margin = parse_box(&margin)?;
    }
    if let Some(width) = args.width {
        config.width = width;
    }
    if let Some(height) = args.height {
        config.height = height;
    }
    if let Some(theme) = args.theme {
        config.theme = theme;
    }
    if let Some(language) = args.language {
        config.language = Some(language);
    }
    if let Some(wrap) = args.wrap {
        config.wrap = wrap;
    }
    if let Some(lines) = args.lines {
        config.lines = parse_lines(&lines)?;
    }
    if let Some(window) = args.window {
        config.window_controls = window;
    }
    if let Some(show) = args.show_line_numbers {
        config.show_line_numbers = show;
    }
    if let Some(radius) = args.border_radius {
        config.border.radius = radius;
    }
    if let Some(width) = args.border_width {
        config.border.width = width;
    }
    if let Some(color) = args.border_color {
        config.border.color = color;
    }
    if let Some(blur) = args.shadow_blur {
        config.shadow.blur = blur;
    }
    if let Some(x) = args.shadow_x {
        config.shadow.x = x;
    }
    if let Some(y) = args.shadow_y {
        config.shadow.y = y;
    }
    if let Some(family) = args.font_family {
        config.font.family = family;
    }
    if let Some(file) = args.font_file {
        config.font.file = Some(file);
    }
    if let Some(size) = args.font_size {
        config.font.size = size;
    }
    if let Some(line_height) = args.line_height {
        config.line_height = line_height;
    }
    if let Some(scale) = args.raster_scale {
        config.raster.scale = scale;
    }
    if let Some(max_pixels) = args.raster_max_pixels {
        config.raster.max_pixels = max_pixels;
    }
    if let Some(backend) = args.raster_backend {
        config.raster.backend = backend.into();
    }
    if let Some(ligatures) = args.font_ligatures {
        config.font.ligatures = ligatures;
    }
    if let Some(timeout) = args.execute_timeout {
        config.execute_timeout_ms = parse_timeout_ms(&timeout)?;
    }
    if let Some(optimize) = args.png_opt {
        config.png.optimize = optimize;
    }
    if let Some(level) = args.png_opt_level {
        config.png.level = level;
    }
    if let Some(strip) = args.png_strip {
        config.png.strip = strip.into();
    }
    if let Some(quantize) = args.png_quant {
        config.png.quantize = quantize;
        quantize_set = true;
    }
    if let Some(quality) = args.png_quant_quality {
        config.png.quantize_quality = quality;
    }
    if let Some(speed) = args.png_quant_speed {
        config.png.quantize_speed = speed;
    }
    if let Some(dither) = args.png_quant_dither {
        config.png.quantize_dither = dither;
    }
    if let Some(preset) = args.png_quant_preset {
        config.png.quantize_preset = Some(preset.into());
        if !quantize_set {
            config.png.quantize = true;
        }
    }
    if let Some(enabled) = args.title {
        config.title.enabled = enabled;
    }
    if let Some(text) = args.title_text {
        config.title.text = Some(text);
    }
    if let Some(style) = args.title_path_style {
        config.title.path_style = style.into();
    }
    if let Some(format) = args.title_tmux_format {
        config.title.tmux_format = format;
    }
    if let Some(align) = args.title_align {
        config.title.align = align.into();
    }
    if let Some(size) = args.title_size {
        config.title.size = size;
    }
    if let Some(color) = args.title_color {
        config.title.color = color;
    }
    if let Some(opacity) = args.title_opacity {
        config.title.opacity = opacity;
    }
    if let Some(max_width) = args.title_max_width {
        config.title.max_width = max_width;
    }
    if let Some(ellipsis) = args.title_ellipsis {
        config.title.ellipsis = ellipsis;
    }

    if args.tmux {
        if args.execute.is_some() || args.input.is_some() {
            return Err("tmux mode cannot be combined with --execute or input".into());
        }
        if args.interactive {
            return Err("tmux mode cannot be combined with --interactive".into());
        }
    }

    let mut input_arg = args.input.clone();
    let mut execute_arg = args.execute.clone();
    if args.interactive {
        if !stdin_is_tty {
            return Err("interactive mode requires a TTY".into());
        }
        run_interactive(&mut config, &mut input_arg, &mut execute_arg)?;
        if is_default_config {
            save_user_config(&config)?;
        }
    }

    let input_for_output = if args.tmux { None } else { input_arg.clone() };
    let input = if args.tmux {
        let tmux_output = capture_tmux_output(args.tmux_args.as_deref())?;
        if config.language.is_none() {
            config.language = Some("ansi".to_string());
        }
        if config.title.enabled {
            let should_fill = config
                .title
                .text
                .as_ref()
                .map(|text| text.trim().is_empty())
                .unwrap_or(true);
            if should_fill {
                if let Some(title) =
                    tmux_title(args.tmux_args.as_deref(), &config.title.tmux_format)
                {
                    config.title.text = Some(title);
                }
            }
        }
        InputSource::Text(tmux_output)
    } else if let Some(cmd) = execute_arg {
        InputSource::Command(cmd)
    } else if let Some(input) = input_arg {
        if input == "-" {
            InputSource::Text(read_stdin_with(stdin_override)?)
        } else {
            InputSource::File(PathBuf::from(input))
        }
    } else if !stdin_is_tty {
        InputSource::Text(read_stdin_with(stdin_override)?)
    } else {
        let mut cmd = Args::command();
        cmd.print_help()?;
        println!();
        return Ok(());
    };

    if let Some(output) = args.output.as_ref() {
        if let Some(expanded) = expand_output_pattern(output)? {
            if args.format.is_some() {
                return Err("output patterns cannot be combined with --format".into());
            }
            let svg = cryosnap_core::render_svg(&input, &config)?;
            for path in expanded {
                let format = format_from_extension(&path)
                    .ok_or_else(|| format!("unknown output format: {}", path.display()))?;
                let bytes = match format {
                    OutputFormat::Svg => svg.clone(),
                    OutputFormat::Png => cryosnap_core::render_png_from_svg(&svg, &config)?,
                    OutputFormat::Webp => cryosnap_core::render_webp_from_svg(&svg, &config)?,
                };
                std::fs::write(&path, bytes)?;
                if stdout_is_tty {
                    print_wrote(&path);
                }
            }
            return Ok(());
        }
    }

    let mut format_arg = args.format;
    let mut format = resolve_format(format_arg, args.output.as_ref());
    if stdout_is_tty && args.output.is_none() && format_arg.is_none() {
        format = OutputFormat::Png;
        format_arg = Some(FormatArg::Png);
    }
    let request = RenderRequest {
        input,
        config,
        format,
    };

    let result = cryosnap_core::render(&request)?;

    write_output_with_tty(
        result,
        args.output.as_ref(),
        input_for_output.as_deref(),
        format_arg,
        stdout_is_tty,
    )?;

    Ok(())
}

fn resolve_format(arg: Option<FormatArg>, output: Option<&PathBuf>) -> OutputFormat {
    if let Some(arg) = arg {
        return match arg {
            FormatArg::Svg => OutputFormat::Svg,
            FormatArg::Png => OutputFormat::Png,
            FormatArg::Webp => OutputFormat::Webp,
        };
    }
    if let Some(path) = output {
        if let Some(format) = format_from_extension(path) {
            return format;
        }
    }
    OutputFormat::Svg
}

fn format_from_extension(path: &Path) -> Option<OutputFormat> {
    let ext = path.extension().and_then(|v| v.to_str())?;
    match ext.to_ascii_lowercase().as_str() {
        "png" => Some(OutputFormat::Png),
        "svg" => Some(OutputFormat::Svg),
        "webp" => Some(OutputFormat::Webp),
        _ => None,
    }
}

fn load_config(config_arg: Option<&str>) -> Result<(Config, bool), Box<dyn std::error::Error>> {
    let name = config_arg.unwrap_or("default");
    let is_default = name == "default";

    let config = match name {
        "default" | "base" => serde_json::from_str::<Config>(BASE_CONFIG)?,
        "full" => serde_json::from_str::<Config>(FULL_CONFIG)?,
        "user" => match load_user_config() {
            Ok(config) => config,
            Err(_) => serde_json::from_str::<Config>(BASE_CONFIG)?,
        },
        _ => {
            if Path::new(name).exists() {
                let bytes = std::fs::read(name)?;
                serde_json::from_slice::<Config>(&bytes)?
            } else {
                return Err(format!("config not found: {name}").into());
            }
        }
    };

    Ok((config, is_default))
}

fn load_user_config() -> Result<Config, Box<dyn std::error::Error>> {
    let path = user_config_path()?;
    let bytes = std::fs::read(path)?;
    Ok(serde_json::from_slice::<Config>(&bytes)?)
}

fn save_user_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let path = user_config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(config)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

fn user_config_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(path) = std::env::var("CRYOSNAP_CONFIG_PATH") {
        return Ok(PathBuf::from(path));
    }
    if let Ok(dir) = std::env::var("CRYOSNAP_CONFIG_DIR") {
        return Ok(PathBuf::from(dir).join("user.json"));
    }
    let project = ProjectDirs::from("sh", "cryosnap", "cryosnap")
        .ok_or("unable to resolve config directory")?;
    Ok(project.config_dir().join("user.json"))
}

fn run_interactive(
    config: &mut Config,
    input: &mut Option<String>,
    execute: &mut Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let prompter = DialoguerPrompter;
    run_interactive_with(&prompter, config, input, execute)
}

trait Prompter {
    fn select(
        &self,
        prompt: &str,
        items: &[&str],
        default: usize,
    ) -> Result<usize, Box<dyn std::error::Error>>;
    fn input_string(
        &self,
        prompt: &str,
        default: Option<&str>,
        allow_empty: bool,
    ) -> Result<String, Box<dyn std::error::Error>>;
    fn input_f32(&self, prompt: &str, default: f32) -> Result<f32, Box<dyn std::error::Error>>;
    fn confirm(&self, prompt: &str, default: bool) -> Result<bool, Box<dyn std::error::Error>>;
}

struct DialoguerPrompter;

impl Prompter for DialoguerPrompter {
    fn select(
        &self,
        prompt: &str,
        items: &[&str],
        default: usize,
    ) -> Result<usize, Box<dyn std::error::Error>> {
        Ok(Select::new()
            .with_prompt(prompt)
            .items(items)
            .default(default)
            .interact()?)
    }

    fn input_string(
        &self,
        prompt: &str,
        default: Option<&str>,
        allow_empty: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut input = Input::new().with_prompt(prompt).allow_empty(allow_empty);
        if let Some(value) = default {
            input = input.default(value.to_string());
        }
        Ok(input.interact_text()?)
    }

    fn input_f32(&self, prompt: &str, default: f32) -> Result<f32, Box<dyn std::error::Error>> {
        Ok(Input::new()
            .with_prompt(prompt)
            .default(default)
            .interact_text()?)
    }

    fn confirm(&self, prompt: &str, default: bool) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(Confirm::new()
            .with_prompt(prompt)
            .default(default)
            .interact()?)
    }
}

fn run_interactive_with(
    prompter: &dyn Prompter,
    config: &mut Config,
    input: &mut Option<String>,
    execute: &mut Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let choice = prompter.select("Input source", &["file", "command", "stdin"], 0)?;

    match choice {
        0 => {
            let path = prompter.input_string("Input file path", None, true)?;
            if !path.trim().is_empty() {
                *input = Some(path);
                *execute = None;
            }
        }
        1 => {
            let cmd = prompter.input_string("Command to execute", None, true)?;
            if !cmd.trim().is_empty() {
                *execute = Some(cmd);
                *input = None;
            }
        }
        _ => {
            *input = Some("-".to_string());
            *execute = None;
        }
    }

    config.theme = prompter.input_string("Theme", Some(&config.theme), false)?;
    config.background = prompter.input_string("Background", Some(&config.background), false)?;

    let padding_default = config
        .padding
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let padding = prompter.input_string("Padding (1,2,4 values)", Some(&padding_default), false)?;
    config.padding = parse_box(&padding)?;

    let margin_default = config
        .margin
        .iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let margin = prompter.input_string("Margin (1,2,4 values)", Some(&margin_default), false)?;
    config.margin = parse_box(&margin)?;

    config.window_controls = prompter.confirm("Show window controls?", config.window_controls)?;
    config.show_line_numbers = prompter.confirm("Show line numbers?", config.show_line_numbers)?;

    config.border.radius = prompter.input_f32("Border radius", config.border.radius)?;
    config.border.width = prompter.input_f32("Border width", config.border.width)?;
    config.border.color =
        prompter.input_string("Border color", Some(&config.border.color), false)?;

    config.shadow.blur = prompter.input_f32("Shadow blur", config.shadow.blur)?;
    config.shadow.x = prompter.input_f32("Shadow offset X", config.shadow.x)?;
    config.shadow.y = prompter.input_f32("Shadow offset Y", config.shadow.y)?;

    config.font.family = prompter.input_string("Font family", Some(&config.font.family), false)?;
    config.font.size = prompter.input_f32("Font size", config.font.size)?;
    config.font.ligatures = prompter.confirm("Enable ligatures?", config.font.ligatures)?;
    config.line_height = prompter.input_f32("Line height", config.line_height)?;

    Ok(())
}

fn write_output_with_tty(
    result: cryosnap_core::RenderResult,
    output: Option<&PathBuf>,
    _input: Option<&str>,
    format: Option<FormatArg>,
    stdout_is_tty: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(path) = output {
        std::fs::write(path, result.bytes)?;
        if stdout_is_tty {
            print_wrote(path);
        }
        return Ok(());
    }

    if stdout_is_tty {
        let default_name = match format {
            Some(FormatArg::Png) => "cryosnap.png",
            Some(FormatArg::Webp) => "cryosnap.webp",
            _ => "cryosnap.svg",
        };
        let output_name = default_name.to_string();
        std::fs::write(&output_name, result.bytes)?;
        print_wrote(Path::new(&output_name));
        return Ok(());
    }

    let mut stdout = io::stdout();
    stdout.write_all(&result.bytes)?;
    Ok(())
}

fn print_wrote(path: &Path) {
    println!("WROTE {}", path.display());
}

fn read_stdin() -> Result<String, io::Error> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer)
}

fn read_stdin_with(stdin_override: Option<&str>) -> Result<String, io::Error> {
    if let Some(value) = stdin_override {
        return Ok(value.to_string());
    }
    read_stdin()
}

fn capture_tmux_output(raw_args: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    let user_args = normalize_tmux_args(raw_args)?;
    let cmd_args = build_tmux_capture_args(&user_args);
    let output = std::process::Command::new("tmux")
        .args(cmd_args)
        .output()
        .map_err(|err| format!("failed to run tmux: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tmux capture failed: {}", stderr.trim()).into());
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.is_empty() {
        return Err("tmux returned empty output".into());
    }
    Ok(text)
}

fn tmux_title(raw_args: Option<&str>, format: &str) -> Option<String> {
    let format = format.trim();
    if format.is_empty() {
        return None;
    }
    let user_args = normalize_tmux_args(raw_args).ok()?;
    let target = extract_tmux_target(&user_args);
    let mut cmd = std::process::Command::new("tmux");
    cmd.arg("display-message").arg("-p");
    if let Some(target) = target {
        cmd.arg("-t").arg(target);
    }
    cmd.arg(format);
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn extract_tmux_target(args: &[String]) -> Option<String> {
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "-t" {
            if let Some(next) = iter.next() {
                return Some(next.clone());
            }
        } else if let Some(target) = arg.strip_prefix("-t") {
            if !target.is_empty() {
                return Some(target.to_string());
            }
        }
    }
    None
}

fn normalize_tmux_args(raw: Option<&str>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    match raw {
        Some(value) => {
            let args =
                shell_words::split(value).map_err(|err| format!("tmux args parse: {err}"))?;
            Ok(args)
        }
        None => Ok(Vec::new()),
    }
}

fn build_tmux_capture_args(user_args: &[String]) -> Vec<String> {
    let has_p = user_args.iter().any(|arg| arg == "-p");
    let has_e = user_args.iter().any(|arg| arg == "-e");
    let mut args = Vec::new();
    args.push("capture-pane".to_string());
    if !has_p {
        args.push("-p".to_string());
    }
    if !has_e {
        args.push("-e".to_string());
    }
    args.extend(user_args.iter().cloned());
    args
}

fn parse_box(input: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.split([',', ' ']).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Ok(vec![0.0]);
    }
    let mut out = Vec::new();
    for part in parts {
        out.push(part.parse::<f32>()?);
    }
    match out.len() {
        1 | 2 | 4 => Ok(out),
        _ => Err(format!("expected 1, 2, or 4 values, got {}", out.len()).into()),
    }
}

fn parse_lines(input: &str) -> Result<Vec<i32>, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = input.split([',', ' ']).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for part in parts {
        out.push(part.parse::<i32>()?);
    }
    match out.len() {
        1 | 2 => Ok(out),
        _ => Err(format!("expected 1 or 2 values, got {}", out.len()).into()),
    }
}

fn parse_timeout_ms(input: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let trimmed = input.trim();
    if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit()) {
        return Ok(trimmed.parse()?);
    }
    let duration = humantime::parse_duration(trimmed)?;
    let millis = duration.as_millis();
    if millis > u64::MAX as u128 {
        return Err("timeout too large".into());
    }
    Ok(millis as u64)
}

fn expand_output_pattern(
    output: &Path,
) -> Result<Option<Vec<PathBuf>>, Box<dyn std::error::Error>> {
    let output = output.to_string_lossy();
    let open = output.find('{');
    let close = output.find('}');

    match (open, close) {
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => Err("invalid output pattern".into()),
        (Some(start), Some(end)) => {
            if end <= start {
                return Err("invalid output pattern".into());
            }
            if output[start + 1..].contains('{') || output[end + 1..].contains('}') {
                return Err("invalid output pattern".into());
            }
            let inner = &output[start + 1..end];
            let parts: Vec<&str> = inner
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();
            if parts.is_empty() {
                return Err("invalid output pattern".into());
            }
            let prefix = &output[..start];
            let suffix = &output[end + 1..];
            let mut outputs = Vec::new();
            for part in parts {
                outputs.push(PathBuf::from(format!("{prefix}{part}{suffix}")));
            }
            Ok(Some(outputs))
        }
    }
}

const BASE_CONFIG: &str = include_str!("../configurations/base.json");
const FULL_CONFIG: &str = include_str!("../configurations/full.json");

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    #[test]
    fn parse_box_accepts_values() {
        let out = parse_box("10,20,30,40").expect("parse");
        assert_eq!(out, vec![10.0, 20.0, 30.0, 40.0]);
    }

    #[test]
    fn parse_box_rejects_invalid_length() {
        assert!(parse_box("1,2,3").is_err());
    }

    #[test]
    fn parse_box_empty_defaults() {
        let out = parse_box("").expect("parse");
        assert_eq!(out, vec![0.0]);
    }

    #[test]
    fn parse_lines_accepts_values() {
        let out = parse_lines("2,5").expect("parse");
        assert_eq!(out, vec![2, 5]);
    }

    #[test]
    fn parse_lines_rejects_invalid_length() {
        assert!(parse_lines("1,2,3").is_err());
    }

    #[test]
    fn parse_lines_empty_defaults() {
        let out = parse_lines("").expect("parse");
        assert!(out.is_empty());
    }

    #[test]
    fn parse_timeout_ms_numeric() {
        let out = parse_timeout_ms("1500").expect("parse");
        assert_eq!(out, 1500);
    }

    #[test]
    fn parse_timeout_ms_human() {
        let out = parse_timeout_ms("2s").expect("parse");
        assert_eq!(out, 2000);
    }

    #[test]
    fn load_config_default() {
        let (cfg, is_default) = load_config(None).expect("load config");
        assert!(is_default);
        assert_eq!(cfg.theme, "charm");
    }

    #[test]
    fn load_config_full() {
        let (cfg, is_default) = load_config(Some("full")).expect("load config");
        assert!(!is_default);
        assert!(cfg.window_controls);
        assert_eq!(cfg.border.radius, 8.0);
    }

    #[test]
    fn load_config_user_fallback() {
        let _lock = env_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        std::env::set_var("CRYOSNAP_CONFIG_DIR", dir.path());
        let (cfg, _) = load_config(Some("user")).expect("load config");
        assert!(!cfg.window_controls);
        std::env::remove_var("CRYOSNAP_CONFIG_DIR");
    }

    #[test]
    fn load_config_missing_errors() {
        let err = load_config(Some("does-not-exist")).err();
        assert!(err.is_some());
    }

    struct FakePrompter {
        selects: RefCell<VecDeque<usize>>,
        strings: RefCell<VecDeque<String>>,
        floats: RefCell<VecDeque<f32>>,
        bools: RefCell<VecDeque<bool>>,
    }

    impl FakePrompter {
        fn new() -> Self {
            Self {
                selects: RefCell::new(VecDeque::new()),
                strings: RefCell::new(VecDeque::new()),
                floats: RefCell::new(VecDeque::new()),
                bools: RefCell::new(VecDeque::new()),
            }
        }
    }

    impl Prompter for FakePrompter {
        fn select(
            &self,
            _prompt: &str,
            _items: &[&str],
            _default: usize,
        ) -> Result<usize, Box<dyn std::error::Error>> {
            self.selects
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| "missing select".into())
        }

        fn input_string(
            &self,
            _prompt: &str,
            _default: Option<&str>,
            _allow_empty: bool,
        ) -> Result<String, Box<dyn std::error::Error>> {
            self.strings
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| "missing string".into())
        }

        fn input_f32(
            &self,
            _prompt: &str,
            _default: f32,
        ) -> Result<f32, Box<dyn std::error::Error>> {
            self.floats
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| "missing float".into())
        }

        fn confirm(
            &self,
            _prompt: &str,
            _default: bool,
        ) -> Result<bool, Box<dyn std::error::Error>> {
            self.bools
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| "missing bool".into())
        }
    }

    #[test]
    fn run_interactive_updates_config() {
        let prompter = FakePrompter::new();
        prompter.selects.borrow_mut().push_back(0);
        prompter
            .strings
            .borrow_mut()
            .push_back("input.rs".to_string());
        prompter.strings.borrow_mut().push_back("charm".to_string());
        prompter
            .strings
            .borrow_mut()
            .push_back("#000000".to_string());
        prompter.strings.borrow_mut().push_back("10,20".to_string());
        prompter.strings.borrow_mut().push_back("5".to_string());
        prompter
            .strings
            .borrow_mut()
            .push_back("#333333".to_string());
        prompter.strings.borrow_mut().push_back("Test".to_string());
        prompter.floats.borrow_mut().push_back(4.0);
        prompter.floats.borrow_mut().push_back(1.0);
        prompter.floats.borrow_mut().push_back(6.0);
        prompter.floats.borrow_mut().push_back(0.0);
        prompter.floats.borrow_mut().push_back(12.0);
        prompter.floats.borrow_mut().push_back(14.0);
        prompter.floats.borrow_mut().push_back(1.3);
        prompter.bools.borrow_mut().push_back(true);
        prompter.bools.borrow_mut().push_back(true);
        prompter.bools.borrow_mut().push_back(false);

        let mut cfg = Config::default();
        let mut input = None;
        let mut execute = None;
        run_interactive_with(&prompter, &mut cfg, &mut input, &mut execute).expect("interactive");

        assert_eq!(input.as_deref(), Some("input.rs"));
        assert_eq!(cfg.padding, vec![10.0, 20.0]);
        assert_eq!(cfg.margin, vec![5.0]);
        assert!(cfg.window_controls);
        assert!(cfg.show_line_numbers);
        assert_eq!(cfg.border.radius, 4.0);
        assert_eq!(cfg.border.width, 1.0);
        assert_eq!(cfg.border.color, "#333333");
        assert_eq!(cfg.shadow.blur, 6.0);
        assert_eq!(cfg.font.family, "Test");
        assert_eq!(cfg.font.size, 14.0);
        assert_eq!(cfg.line_height, 1.3);
        assert!(!cfg.font.ligatures);
    }

    #[test]
    fn save_and_load_user_config() {
        let _lock = env_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        std::env::set_var("CRYOSNAP_CONFIG_DIR", dir.path());
        let cfg = Config {
            theme: "custom".to_string(),
            ..Config::default()
        };
        save_user_config(&cfg).expect("save");
        let loaded = load_user_config().expect("load");
        assert_eq!(loaded.theme, "custom");
        std::env::remove_var("CRYOSNAP_CONFIG_DIR");
    }

    #[test]
    fn load_config_from_path() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"theme":"custom"}"#).expect("write");
        let (cfg, is_default) = load_config(path.to_str()).expect("load");
        assert!(!is_default);
        assert_eq!(cfg.theme, "custom");
    }

    #[test]
    fn write_output_to_file() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("out.svg");
        let result = cryosnap_core::RenderResult {
            format: OutputFormat::Svg,
            bytes: b"<svg/>".to_vec(),
        };
        write_output_with_tty(result, Some(&path), None, None, false).expect("write");
        let content = std::fs::read_to_string(path).expect("read");
        assert!(content.contains("<svg"));
    }

    #[test]
    fn write_output_default_name() {
        let _lock = cwd_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        let cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("chdir");
        let result = cryosnap_core::RenderResult {
            format: OutputFormat::Svg,
            bytes: b"<svg/>".to_vec(),
        };
        write_output_with_tty(result, None, Some("file.rs"), Some(FormatArg::Png), true)
            .expect("write");
        assert!(dir.path().join("cryosnap.png").exists());
        std::env::set_current_dir(cwd).expect("restore");
    }

    #[test]
    fn write_output_stdin_default_name() {
        let _lock = cwd_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        let cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("chdir");
        let result = cryosnap_core::RenderResult {
            format: OutputFormat::Svg,
            bytes: b"<svg/>".to_vec(),
        };
        write_output_with_tty(result, None, Some("-"), Some(FormatArg::Png), true).expect("write");
        assert!(dir.path().join("cryosnap.png").exists());
        std::env::set_current_dir(cwd).expect("restore");
    }

    #[test]
    fn write_output_stdout_branch() {
        let result = cryosnap_core::RenderResult {
            format: OutputFormat::Svg,
            bytes: b"test".to_vec(),
        };
        write_output_with_tty(result, None, None, None, false).expect("write");
    }

    #[test]
    fn resolve_format_from_output() {
        let out = resolve_format(None, Some(&PathBuf::from("out.png")));
        assert!(matches!(out, OutputFormat::Png));
        let out = resolve_format(None, Some(&PathBuf::from("out.svg")));
        assert!(matches!(out, OutputFormat::Svg));
        let out = resolve_format(None, Some(&PathBuf::from("out.webp")));
        assert!(matches!(out, OutputFormat::Webp));
    }

    #[test]
    fn expand_output_pattern_basic() {
        let outputs = expand_output_pattern(&PathBuf::from("out.{svg,png,webp}"))
            .expect("expand")
            .expect("outputs");
        let names: Vec<String> = outputs
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        assert_eq!(names, vec!["out.svg", "out.png", "out.webp"]);
    }

    #[test]
    fn expand_output_pattern_invalid() {
        let err = expand_output_pattern(&PathBuf::from("out.{svg,png")).err();
        assert!(err.is_some());
    }

    #[test]
    fn read_stdin_with_override() {
        let out = read_stdin_with(Some("hello")).expect("read");
        assert_eq!(out, "hello");
    }

    #[test]
    fn run_with_rejects_tmux_execute_combo() {
        let mut args = Args::parse_from(["cryosnap"]);
        args.tmux = true;
        args.execute = Some("echo hi".to_string());
        let err = run_with(args, true, false, None).unwrap_err();
        assert!(err.to_string().contains("tmux mode"));
    }

    #[test]
    fn run_with_interactive_requires_tty() {
        let mut args = Args::parse_from(["cryosnap"]);
        args.interactive = true;
        let err = run_with(args, false, false, None).unwrap_err();
        assert!(err.to_string().contains("interactive mode requires a TTY"));
    }

    #[test]
    fn run_with_output_pattern_conflicts_with_format() {
        let dir = tempdir().expect("temp dir");
        let mut args = Args::parse_from(["cryosnap"]);
        args.input = Some("-".to_string());
        args.output = Some(dir.path().join("out.{svg,png}"));
        args.format = Some(FormatArg::Svg);
        let err = run_with(args, false, false, Some("hello")).unwrap_err();
        assert!(err
            .to_string()
            .contains("output patterns cannot be combined"));
    }

    #[test]
    fn run_with_reads_stdin_when_piped() {
        let dir = tempdir().expect("temp dir");
        let out_path = dir.path().join("out.svg");
        let mut args = Args::parse_from(["cryosnap"]);
        args.output = Some(out_path.clone());
        args.png_quant = Some(true);
        let result = run_with(args, false, false, Some("hello"));
        assert!(result.is_ok());
        let content = std::fs::read_to_string(out_path).expect("read svg");
        assert!(content.contains("<svg"));
    }

    #[test]
    fn run_with_applies_many_overrides() {
        let dir = tempdir().expect("temp dir");
        let out_path = dir.path().join("out.svg");
        let mut args = Args::parse_from(["cryosnap"]);
        args.input = Some("-".to_string());
        args.output = Some(out_path.clone());
        args.background = Some("#101010".to_string());
        args.padding = Some("1,2,3,4".to_string());
        args.margin = Some("5,6".to_string());
        args.width = Some(800.0);
        args.height = Some(600.0);
        args.theme = Some("charm".to_string());
        args.language = Some("rust".to_string());
        args.wrap = Some(80);
        args.lines = Some("1,2".to_string());
        args.window = Some(true);
        args.show_line_numbers = Some(true);
        args.border_radius = Some(4.0);
        args.border_width = Some(1.0);
        args.border_color = Some("#333333".to_string());
        args.shadow_blur = Some(6.0);
        args.shadow_x = Some(1.0);
        args.shadow_y = Some(2.0);
        args.font_family = Some("JetBrains Mono".to_string());
        args.font_size = Some(12.0);
        args.line_height = Some(1.4);
        args.raster_scale = Some(2.0);
        args.raster_max_pixels = Some(1_000_000);
        args.raster_backend = Some(RasterBackendArg::Resvg);
        args.font_ligatures = Some(false);
        args.execute_timeout = Some("500ms".to_string());
        args.png_opt = Some(false);
        args.png_opt_level = Some(3);
        args.png_strip = Some(PngStripArg::All);
        args.png_quant_quality = Some(80);
        args.png_quant_speed = Some(5);
        args.png_quant_dither = Some(0.7);
        args.png_quant_preset = Some(PngQuantPresetArg::Fast);
        args.title = Some(true);
        args.title_text = Some("Title".to_string());
        args.title_path_style = Some(TitlePathStyleArg::Basename);
        args.title_tmux_format = Some("format".to_string());
        args.title_align = Some(TitleAlignArg::Right);
        args.title_size = Some(10.0);
        args.title_color = Some("#ffffff".to_string());
        args.title_opacity = Some(0.7);
        args.title_max_width = Some(30);
        args.title_ellipsis = Some("..".to_string());

        let result = run_with(args, false, false, Some("hello"));
        assert!(result.is_ok());
        assert!(out_path.exists());
    }

    #[test]
    fn resolve_format_from_arg() {
        let out = resolve_format(Some(FormatArg::Png), None);
        assert!(matches!(out, OutputFormat::Png));
    }

    #[test]
    fn format_from_extension_unknown() {
        assert!(format_from_extension(Path::new("out.txt")).is_none());
    }

    #[test]
    fn interactive_command_branch() {
        let prompter = FakePrompter::new();
        prompter.selects.borrow_mut().push_back(1);
        prompter
            .strings
            .borrow_mut()
            .push_back("echo hi".to_string());
        prompter.strings.borrow_mut().push_back("charm".to_string());
        prompter
            .strings
            .borrow_mut()
            .push_back("#000000".to_string());
        prompter.strings.borrow_mut().push_back("10".to_string());
        prompter.strings.borrow_mut().push_back("5".to_string());
        prompter
            .strings
            .borrow_mut()
            .push_back("#333333".to_string());
        prompter.strings.borrow_mut().push_back("Test".to_string());
        prompter.floats.borrow_mut().push_back(4.0);
        prompter.floats.borrow_mut().push_back(1.0);
        prompter.floats.borrow_mut().push_back(6.0);
        prompter.floats.borrow_mut().push_back(0.0);
        prompter.floats.borrow_mut().push_back(12.0);
        prompter.floats.borrow_mut().push_back(14.0);
        prompter.floats.borrow_mut().push_back(1.3);
        prompter.bools.borrow_mut().push_back(true);
        prompter.bools.borrow_mut().push_back(true);
        prompter.bools.borrow_mut().push_back(false);

        let mut cfg = Config::default();
        let mut input = None;
        let mut execute = None;
        run_interactive_with(&prompter, &mut cfg, &mut input, &mut execute).expect("interactive");
        assert_eq!(execute.as_deref(), Some("echo hi"));
        assert!(input.is_none());
    }

    #[test]
    fn build_tmux_capture_args_defaults() {
        let args = build_tmux_capture_args(&[]);
        assert_eq!(args, vec!["capture-pane", "-p", "-e"]);
    }

    #[test]
    fn build_tmux_capture_args_preserves_flags() {
        let args = build_tmux_capture_args(&[
            "-p".to_string(),
            "-e".to_string(),
            "-t".to_string(),
            "%3".to_string(),
        ]);
        assert_eq!(args, vec!["capture-pane", "-p", "-e", "-t", "%3",]);
    }

    #[test]
    fn normalize_tmux_args_accepts_raw_string() {
        let args = normalize_tmux_args(Some("-t %3 -S -200 -J")).expect("parse");
        assert_eq!(args, vec!["-t", "%3", "-S", "-200", "-J"]);
    }

    #[test]
    fn extract_tmux_target_from_separate_flag() {
        let args = vec![
            "-t".to_string(),
            "%7".to_string(),
            "-S".to_string(),
            "-10".to_string(),
        ];
        assert_eq!(extract_tmux_target(&args), Some("%7".to_string()));
    }

    #[test]
    fn extract_tmux_target_from_compact_flag() {
        let args = vec!["-t%9".to_string(), "-S".to_string(), "-5".to_string()];
        assert_eq!(extract_tmux_target(&args), Some("%9".to_string()));
    }

    fn cwd_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }
}
