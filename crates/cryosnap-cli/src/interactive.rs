use crate::parse::{parse_box, parse_font_dirs, parse_font_fallbacks};
use cryosnap_core::{CjkRegion, Config, FontSystemFallback};
use dialoguer::{Confirm, Input, Select};
use std::error::Error;

pub(crate) fn run_interactive(
    config: &mut Config,
    input: &mut Option<String>,
    execute: &mut Option<String>,
) -> Result<(), Box<dyn Error>> {
    let prompter = DialoguerPrompter;
    run_interactive_with(&prompter, config, input, execute)
}

pub(crate) trait Prompter {
    fn select(&self, prompt: &str, items: &[&str], default: usize)
        -> Result<usize, Box<dyn Error>>;
    fn input_string(
        &self,
        prompt: &str,
        default: Option<&str>,
        allow_empty: bool,
    ) -> Result<String, Box<dyn Error>>;
    fn input_f32(&self, prompt: &str, default: f32) -> Result<f32, Box<dyn Error>>;
    fn confirm(&self, prompt: &str, default: bool) -> Result<bool, Box<dyn Error>>;
}

struct DialoguerPrompter;

impl Prompter for DialoguerPrompter {
    fn select(
        &self,
        prompt: &str,
        items: &[&str],
        default: usize,
    ) -> Result<usize, Box<dyn Error>> {
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
    ) -> Result<String, Box<dyn Error>> {
        let mut input = Input::new().with_prompt(prompt).allow_empty(allow_empty);
        if let Some(value) = default {
            input = input.default(value.to_string());
        }
        Ok(input.interact_text()?)
    }

    fn input_f32(&self, prompt: &str, default: f32) -> Result<f32, Box<dyn Error>> {
        Ok(Input::new()
            .with_prompt(prompt)
            .default(default)
            .interact_text()?)
    }

    fn confirm(&self, prompt: &str, default: bool) -> Result<bool, Box<dyn Error>> {
        Ok(Confirm::new()
            .with_prompt(prompt)
            .default(default)
            .interact()?)
    }
}

