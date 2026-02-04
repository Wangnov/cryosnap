use crate::{Error, Line, Result, DEFAULT_TAB_WIDTH};

pub(crate) fn expand_box(values: &[f32]) -> [f32; 4] {
    match values.len() {
        1 => [values[0], values[0], values[0], values[0]],
        2 => [values[0], values[1], values[0], values[1]],
        4 => [values[0], values[1], values[2], values[3]],
        _ => [0.0, 0.0, 0.0, 0.0],
    }
}

pub(crate) fn text_width_cells(text: &str) -> usize {
    let mut width = 0usize;
    for ch in text.chars() {
        if ch == '\t' {
            width += DEFAULT_TAB_WIDTH;
        } else {
            width += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        }
    }
    width
}

pub(crate) fn truncate_to_cells(text: &str, max_cells: usize, ellipsis: &str) -> String {
    if max_cells == 0 {
        return String::new();
    }
    let width = text_width_cells(text);
    if width <= max_cells {
        return text.to_string();
    }
    let ellipsis_width = text_width_cells(ellipsis);
    if ellipsis_width >= max_cells {
        return ellipsis.chars().take(1).collect();
    }
    let mut out = String::new();
    let mut current = 0usize;
    for ch in text.chars() {
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if current + w > max_cells - ellipsis_width {
            break;
        }
        out.push(ch);
        current += w;
    }
    out.push_str(ellipsis);
    out
}

pub(crate) fn line_width_cells(line: &Line) -> usize {
    let mut width = 0usize;
    for span in &line.spans {
        for ch in span.text.chars() {
            if ch == '\t' {
                let mut count = 0;
                while !(width + count).is_multiple_of(DEFAULT_TAB_WIDTH) {
                    count += 1;
                }
                if count == 0 {
                    count = DEFAULT_TAB_WIDTH;
                }
                width += count;
            } else {
                width += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            }
        }
    }
    width
}

pub(crate) fn span_width_px(text: &str, char_width: f32) -> f32 {
    let mut width = 0usize;
    for ch in text.chars() {
        if ch == '\t' {
            let mut count = 0;
            while !(width + count).is_multiple_of(DEFAULT_TAB_WIDTH) {
                count += 1;
            }
            if count == 0 {
                count = DEFAULT_TAB_WIDTH;
            }
            width += count;
        } else {
            width += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        }
    }
    width as f32 * char_width
}

pub(crate) fn scale_dimension(value: u32, scale: f32) -> Result<u32> {
    let scaled = (value as f32 * scale).round();
    if !scaled.is_finite() || scaled <= 0.0 {
        return Err(Error::Render("invalid raster scale".to_string()));
    }
    if scaled > u32::MAX as f32 {
        return Err(Error::Render("raster size overflow".to_string()));
    }
    Ok(scaled as u32)
}
