#[derive(Debug, Clone)]
pub(crate) struct CutResult {
    pub(crate) text: String,
    pub(crate) start: usize,
}

pub(crate) fn cut_text(text: &str, window: &[i32]) -> CutResult {
    if window.is_empty() {
        return CutResult {
            text: text.to_string(),
            start: 0,
        };
    }
    if window.len() == 1 && window[0] == 0 {
        return CutResult {
            text: text.to_string(),
            start: 0,
        };
    }
    if window.len() == 2 && window[0] == 0 && window[1] == -1 {
        return CutResult {
            text: text.to_string(),
            start: 0,
        };
    }

    let lines: Vec<&str> = text.split('\n').collect();
    let total = lines.len() as i32;
    let mut start;
    let mut end = total;

    match window.len() {
        1 => {
            if window[0] > 0 {
                start = window[0];
            } else {
                start = total + window[0];
            }
        }
        _ => {
            start = window[0];
            end = window[1];
        }
    }

    if start < 0 {
        start = 0;
    }
    if start > total {
        start = total;
    }
    end += 1;
    if end < start {
        end = start;
    }
    if end > total {
        end = total;
    }

    let start_usize = start as usize;
    let end_usize = end as usize;
    if start_usize >= lines.len() {
        return CutResult {
            text: String::new(),
            start: start_usize,
        };
    }
    CutResult {
        text: lines[start_usize..end_usize].join("\n"),
        start: start_usize,
    }
}

pub(crate) fn detab(text: &str, tab_width: usize) -> String {
    let mut out = String::new();
    let mut col = 0usize;
    for ch in text.chars() {
        if ch == '\t' {
            let mut count = 0;
            while !(col + count).is_multiple_of(tab_width) {
                count += 1;
            }
            if count == 0 {
                count = tab_width;
            }
            for _ in 0..count {
                out.push(' ');
            }
            col += count;
        } else {
            if ch == '\n' {
                col = 0;
            } else {
                col += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            }
            out.push(ch);
        }
    }
    out
}

pub(crate) fn wrap_text(text: &str, width: usize) -> String {
    if width == 0 {
        return text.to_string();
    }
    let mut out_lines = Vec::new();
    for line in text.split('\n') {
        let mut current = String::new();
        let mut current_width = 0usize;
        for ch in line.chars() {
            let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if current_width + w > width && !current.is_empty() {
                out_lines.push(current);
                current = String::new();
                current_width = 0;
            }
            current.push(ch);
            current_width += w;
            if current_width >= width {
                out_lines.push(current);
                current = String::new();
                current_width = 0;
            }
        }
        out_lines.push(current);
    }
    out_lines.join("\n")
}
