use crate::{Error, Line, Result, Span, TextStyle};
use once_cell::sync::Lazy;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Color, FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;

pub(crate) fn highlight_code(
    text: &str,
    path: Option<&Path>,
    language: Option<&str>,
    theme_name: &str,
) -> Result<(Vec<Line>, String)> {
    static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
    static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);
    static CHARM_THEME: Lazy<syntect::highlighting::Theme> = Lazy::new(charm_theme);

    let ps = &*SYNTAX_SET;
    let ts = &*THEME_SET;

    let theme = if theme_name.eq_ignore_ascii_case("charm") {
        &*CHARM_THEME
    } else if let Some(theme) = ts.themes.get(theme_name) {
        theme
    } else if let Some(theme) = ts.themes.get("base16-ocean.dark") {
        theme
    } else if let Some(theme) = ts.themes.values().next() {
        theme
    } else {
        return Err(Error::Render("no themes available".to_string()));
    };

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

    let mut highlighter = HighlightLines::new(syntax, theme);
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

fn push_span(spans: &mut Vec<Span>, text: String, style: TextStyle) {
    if let Some(last) = spans.last_mut() {
        if last.style == style {
            last.text.push_str(&text);
            return;
        }
    }
    spans.push(Span { text, style });
}

fn color_to_hex(color: syntect::highlighting::Color) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
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
