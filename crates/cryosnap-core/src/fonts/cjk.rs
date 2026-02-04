use super::constants::{
    AUTO_FALLBACK_CJK_HK, AUTO_FALLBACK_CJK_JP, AUTO_FALLBACK_CJK_KR, AUTO_FALLBACK_CJK_SC,
    AUTO_FALLBACK_CJK_TC, NOTO_CJK_HK_URLS, NOTO_CJK_JP_URLS, NOTO_CJK_KR_URLS, NOTO_CJK_SC_URLS,
    NOTO_CJK_TC_URLS,
};
use super::models::FontFallbackNeeds;
use crate::{CjkRegion, Config};
use std::collections::HashSet;
use std::env;
use unicode_script::Script;

pub(crate) fn parse_cjk_region_from_locale(value: &str) -> Option<CjkRegion> {
    let raw = value.trim();
    if raw.is_empty() {
        return None;
    }
    let mut base = raw.to_ascii_lowercase();
    if let Some(pos) = base.find(['.', '@']) {
        base.truncate(pos);
    }
    let normalized = base.replace('-', "_");
    let parts = normalized
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return None;
    }
    match parts[0] {
        "ja" => return Some(CjkRegion::Jp),
        "ko" => return Some(CjkRegion::Kr),
        "zh" => {
            if parts.iter().any(|part| matches!(*part, "hk" | "mo")) {
                return Some(CjkRegion::Hk);
            }
            if parts.contains(&"tw") {
                return Some(CjkRegion::Tc);
            }
            if parts.contains(&"hant") {
                return Some(CjkRegion::Tc);
            }
            if parts.iter().any(|part| matches!(*part, "cn" | "sg")) {
                return Some(CjkRegion::Sc);
            }
            if parts.contains(&"hans") {
                return Some(CjkRegion::Sc);
            }
        }
        _ => {}
    }
    None
}

pub(crate) fn locale_cjk_region() -> Option<CjkRegion> {
    for key in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Ok(value) = env::var(key) {
            if let Some(region) = parse_cjk_region_from_locale(&value) {
                return Some(region);
            }
        }
    }
    None
}

pub(crate) fn push_cjk_region(
    out: &mut Vec<CjkRegion>,
    seen: &mut HashSet<CjkRegion>,
    region: CjkRegion,
) {
    if seen.insert(region) {
        out.push(region);
    }
}

pub(crate) fn collect_cjk_regions(config: &Config, needs: &FontFallbackNeeds) -> Vec<CjkRegion> {
    if !needs.needs_cjk {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    if needs.scripts.contains(&Script::Hiragana) || needs.scripts.contains(&Script::Katakana) {
        push_cjk_region(&mut out, &mut seen, CjkRegion::Jp);
    }
    if needs.scripts.contains(&Script::Hangul) {
        push_cjk_region(&mut out, &mut seen, CjkRegion::Kr);
    }
    if needs.scripts.contains(&Script::Bopomofo) {
        push_cjk_region(&mut out, &mut seen, CjkRegion::Tc);
    }

    if needs.scripts.contains(&Script::Han) || out.is_empty() {
        let region = match config.font.cjk_region {
            CjkRegion::Auto => locale_cjk_region().unwrap_or(CjkRegion::Sc),
            other => other,
        };
        push_cjk_region(&mut out, &mut seen, region);
    }

    out
}

pub(crate) fn cjk_region_families(region: CjkRegion) -> &'static [&'static str] {
    match region {
        CjkRegion::Sc => AUTO_FALLBACK_CJK_SC,
        CjkRegion::Tc => AUTO_FALLBACK_CJK_TC,
        CjkRegion::Hk => AUTO_FALLBACK_CJK_HK,
        CjkRegion::Jp => AUTO_FALLBACK_CJK_JP,
        CjkRegion::Kr => AUTO_FALLBACK_CJK_KR,
        CjkRegion::Auto => AUTO_FALLBACK_CJK_SC,
    }
}

pub(crate) fn cjk_region_urls(region: CjkRegion) -> &'static [&'static str] {
    match region {
        CjkRegion::Sc => NOTO_CJK_SC_URLS,
        CjkRegion::Tc => NOTO_CJK_TC_URLS,
        CjkRegion::Hk => NOTO_CJK_HK_URLS,
        CjkRegion::Jp => NOTO_CJK_JP_URLS,
        CjkRegion::Kr => NOTO_CJK_KR_URLS,
        CjkRegion::Auto => NOTO_CJK_SC_URLS,
    }
}

pub(crate) fn cjk_region_filename(region: CjkRegion) -> &'static str {
    match region {
        CjkRegion::Sc => "NotoSansCJKsc-Regular.otf",
        CjkRegion::Tc => "NotoSansCJKtc-Regular.otf",
        CjkRegion::Hk => "NotoSansCJKhk-Regular.otf",
        CjkRegion::Jp => "NotoSansCJKjp-Regular.otf",
        CjkRegion::Kr => "NotoSansCJKkr-Regular.otf",
        CjkRegion::Auto => "NotoSansCJKsc-Regular.otf",
    }
}
