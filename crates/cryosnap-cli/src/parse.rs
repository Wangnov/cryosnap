use std::error::Error;

pub(crate) fn parse_box(input: &str) -> Result<Vec<f32>, Box<dyn Error>> {
    let parts: Vec<&str> = input.split([',', ' ']).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Ok(vec![0.0]);
    }
    if !matches!(parts.len(), 1 | 2 | 4) {
        return Err("invalid box length".into());
    }
    Ok(parts
        .iter()
        .map(|part| part.parse::<f32>())
        .collect::<Result<Vec<f32>, _>>()?)
}

pub(crate) fn parse_lines(input: &str) -> Result<Vec<i32>, Box<dyn Error>> {
    let parts: Vec<&str> = input.split([',', ' ']).filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return Ok(vec![]);
    }
    if !matches!(parts.len(), 1 | 2) {
        return Err("invalid lines length".into());
    }
    Ok(parts
        .iter()
        .map(|part| part.parse::<i32>())
        .collect::<Result<Vec<i32>, _>>()?)
}

pub(crate) fn parse_font_fallbacks(input: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    Ok(trimmed
        .split(',')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect())
}

pub(crate) fn parse_font_dirs(input: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    Ok(trimmed
        .split(',')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(|part| part.to_string())
        .collect())
}

pub(crate) fn parse_timeout_ms(input: &str) -> Result<u64, Box<dyn Error>> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn parse_font_fallbacks_splits_and_trims() {
        let out = parse_font_fallbacks("A, B , ,C").expect("parse");
        assert_eq!(out, vec!["A", "B", "C"]);
    }

    #[test]
    fn parse_font_fallbacks_empty_returns_empty() {
        let out = parse_font_fallbacks(" ").expect("parse");
        assert!(out.is_empty());
    }

    #[test]
    fn parse_font_dirs_splits_and_trims() {
        let out = parse_font_dirs(" /a, , /b ").expect("parse");
        assert_eq!(out, vec!["/a", "/b"]);
    }

    #[test]
    fn parse_font_dirs_empty_returns_empty() {
        let out = parse_font_dirs(" ").expect("parse");
        assert!(out.is_empty());
    }

    #[test]
    fn parse_timeout_ms_invalid() {
        assert!(parse_timeout_ms("oops").is_err());
    }

    #[test]
    fn parse_timeout_ms_rejects_overflow() {
        let err = parse_timeout_ms("18446744073709552s").unwrap_err();
        assert!(err.to_string().contains("timeout too large"));
    }
}
