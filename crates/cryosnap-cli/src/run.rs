use crate::args::{Args, FormatArg};
use crate::config::{load_config, save_user_config};
use crate::interactive::run_interactive;
use crate::io::{print_wrote, read_stdin_with, write_output_with_tty};
use crate::parse::{
    parse_box, parse_font_dirs, parse_font_fallbacks, parse_lines, parse_timeout_ms,
};
use crate::tmux::{capture_tmux_output, tmux_title};
use clap::{CommandFactory, Parser};
use cryosnap_core::{InputSource, OutputFormat, RenderRequest};
use std::error::Error;
use std::path::{Path, PathBuf};

pub(crate) fn run() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    run_with(
        args,
        atty::is(atty::Stream::Stdin),
        atty::is(atty::Stream::Stdout),
        None,
    )
}

pub(crate) fn run_with(
    args: Args,
    stdin_is_tty: bool,
    stdout_is_tty: bool,
    stdin_override: Option<&str>,
) -> Result<(), Box<dyn Error>> {
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
    if let Some(fallbacks) = args.font_fallbacks {
        config.font.fallbacks = parse_font_fallbacks(&fallbacks)?;
    }
    if let Some(dirs) = args.font_dirs {
        config.font.dirs = parse_font_dirs(&dirs)?;
    }
    if let Some(region) = args.font_cjk_region {
        config.font.cjk_region = region.into();
    }
    if let Some(auto_download) = args.font_auto_download {
        config.font.auto_download = auto_download;
    }
    if let Some(force_update) = args.font_force_update {
        config.font.force_update = force_update;
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
    if let Some(mode) = args.font_system_fallback {
        config.font.system_fallback = mode.into();
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
            let mut outputs = Vec::with_capacity(expanded.len());
            for path in expanded {
                let format = format_from_extension(&path)
                    .ok_or_else(|| format!("unknown output format: {}", path.display()))?;
                outputs.push((path, format));
            }

            let wants_png = outputs
                .iter()
                .any(|(_, format)| matches!(format, OutputFormat::Png));
            let wants_webp = outputs
                .iter()
                .any(|(_, format)| matches!(format, OutputFormat::Webp));

            let planned = cryosnap_core::render_svg_planned(&input, &config)?;
            let svg = planned.bytes;
            let png_webp = if wants_png && wants_webp {
                Some(cryosnap_core::render_png_webp_from_svg_once(
                    &svg,
                    &config,
                    planned.needs_system_fonts,
                )?)
            } else {
                None
            };

            for (path, format) in outputs {
                let bytes = match format {
                    OutputFormat::Svg => svg.clone(),
                    OutputFormat::Png => {
                        match &png_webp {
                            Some((png, _)) => png.clone(),
                            None => cryosnap_core::render_png_from_svg(&svg, &config)?,
                        }
                    }
                    OutputFormat::Webp => {
                        match &png_webp {
                            Some((_, webp)) => webp.clone(),
                            None => cryosnap_core::render_webp_from_svg(&svg, &config)?,
                        }
                    }
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

fn expand_output_pattern(output: &Path) -> Result<Option<Vec<PathBuf>>, Box<dyn Error>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::{
        FontCjkRegionArg, FontSystemFallbackArg, PngQuantPresetArg, PngStripArg, RasterBackendArg,
        TitleAlignArg, TitlePathStyleArg,
    };
    use crate::test_utils::{cwd_lock, env_lock};
    use tempfile::tempdir;

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
    fn resolve_format_from_arg() {
        let out = resolve_format(Some(FormatArg::Png), None);
        assert!(matches!(out, OutputFormat::Png));
    }

    #[test]
    fn format_from_extension_unknown() {
        assert!(format_from_extension(Path::new("out.txt")).is_none());
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
    fn expand_output_pattern_invalid_variants() {
        assert!(expand_output_pattern(Path::new("out.}{")).is_err());
        assert!(expand_output_pattern(Path::new("out.{svg,{png}}")).is_err());
        assert!(expand_output_pattern(Path::new("out.{}")).is_err());
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
        args.font_family = Some("monospace".to_string());
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

    fn asset_path(name: &str) -> PathBuf {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        manifest.join("..").join("..").join("assets").join(name)
    }

    #[test]
    fn run_with_font_overrides_and_file_input() {
        let _lock = env_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        let input_path = dir.path().join("input.txt");
        let out_path = dir.path().join("out.svg");
        let fonts_dir = dir.path().join("fonts");
        std::fs::create_dir_all(&fonts_dir).expect("fonts dir");
        std::fs::write(&input_path, "hello").expect("write");
        let font_file = asset_path("JetBrainsMono-Regular.ttf");

        let mut args = Args::parse_from(["cryosnap"]);
        args.input = Some(input_path.to_string_lossy().to_string());
        args.output = Some(out_path.clone());
        args.font_file = Some(font_file.to_string_lossy().to_string());
        args.font_fallbacks = Some("Noto Sans CJK SC".to_string());
        args.font_dirs = Some(fonts_dir.to_string_lossy().to_string());
        args.font_cjk_region = Some(FontCjkRegionArg::Jp);
        args.font_auto_download = Some(false);
        args.font_force_update = Some(true);
        args.font_system_fallback = Some(FontSystemFallbackArg::Never);

        let result = run_with(args, true, false, None);
        assert!(result.is_ok());
        assert!(out_path.exists());
    }

    #[test]
    fn run_with_output_pattern_writes_multiple() {
        let dir = tempdir().expect("temp dir");
        let mut args = Args::parse_from(["cryosnap"]);
        args.input = Some("-".to_string());
        args.output = Some(dir.path().join("out.{svg,png}"));
        let result = run_with(args, false, false, Some("hello"));
        assert!(result.is_ok());
        assert!(dir.path().join("out.svg").exists());
        assert!(dir.path().join("out.png").exists());
    }

    #[test]
    fn run_with_output_pattern_writes_webp_and_prints() {
        let dir = tempdir().expect("temp dir");
        let mut args = Args::parse_from(["cryosnap"]);
        args.input = Some("-".to_string());
        args.output = Some(dir.path().join("out.{svg,webp}"));
        let result = run_with(args, false, true, Some("hello"));
        assert!(result.is_ok());
        assert!(dir.path().join("out.svg").exists());
        assert!(dir.path().join("out.webp").exists());
    }

    #[test]
    fn run_with_help_when_no_input_and_tty() {
        let args = Args::parse_from(["cryosnap"]);
        let result = run_with(args, true, false, None);
        assert!(result.is_ok());
    }

    #[test]
    fn run_with_stdout_tty_default_png() {
        let _lock = cwd_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        let cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("chdir");

        let args = Args::parse_from(["cryosnap"]);
        let result = run_with(args, false, true, Some("hello"));
        assert!(result.is_ok());
        assert!(dir.path().join("cryosnap.png").exists());

        std::env::set_current_dir(cwd).expect("restore");
    }

    #[cfg(unix)]
    #[test]
    fn run_with_execute_command_output() {
        let _lock = env_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        let out_path = dir.path().join("out.svg");
        let mut args = Args::parse_from(["cryosnap"]);
        args.execute = Some("printf 'hello'".to_string());
        args.output = Some(out_path.clone());
        let result = run_with(args, true, false, None);
        assert!(result.is_ok());
        let content = std::fs::read_to_string(out_path).expect("read");
        assert!(content.contains("<svg"));
    }

    #[cfg(unix)]
    #[test]
    fn run_with_tmux_capture_uses_fake_tmux() {
        use std::os::unix::fs::PermissionsExt;

        let _lock = env_lock().lock().expect("lock");
        let bin_dir = tempdir().expect("temp dir");
        let tmux_path = bin_dir.path().join("tmux");
        let script = r#"#!/bin/sh
if [ "$1" = "capture-pane" ]; then
  echo "line1"
  exit 0
fi
if [ "$1" = "display-message" ]; then
  echo "tmux title"
  exit 0
fi
echo "unexpected" 1>&2
exit 1
"#;
        std::fs::write(&tmux_path, script).expect("write");
        let mut perms = std::fs::metadata(&tmux_path).expect("meta").permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tmux_path, perms).expect("chmod");

        let prev_path = std::env::var("PATH").ok();
        let new_path = format!(
            "{}:{}",
            bin_dir.path().display(),
            prev_path.clone().unwrap_or_default()
        );
        std::env::set_var("PATH", new_path);

        let dir = tempdir().expect("temp dir");
        let out_path = dir.path().join("out.svg");
        let mut args = Args::parse_from(["cryosnap"]);
        args.tmux = true;
        args.tmux_args = Some("-t %3".to_string());
        args.output = Some(out_path.clone());
        args.title = Some(true);
        args.title_tmux_format = Some("#{pane_title}".to_string());

        let result = run_with(args, true, false, None);
        assert!(result.is_ok());
        assert!(out_path.exists());

        if let Some(path) = prev_path {
            std::env::set_var("PATH", path);
        } else {
            std::env::remove_var("PATH");
        }
    }
}
