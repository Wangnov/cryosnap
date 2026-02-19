use super::constants::{
    AUTO_FALLBACK_CJK, AUTO_FALLBACK_EMOJI, AUTO_FALLBACK_GLOBAL, AUTO_FALLBACK_NF,
};
use super::dirs::resolve_font_dirs;
use super::models::{FontFallbackNeeds, FontPlan};
use crate::{Config, FontSystemFallback, Result};
use once_cell::sync::Lazy;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Mutex;

#[cfg(test)]
use std::collections::HashMap;

const FONTDB_CACHE_CAPACITY: usize = 8;
const APP_FAMILIES_CACHE_CAPACITY: usize = 8;

#[cfg(test)]
static FONTDB_BUILD_MISSES: Lazy<Mutex<HashMap<FontDbCacheKey, usize>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

#[cfg(test)]
pub(crate) fn reset_fontdb_build_miss_count_for_tests() {
    FONTDB_BUILD_MISSES.lock().expect("font miss lock").clear();
}

#[cfg(test)]
pub(crate) fn fontdb_build_miss_count_for_config(
    config: &Config,
    needs_system_fonts: bool,
) -> usize {
    let Ok(dirs) = resolve_font_dirs(config) else {
        return 0;
    };
    let key = FontDbCacheKey {
        dirs_key: dirs_cache_key(&dirs),
        font_file: config.font.file.as_ref().map(|v| font_file_key(v)),
        needs_system_fonts,
    };
    *FONTDB_BUILD_MISSES
        .lock()
        .expect("font miss lock")
        .get(&key)
        .unwrap_or(&0)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FontFileKey {
    path: String,
    len: u64,
    modified_ns: Option<u128>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FontDbCacheKey {
    dirs_key: String,
    font_file: Option<FontFileKey>,
    needs_system_fonts: bool,
}

#[derive(Default)]
struct FontCacheState {
    fontdb: VecDeque<(FontDbCacheKey, usvg::fontdb::Database)>,
    app_families: VecDeque<(String, HashSet<String>)>,
}

static FONT_CACHE: Lazy<Mutex<FontCacheState>> =
    Lazy::new(|| Mutex::new(FontCacheState::default()));

pub(crate) fn invalidate_font_caches() {
    let mut cache = FONT_CACHE.lock().expect("font cache lock");
    cache.fontdb.clear();
    cache.app_families.clear();
}

fn dirs_cache_key(dirs: &[PathBuf]) -> String {
    dirs.iter()
        .map(|dir| dir.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n")
}

fn font_file_key(path: &str) -> FontFileKey {
    let metadata = std::fs::metadata(path).ok();
    let len = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
    let modified_ns = metadata
        .and_then(|m| m.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_nanos());
    FontFileKey {
        path: path.to_string(),
        len,
        modified_ns,
    }
}

fn fontdb_cache_get(key: &FontDbCacheKey) -> Option<usvg::fontdb::Database> {
    let mut cache = FONT_CACHE.lock().expect("font cache lock");
    let pos = cache.fontdb.iter().position(|(k, _)| k == key)?;
    let (k, db) = cache.fontdb.remove(pos)?;
    let out = db.clone();
    cache.fontdb.push_back((k, db));
    Some(out)
}

fn fontdb_cache_put(key: FontDbCacheKey, db: usvg::fontdb::Database) {
    let mut cache = FONT_CACHE.lock().expect("font cache lock");
    if let Some(pos) = cache.fontdb.iter().position(|(k, _)| k == &key) {
        let _ = cache.fontdb.remove(pos);
    }
    cache.fontdb.push_back((key, db));
    while cache.fontdb.len() > FONTDB_CACHE_CAPACITY {
        cache.fontdb.pop_front();
    }
}

fn app_families_cache_get(key: &str) -> Option<HashSet<String>> {
    let mut cache = FONT_CACHE.lock().expect("font cache lock");
    let pos = cache.app_families.iter().position(|(k, _)| k == key)?;
    let (k, families) = cache.app_families.remove(pos)?;
    let out = families.clone();
    cache.app_families.push_back((k, families));
    Some(out)
}

fn app_families_cache_put(key: String, families: HashSet<String>) {
    let mut cache = FONT_CACHE.lock().expect("font cache lock");
    if let Some(pos) = cache.app_families.iter().position(|(k, _)| k == &key) {
        let _ = cache.app_families.remove(pos);
    }
    cache.app_families.push_back((key, families));
    while cache.app_families.len() > APP_FAMILIES_CACHE_CAPACITY {
        cache.app_families.pop_front();
    }
}

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
    let dirs = resolve_font_dirs(config)?;
    let dirs_key = dirs_cache_key(&dirs);
    let font_file = config.font.file.as_ref().map(|v| font_file_key(v));
    let key = FontDbCacheKey {
        dirs_key,
        font_file,
        needs_system_fonts,
    };
    if let Some(db) = fontdb_cache_get(&key) {
        return Ok(db);
    }

    #[cfg(test)]
    {
        let mut misses = FONTDB_BUILD_MISSES.lock().expect("font miss lock");
        *misses.entry(key.clone()).or_insert(0) += 1;
    }

    let mut fontdb = usvg::fontdb::Database::new();
    if let Some(font_file) = &config.font.file {
        let bytes = std::fs::read(font_file)?;
        fontdb.load_font_data(bytes);
    }
    for dir in dirs {
        if dir.is_dir() {
            fontdb.load_fonts_dir(dir);
        }
    }
    if needs_system_fonts {
        fontdb.load_system_fonts();
    }
    fontdb_cache_put(key, fontdb.clone());
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
    let dirs = resolve_font_dirs(config)?;
    let key = dirs_cache_key(&dirs);
    if let Some(families) = app_families_cache_get(&key) {
        return Ok(families);
    }

    let mut fontdb = usvg::fontdb::Database::new();
    for dir in dirs {
        if dir.is_dir() {
            fontdb.load_fonts_dir(dir);
        }
    }
    let families = collect_font_families(&fontdb);
    app_families_cache_put(key, families.clone());
    Ok(families)
}

pub(crate) fn load_system_font_families() -> HashSet<String> {
    static SYSTEM_FAMILIES: Lazy<HashSet<String>> = Lazy::new(|| {
        let mut fontdb = usvg::fontdb::Database::new();
        fontdb.load_system_fonts();
        collect_font_families(&fontdb)
    });
    SYSTEM_FAMILIES.clone()
}
