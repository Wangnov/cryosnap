use super::constants::{
    AUTO_FALLBACK_CJK, AUTO_FALLBACK_EMOJI, AUTO_FALLBACK_GLOBAL, AUTO_FALLBACK_NF,
};
use super::dirs::resolve_font_dirs;
use super::models::{FontFallbackNeeds, FontPlan};
use crate::{Config, FontSystemFallback, Result};
use std::collections::HashSet;

pub(crate) fn push_family(out: &mut Vec<String>, seen: &mut HashSet<String>, name: &str) {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return;
    }
    let key = trimmed.to_ascii_lowercase();
    if seen.insert(key) {
        out.push(trimmed.to_string());
    }
}

pub(crate) fn family_key(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

pub(crate) fn is_generic_family(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "serif" | "sans-serif" | "sans" | "monospace" | "cursive" | "fantasy"
    )
}

pub(crate) fn build_font_families(
    config: &Config,
    needs: &FontFallbackNeeds,
    script_families: &[String],
) -> Vec<String> {
    let mut families = Vec::new();
    let mut seen = HashSet::new();
    push_family(&mut families, &mut seen, &config.font.family);
    for name in &config.font.fallbacks {
        push_family(&mut families, &mut seen, name);
    }
    for name in script_families {
        push_family(&mut families, &mut seen, name);
    }

    if needs.needs_nf {
        for name in AUTO_FALLBACK_NF {
            push_family(&mut families, &mut seen, name);
        }
    }
    if needs.needs_cjk {
        for name in AUTO_FALLBACK_CJK {
            push_family(&mut families, &mut seen, name);
        }
    }
    if needs.needs_unicode {
        for name in AUTO_FALLBACK_GLOBAL {
            push_family(&mut families, &mut seen, name);
        }
    }
    if needs.needs_emoji {
        for name in AUTO_FALLBACK_EMOJI {
            push_family(&mut families, &mut seen, name);
        }
    }

    families
}

pub(crate) fn family_requires_system(name: &str, app_families: &HashSet<String>) -> bool {
    if is_generic_family(name) {
        return true;
    }
    let key = family_key(name);
    !app_families.contains(&key)
}

pub(crate) fn needs_system_fonts(
    config: &Config,
    app_families: &HashSet<String>,
    families: &[String],
) -> bool {
    match config.font.system_fallback {
        FontSystemFallback::Never => return false,
        FontSystemFallback::Always => return true,
        FontSystemFallback::Auto => {}
    }
    let mut needs = false;
    if config.font.file.is_none() && family_requires_system(&config.font.family, app_families) {
        needs = true;
    }
    if !needs {
        for name in families {
            if config.font.file.is_some() && name.eq_ignore_ascii_case(&config.font.family) {
                continue;
            }
            if family_requires_system(name, app_families) {
                needs = true;
                break;
            }
        }
    }
    needs
}

pub(crate) fn build_font_plan(
    config: &Config,
    needs: &FontFallbackNeeds,
    app_families: &HashSet<String>,
    script_families: &[String],
) -> FontPlan {
    let families = build_font_families(config, needs, script_families);
    let font_family = families.join(", ");
    let needs_system_fonts = needs_system_fonts(config, app_families, &families);
    FontPlan {
        font_family,
        needs_system_fonts,
    }
}

pub(crate) fn build_fontdb(
    config: &Config,
    needs_system_fonts: bool,
) -> Result<usvg::fontdb::Database> {
    let mut fontdb = usvg::fontdb::Database::new();
    if let Some(font_file) = &config.font.file {
        let bytes = std::fs::read(font_file)?;
        fontdb.load_font_data(bytes);
    }
    for dir in resolve_font_dirs(config)? {
        if dir.is_dir() {
            fontdb.load_fonts_dir(dir);
        }
    }
    if needs_system_fonts {
        fontdb.load_system_fonts();
    }
    Ok(fontdb)
}

pub(crate) fn collect_font_families(db: &usvg::fontdb::Database) -> HashSet<String> {
    let mut families = HashSet::new();
    for face in db.faces() {
        for (family, _) in &face.families {
            families.insert(family_key(family));
        }
    }
    families
}

pub(crate) fn load_app_font_families(config: &Config) -> Result<HashSet<String>> {
    let mut fontdb = usvg::fontdb::Database::new();
    for dir in resolve_font_dirs(config)? {
        if dir.is_dir() {
            fontdb.load_fonts_dir(dir);
        }
    }
    Ok(collect_font_families(&fontdb))
}

pub(crate) fn load_system_font_families() -> HashSet<String> {
    let mut fontdb = usvg::fontdb::Database::new();
    fontdb.load_system_fonts();
    collect_font_families(&fontdb)
}
