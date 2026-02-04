use crate::{Line, Span, TextStyle, ANSI_TAB_WIDTH};

pub(crate) fn parse_ansi(text: &str) -> Vec<Line> {
    let mut parser = vte::Parser::new();
    let mut performer = AnsiPerformer::new();
    parser.advance(&mut performer, text.as_bytes());
    performer.into_lines()
}

struct AnsiPerformer {
    lines: Vec<Line>,
    style: TextStyle,
    col: usize,
}

impl AnsiPerformer {
    fn new() -> Self {
        Self {
            lines: vec![Line::default()],
            style: TextStyle::default(),
            col: 0,
        }
    }

    fn current_line_mut(&mut self) -> &mut Line {
        if self.lines.is_empty() {
            self.lines.push(Line::default());
        }
        self.lines.last_mut().unwrap()
    }

    fn push_char(&mut self, ch: char) {
        let width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        let style = self.style.clone();
        let line = self.current_line_mut();
        if let Some(last) = line.spans.last_mut() {
            if last.style == style {
                last.text.push(ch);
            } else {
                line.spans.push(Span {
                    text: ch.to_string(),
                    style,
                });
            }
        } else {
            line.spans.push(Span {
                text: ch.to_string(),
                style,
            });
        }
        self.col += width;
    }

    fn new_line(&mut self) {
        self.lines.push(Line::default());
        self.col = 0;
    }

    fn expand_tab(&mut self) {
        let mut count = 0;
        while !(self.col + count).is_multiple_of(ANSI_TAB_WIDTH) {
            count += 1;
        }
        if count == 0 {
            count = ANSI_TAB_WIDTH;
        }
        for _ in 0..count {
            self.push_char(' ');
        }
    }

    fn reset_style(&mut self) {
        self.style = TextStyle::default();
    }
}

impl vte::Perform for AnsiPerformer {
    fn print(&mut self, c: char) {
        self.push_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\r' => {
                self.col = 0;
            }
            b'\t' => self.expand_tab(),
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        if action != 'm' {
            return;
        }

        let mut values = params_to_vec(params);
        if values.is_empty() {
            values.push(0);
        }

        let mut i = 0;
        while i < values.len() {
            match values[i] {
                0 => self.reset_style(),
                1 => self.style.bold = true,
                3 => self.style.italic = true,
                4 => self.style.underline = true,
                9 => self.style.strike = true,
                22 => self.style.bold = false,
                23 => self.style.italic = false,
                24 => self.style.underline = false,
                29 => self.style.strike = false,
                30..=37 => self.style.fg = Some(ansi_color(values[i] as u8)),
                39 => self.style.fg = None,
                40..=47 => self.style.bg = Some(ansi_color((values[i] - 10) as u8)),
                49 => self.style.bg = None,
                90..=97 => self.style.fg = Some(ansi_color((values[i] - 60) as u8)),
                100..=107 => self.style.bg = Some(ansi_color((values[i] - 90) as u8)),
                38 => {
                    if let Some((color, consumed)) = parse_extended_color(&values[i + 1..]) {
                        self.style.fg = Some(color);
                        i += consumed;
                    }
                }
                48 => {
                    if let Some((color, consumed)) = parse_extended_color(&values[i + 1..]) {
                        self.style.bg = Some(color);
                        i += consumed;
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
}

impl AnsiPerformer {
    fn into_lines(mut self) -> Vec<Line> {
        if self.lines.is_empty() {
            self.lines.push(Line::default());
        }
        self.lines
    }
}

fn params_to_vec(params: &vte::Params) -> Vec<u16> {
    let mut values = Vec::new();
    for p in params.iter() {
        if p.is_empty() {
            values.push(0);
        } else {
            values.push(p[0]);
        }
    }
    values
}

fn parse_extended_color(values: &[u16]) -> Option<(String, usize)> {
    if values.is_empty() {
        return None;
    }
    match values[0] {
        5 => {
            if values.len() >= 2 {
                Some((xterm_color(values[1] as u8), 2))
            } else {
                None
            }
        }
        2 => {
            if values.len() >= 4 {
                let r = values[1] as u8;
                let g = values[2] as u8;
                let b = values[3] as u8;
                Some((format!("#{r:02X}{g:02X}{b:02X}"), 4))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(crate) fn ansi_color(code: u8) -> String {
    let palette = [
        "#282a2e", "#D74E6F", "#31BB71", "#D3E561", "#8056FF", "#ED61D7", "#04D7D7", "#C5C8C6",
        "#4B4B4B", "#FE5F86", "#00D787", "#EBFF71", "#8F69FF", "#FF7AEA", "#00FEFE", "#FFFFFF",
    ];
    let idx = match code {
        30..=37 => (code - 30) as usize,
        40..=47 => (code - 40) as usize,
        90..=97 => (code - 90 + 8) as usize,
        100..=107 => (code - 100 + 8) as usize,
        _ => code as usize,
    };
    if idx < palette.len() {
        palette[idx].to_string()
    } else {
        "#C5C8C6".to_string()
    }
}

pub(crate) fn xterm_color(idx: u8) -> String {
    if idx < 16 {
        return ansi_color(idx);
    }
    if idx >= 232 {
        let v = 8 + (idx - 232) * 10;
        return format!("#{v:02X}{v:02X}{v:02X}");
    }
    let idx = idx - 16;
    let r = idx / 36;
    let g = (idx % 36) / 6;
    let b = idx % 6;
    let to_comp = |v: u8| if v == 0 { 0 } else { 55 + 40 * v };
    let rr = to_comp(r);
    let gg = to_comp(g);
    let bb = to_comp(b);
    format!("#{rr:02X}{gg:02X}{bb:02X}")
}

pub(crate) fn wrap_ansi_lines(lines: &[Line], width: usize) -> Vec<Line> {
    if width == 0 {
        return lines.to_vec();
    }
    let mut out = Vec::new();
    for line in lines {
        out.extend(split_line_by_width(line, width));
    }
    out
}

pub(crate) fn split_line_by_width(line: &Line, width: usize) -> Vec<Line> {
    if width == 0 {
        return vec![line.clone()];
    }
    let mut out = Vec::new();
    let mut current = Line::default();
    let mut current_width = 0usize;

    for span in &line.spans {
        let mut buf = String::new();
        for ch in span.text.chars() {
            let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if current_width + w > width && !current.spans.is_empty() {
                if !buf.is_empty() {
                    current.spans.push(Span {
                        text: buf.clone(),
                        style: span.style.clone(),
                    });
                    buf.clear();
                }
                out.push(current);
                current = Line::default();
                current_width = 0;
            }
            buf.push(ch);
            current_width += w;
            if current_width >= width {
                current.spans.push(Span {
                    text: buf.clone(),
                    style: span.style.clone(),
                });
                buf.clear();
                out.push(current);
                current = Line::default();
                current_width = 0;
            }
        }
        if !buf.is_empty() {
            current.spans.push(Span {
                text: buf.clone(),
                style: span.style.clone(),
            });
        }
    }

    if !current.spans.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        out.push(Line::default());
    }
    out
}