pub(crate) fn run_interactive_with(
    prompter: &dyn Prompter,
    config: &mut Config,
    input: &mut Option<String>,
    execute: &mut Option<String>,
) -> Result<(), Box<dyn Error>> {
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
    let fallbacks_default = if config.font.fallbacks.is_empty() {
        String::new()
    } else {
        config.font.fallbacks.join(", ")
    };
    let fallbacks = prompter.input_string(
        "Font fallbacks (comma-separated)",
        Some(&fallbacks_default),
        true,
    )?;
    config.font.fallbacks = parse_font_fallbacks(&fallbacks)?;
    let fallback_items = ["auto", "always", "never"];
    let fallback_default = match config.font.system_fallback {
        FontSystemFallback::Auto => 0,
        FontSystemFallback::Always => 1,
        FontSystemFallback::Never => 2,
    };
    let fallback_choice =
        prompter.select("System font fallback", &fallback_items, fallback_default)?;
    config.font.system_fallback = match fallback_choice {
        1 => FontSystemFallback::Always,
        2 => FontSystemFallback::Never,
        _ => FontSystemFallback::Auto,
    };
    config.font.auto_download =
        prompter.confirm("Auto-download missing fonts?", config.font.auto_download)?;
    let cjk_items = ["auto", "sc", "tc", "hk", "jp", "kr"];
    let cjk_default = match config.font.cjk_region {
        CjkRegion::Auto => 0,
        CjkRegion::Sc => 1,
        CjkRegion::Tc => 2,
        CjkRegion::Hk => 3,
        CjkRegion::Jp => 4,
        CjkRegion::Kr => 5,
    };
    let cjk_choice = prompter.select("CJK region", &cjk_items, cjk_default)?;
    config.font.cjk_region = match cjk_choice {
        1 => CjkRegion::Sc,
        2 => CjkRegion::Tc,
        3 => CjkRegion::Hk,
        4 => CjkRegion::Jp,
        5 => CjkRegion::Kr,
        _ => CjkRegion::Auto,
    };
    config.font.force_update =
        prompter.confirm("Force refresh downloaded fonts?", config.font.force_update)?;
    let dirs_default = if config.font.dirs.is_empty() {
        String::new()
    } else {
        config.font.dirs.join(", ")
    };
    let dirs = prompter.input_string(
        "Font dirs (comma-separated, empty for default)",
        Some(&dirs_default),
        true,
    )?;
    config.font.dirs = parse_font_dirs(&dirs)?;
    config.font.size = prompter.input_f32("Font size", config.font.size)?;
    config.font.ligatures = prompter.confirm("Enable ligatures?", config.font.ligatures)?;
    config.line_height = prompter.input_f32("Line height", config.line_height)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::VecDeque;

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
        ) -> Result<usize, Box<dyn Error>> {
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
        ) -> Result<String, Box<dyn Error>> {
            self.strings
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| "missing string".into())
        }

        fn input_f32(&self, _prompt: &str, _default: f32) -> Result<f32, Box<dyn Error>> {
            self.floats
                .borrow_mut()
                .pop_front()
                .ok_or_else(|| "missing float".into())
        }

        fn confirm(&self, _prompt: &str, _default: bool) -> Result<bool, Box<dyn Error>> {
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
        prompter.selects.borrow_mut().push_back(0);
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
        prompter
            .strings
            .borrow_mut()
            .push_back("Symbols Nerd Font Mono, Noto Sans CJK SC".to_string());
        prompter.strings.borrow_mut().push_back(String::new());
        prompter.floats.borrow_mut().push_back(4.0);
        prompter.floats.borrow_mut().push_back(1.0);
        prompter.floats.borrow_mut().push_back(6.0);
        prompter.floats.borrow_mut().push_back(0.0);
        prompter.floats.borrow_mut().push_back(12.0);
        prompter.floats.borrow_mut().push_back(14.0);
        prompter.floats.borrow_mut().push_back(1.3);
        prompter.bools.borrow_mut().push_back(true);
        prompter.bools.borrow_mut().push_back(true);
        prompter.bools.borrow_mut().push_back(true);
        prompter.bools.borrow_mut().push_back(false);
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
        assert_eq!(
            cfg.font.fallbacks,
            vec!["Symbols Nerd Font Mono", "Noto Sans CJK SC"]
        );
        assert!(matches!(cfg.font.system_fallback, FontSystemFallback::Auto));
        assert!(cfg.font.auto_download);
        assert!(matches!(cfg.font.cjk_region, CjkRegion::Auto));
    }

    #[test]
    fn run_interactive_with_stdin_choice() {
        let prompter = FakePrompter::new();
        prompter.selects.borrow_mut().push_back(2);
        prompter.selects.borrow_mut().push_back(0);
        prompter.selects.borrow_mut().push_back(0);
        prompter.strings.borrow_mut().push_back("charm".to_string());
        prompter
            .strings
            .borrow_mut()
            .push_back("#000000".to_string());
        prompter.strings.borrow_mut().push_back("10".to_string());
        prompter.strings.borrow_mut().push_back("0".to_string());
        prompter
            .strings
            .borrow_mut()
            .push_back("#333333".to_string());
        prompter.strings.borrow_mut().push_back("Test".to_string());
        prompter.strings.borrow_mut().push_back(String::new());
        prompter.strings.borrow_mut().push_back(String::new());
        prompter.floats.borrow_mut().push_back(0.0);
        prompter.floats.borrow_mut().push_back(0.0);
        prompter.floats.borrow_mut().push_back(0.0);
        prompter.floats.borrow_mut().push_back(0.0);
        prompter.floats.borrow_mut().push_back(12.0);
        prompter.floats.borrow_mut().push_back(14.0);
        prompter.floats.borrow_mut().push_back(1.3);
        prompter.bools.borrow_mut().push_back(false);
        prompter.bools.borrow_mut().push_back(false);
        prompter.bools.borrow_mut().push_back(false);
        prompter.bools.borrow_mut().push_back(false);
        prompter.bools.borrow_mut().push_back(false);

        let mut cfg = Config::default();
        let mut input = None;
        let mut execute = None;
        run_interactive_with(&prompter, &mut cfg, &mut input, &mut execute).expect("interactive");
        assert_eq!(input.as_deref(), Some("-"));
    }

    #[test]
    fn run_interactive_selects_never_fallback_and_hk_region() {
        let prompter = FakePrompter::new();
        prompter.selects.borrow_mut().push_back(0);
        prompter.selects.borrow_mut().push_back(2);
        prompter.selects.borrow_mut().push_back(3);
        prompter
            .strings
            .borrow_mut()
            .push_back("input.rs".to_string());
        prompter.strings.borrow_mut().push_back("charm".to_string());
        prompter
            .strings
            .borrow_mut()
            .push_back("#000000".to_string());
        prompter.strings.borrow_mut().push_back("10".to_string());
        prompter.strings.borrow_mut().push_back("0".to_string());
        prompter
            .strings
            .borrow_mut()
            .push_back("#333333".to_string());
        prompter.strings.borrow_mut().push_back("Test".to_string());
        prompter.strings.borrow_mut().push_back(String::new());
        prompter.strings.borrow_mut().push_back(String::new());
        prompter.floats.borrow_mut().push_back(0.0);
        prompter.floats.borrow_mut().push_back(0.0);
        prompter.floats.borrow_mut().push_back(0.0);
        prompter.floats.borrow_mut().push_back(0.0);
        prompter.floats.borrow_mut().push_back(12.0);
        prompter.floats.borrow_mut().push_back(14.0);
        prompter.floats.borrow_mut().push_back(1.3);
        prompter.bools.borrow_mut().push_back(false);
        prompter.bools.borrow_mut().push_back(false);
        prompter.bools.borrow_mut().push_back(false);
        prompter.bools.borrow_mut().push_back(false);
        prompter.bools.borrow_mut().push_back(false);

        let mut cfg = Config::default();
        let mut input = None;
        let mut execute = None;
        run_interactive_with(&prompter, &mut cfg, &mut input, &mut execute).expect("interactive");

        assert!(matches!(
            cfg.font.system_fallback,
            FontSystemFallback::Never
        ));
        assert!(matches!(cfg.font.cjk_region, CjkRegion::Hk));
    }

    #[test]
    fn interactive_command_branch() {
        let prompter = FakePrompter::new();
        prompter.selects.borrow_mut().push_back(1);
        prompter.selects.borrow_mut().push_back(0);
        prompter.selects.borrow_mut().push_back(0);
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
        prompter
            .strings
            .borrow_mut()
            .push_back("Symbols Nerd Font Mono".to_string());
        prompter.strings.borrow_mut().push_back(String::new());
        prompter.floats.borrow_mut().push_back(4.0);
        prompter.floats.borrow_mut().push_back(1.0);
        prompter.floats.borrow_mut().push_back(6.0);
        prompter.floats.borrow_mut().push_back(0.0);
        prompter.floats.borrow_mut().push_back(12.0);
        prompter.floats.borrow_mut().push_back(14.0);
        prompter.floats.borrow_mut().push_back(1.3);
        prompter.bools.borrow_mut().push_back(true);
        prompter.bools.borrow_mut().push_back(true);
        prompter.bools.borrow_mut().push_back(true);
        prompter.bools.borrow_mut().push_back(false);
        prompter.bools.borrow_mut().push_back(false);

        let mut cfg = Config::default();
        let mut input = None;
        let mut execute = None;
        run_interactive_with(&prompter, &mut cfg, &mut input, &mut execute).expect("interactive");
        assert_eq!(execute.as_deref(), Some("echo hi"));
        assert!(input.is_none());
    }
}
