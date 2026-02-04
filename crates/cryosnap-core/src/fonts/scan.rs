use super::models::FontFallbackNeeds;
use crate::Line;
use unicode_script::{Script, UnicodeScript};

pub(crate) fn is_private_use(ch: char) -> bool {
    let cp = ch as u32;
    (0xE000..=0xF8FF).contains(&cp)
        || (0xF0000..=0xFFFFD).contains(&cp)
        || (0x100000..=0x10FFFD).contains(&cp)
}

pub(crate) fn is_cjk(ch: char) -> bool {
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

pub(crate) fn is_emoji(ch: char) -> bool {
    let cp = ch as u32;
    matches!(
        cp,
        0x2300..=0x23FF
            | 0x2600..=0x27BF
            | 0x2B00..=0x2BFF
            | 0x1F000..=0x1FAFF
    )
}

pub(crate) fn scan_text_fallbacks(text: &str, needs: &mut FontFallbackNeeds) {
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

pub(crate) fn collect_font_fallback_needs(
    lines: &[Line],
    title_text: Option<&str>,
) -> FontFallbackNeeds {
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
