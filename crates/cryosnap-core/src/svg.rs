use base64::Engine;
use std::collections::HashSet;
use std::path::Path;
use unicode_script::{Script, UnicodeScript};

use crate::fonts::{
    cjk_region_families, is_cjk, is_emoji, is_private_use, locale_cjk_region, push_family,
    AUTO_FALLBACK_EMOJI, AUTO_FALLBACK_GLOBAL, AUTO_FALLBACK_NF,
};
use crate::layout::{expand_box, line_width_cells, span_width_px, truncate_to_cells};
use crate::render::sanitize_title_text;
use crate::{
    CjkRegion, Config, Line, Result, TitleAlign, FONT_HEIGHT_TO_WIDTH_RATIO,
    WINDOW_CONTROLS_HEIGHT, WINDOW_CONTROLS_SPACING, WINDOW_CONTROLS_X_OFFSET,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FontGroup {
    Default,
    Cjk,
    Emoji,
    Nerd,
    Unicode,
}

fn font_group_for_char(ch: char, prev: Option<FontGroup>) -> FontGroup {
    if is_private_use(ch) {
        return FontGroup::Nerd;
    }
    if is_emoji(ch) {
        return FontGroup::Emoji;
    }
    if is_cjk(ch) {
        return FontGroup::Cjk;
    }
    let script = ch.script();
    if matches!(script, Script::Common | Script::Inherited | Script::Unknown) {
        if let Some(prev) = prev {
            return prev;
        }
    }
    if ch <= '\u{7f}' {
        FontGroup::Default
    } else {
        FontGroup::Unicode
    }
}

fn split_text_by_font_group(text: &str) -> Vec<(FontGroup, String)> {
    let mut out = Vec::new();
    let mut current_group: Option<FontGroup> = None;
    let mut current = String::new();
    for ch in text.chars() {
        let group = font_group_for_char(ch, current_group);
        match current_group {
            Some(existing) if existing == group => {
                current.push(ch);
            }
            Some(existing) => {
                if !current.is_empty() {
                    out.push((existing, current));
                }
                current = String::new();
                current.push(ch);
                current_group = Some(group);
            }
            None => {
                current_group = Some(group);
                current.push(ch);
            }
        }
    }
    if let Some(group) = current_group {
        if !current.is_empty() {
            out.push((group, current));
        }
    }
    out
}

struct FontFamilyVariants {
    default: String,
    cjk: String,
    emoji: String,
    nerd: String,
    unicode: String,
}

impl FontFamilyVariants {
    fn for_group(&self, group: FontGroup) -> &str {
        match group {
            FontGroup::Default => &self.default,
            FontGroup::Cjk => &self.cjk,
            FontGroup::Emoji => &self.emoji,
            FontGroup::Nerd => &self.nerd,
            FontGroup::Unicode => &self.unicode,
        }
    }
}

fn parse_font_family_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect()
}

fn build_font_family_variant(base: &[String], prefix: &[&str]) -> String {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for name in prefix {
        push_family(&mut out, &mut seen, name);
    }
    for name in base {
        push_family(&mut out, &mut seen, name);
    }
    out.join(", ")
}

pub(crate) fn build_svg(
    lines: &[Line],
    config: &Config,
    default_fg: &str,
    font_css: Option<String>,
    line_offset: usize,
    title_text: Option<&str>,
    font_family: &str,
) -> String {
    let base_families = parse_font_family_list(font_family);
    let default_family = if base_families.is_empty() {
        font_family.to_string()
    } else {
        base_families.join(", ")
    };
    let cjk_region = match config.font.cjk_region {
        CjkRegion::Auto => locale_cjk_region().unwrap_or(CjkRegion::Sc),
        other => other,
    };
    let font_variants = FontFamilyVariants {
        default: default_family,
        cjk: build_font_family_variant(&base_families, cjk_region_families(cjk_region)),
        emoji: build_font_family_variant(&base_families, AUTO_FALLBACK_EMOJI),
        nerd: build_font_family_variant(&base_families, AUTO_FALLBACK_NF),
        unicode: build_font_family_variant(&base_families, AUTO_FALLBACK_GLOBAL),
    };

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
                            escape_attr(&font_variants.default),
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
        escape_attr(&font_variants.default),
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

            for (group, chunk) in split_text_by_font_group(text) {
                let mut chunk_attrs = attrs.clone();
                let family = font_variants.for_group(group);
                if !family.is_empty() {
                    chunk_attrs.push_str(&format!(r#" font-family="{}""#, escape_attr(family)));
                }
                text_layer.push_str(&format!(
                    r#"<tspan xml:space="preserve"{}>{}</tspan>"#,
                    chunk_attrs,
                    escape_text(&chunk)
                ));
            }
            cursor_x += width_px;
        }
        text_layer.push_str("</text>");
    }

    svg.push_str(&bg_layer);
    svg.push_str(&text_layer);
    svg.push_str("</g></svg>");
    svg
}

fn escape_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(text: &str) -> String {
    escape_text(text).replace('"', "&quot;")
}

pub(crate) fn svg_font_face_css(config: &Config) -> Result<Option<String>> {
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
