use super::cjk::{cjk_region_families, cjk_region_filename, cjk_region_urls, collect_cjk_regions};
use super::constants::{
    AUTO_FALLBACK_EMOJI, DEFAULT_GITHUB_PROXIES, NOTOFONTS_STATE_URL, NOTO_EMOJI_URLS,
};
use super::dirs::{default_app_dir, resolve_font_dirs};
use super::models::{FontFallbackNeeds, ScriptDownload, ScriptFontPlan};
use super::system::{family_key, invalidate_font_caches, load_app_font_families, load_system_font_families};
use crate::{Config, Error, FontSystemFallback, Result};
use once_cell::sync::Lazy;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NotofontsState(pub(crate) HashMap<String, NotofontsRepo>);

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NotofontsRepo {
    #[serde(default)]
    pub(crate) families: HashMap<String, NotofontsFamily>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NotofontsFamily {
    #[serde(default)]
    pub(crate) latest_release: Option<NotofontsRelease>,
    #[serde(default)]
    pub(crate) files: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct NotofontsRelease {
    pub(crate) url: String,
}

static NOTOFONTS_STATE: Lazy<Mutex<Option<Arc<NotofontsState>>>> = Lazy::new(|| Mutex::new(None));
static HTTP_AGENT: Lazy<ureq::Agent> = Lazy::new(|| {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(600))
        .build()
});

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DownloadLogLevel {
    Off = 0,
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
    Trace = 5,
}

pub(crate) fn parse_download_log_level(value: &str) -> Option<DownloadLogLevel> {
    let normalized = value.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "" => None,
        "off" | "none" | "0" | "false" | "no" => Some(DownloadLogLevel::Off),
        "error" | "err" | "1" => Some(DownloadLogLevel::Error),
        "warn" | "warning" | "2" => Some(DownloadLogLevel::Warn),
        "info" | "3" => Some(DownloadLogLevel::Info),
        "debug" | "dbg" | "4" => Some(DownloadLogLevel::Debug),
        "trace" | "5" => Some(DownloadLogLevel::Trace),
        _ => None,
    }
}

pub(crate) fn download_log_level() -> DownloadLogLevel {
    if let Ok(value) = env::var("CRYOSNAP_FONT_LOG") {
        if let Some(level) = parse_download_log_level(&value) {
            return level;
        }
    }
    if let Ok(value) = env::var("CRYOSNAP_LOG") {
        if let Some(level) = parse_download_log_level(&value) {
            return level;
        }
    }
    DownloadLogLevel::Info
}

pub(crate) fn download_log(level: DownloadLogLevel, message: impl AsRef<str>) {
    if matches!(level, DownloadLogLevel::Off) {
        return;
    }
    let current = download_log_level();
    if level > current {
        return;
    }
    let label = match level {
        DownloadLogLevel::Error => "error",
        DownloadLogLevel::Warn => "warn",
        DownloadLogLevel::Info => "info",
        DownloadLogLevel::Debug => "debug",
        DownloadLogLevel::Trace => "trace",
        DownloadLogLevel::Off => "off",
    };
    eprintln!("cryosnap [{label}]: {}", message.as_ref());
}

pub(crate) fn auto_download_enabled(config: &Config) -> bool {
    if let Ok(value) = env::var("CRYOSNAP_FONT_AUTO_DOWNLOAD") {
        let value = value.trim().to_ascii_lowercase();
        return !(value == "0" || value == "false" || value == "no" || value == "off");
    }
    config.font.auto_download
}

pub(crate) fn force_update_enabled(config: &Config) -> bool {
    if let Ok(value) = env::var("CRYOSNAP_FONT_FORCE_UPDATE") {
        let value = value.trim().to_ascii_lowercase();
        return !(value == "0" || value == "false" || value == "no" || value == "off");
    }
    config.font.force_update
}

pub(crate) fn github_proxy_candidates() -> Vec<String> {
    if let Ok(value) = env::var("CRYOSNAP_GITHUB_PROXY") {
        let parts = value
            .split(',')
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .collect::<Vec<_>>();
        if !parts.is_empty() {
            return parts;
        }
    }
    DEFAULT_GITHUB_PROXIES
        .iter()
        .map(|v| v.to_string())
        .collect()
}

pub(crate) fn cache_dir() -> Result<PathBuf> {
    Ok(default_app_dir()?.join("cache"))
}

pub(crate) fn apply_github_proxy(url: &str, proxy: &str) -> String {
    let mut base = proxy.trim().to_string();
    if !base.ends_with('/') {
        base.push('/');
    }
    format!("{base}{url}")
}

pub(crate) enum FetchOutcome {
    Ok(Box<ureq::Response>, Option<String>),
    NotModified,
}

pub(crate) fn build_github_candidates() -> Vec<Option<String>> {
    let mut seen = HashSet::new();
    let mut candidates = Vec::new();

    candidates.push(None);
    for proxy in github_proxy_candidates() {
        if seen.insert(proxy.clone()) {
            candidates.push(Some(proxy));
        }
    }
    candidates
}

pub(crate) fn looks_like_json(bytes: &[u8]) -> bool {
    for &b in bytes {
        if !b.is_ascii_whitespace() {
            return b == b'{' || b == b'[';
        }
    }
    false
}

pub(crate) fn fetch_with_candidates(url: &str, headers: &[(&str, &str)]) -> Result<FetchOutcome> {
    let candidates = build_github_candidates();

    let mut last_error: Option<String> = None;
    for proxy_opt in candidates {
        let target = match &proxy_opt {
            Some(proxy) => apply_github_proxy(url, proxy),
            None => url.to_string(),
        };
        download_log(
            DownloadLogLevel::Debug,
            format!(
                "fetching {} via {}",
                url,
                proxy_opt.as_deref().unwrap_or("direct")
            ),
        );
        let mut req = HTTP_AGENT
            .get(&target)
            .set("User-Agent", "cryosnap/auto-font");
        for (key, value) in headers {
            req = req.set(key, value);
        }
        match req.call() {
            Ok(resp) => {
                if resp.status() == 304 {
                    return Ok(FetchOutcome::NotModified);
                }
                return Ok(FetchOutcome::Ok(Box::new(resp), proxy_opt.clone()));
            }
            Err(ureq::Error::Status(304, _)) => return Ok(FetchOutcome::NotModified),
            Err(err) => {
                last_error = Some(format!("{err}"));
                download_log(
                    DownloadLogLevel::Debug,
                    format!(
                        "fetch failed via {}: {}",
                        proxy_opt.as_deref().unwrap_or("direct"),
                        last_error.as_deref().unwrap_or("unknown error")
                    ),
                );
                continue;
            }
        }
    }
    Err(Error::Render(format!(
        "download failed: {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    )))
}

pub(crate) fn fetch_bytes_with_cache(
    url: &str,
    cache_name: &str,
    force_update: bool,
) -> Result<Vec<u8>> {
    let cache_dir = cache_dir()?;
    let data_path = cache_dir.join(cache_name);
    let etag_path = cache_dir.join(format!("{cache_name}.etag"));
    let mut headers: Vec<(&str, &str)> = Vec::new();
    let mut etag_holder: Option<String> = None;
    if data_path.exists() && !force_update {
        if let Ok(etag) = fs::read_to_string(&etag_path) {
            let tag = etag.trim().to_string();
            if !tag.is_empty() {
                etag_holder = Some(tag);
            }
        }
    }
    if let Some(tag) = etag_holder.as_ref() {
        headers.push(("If-None-Match", tag.as_str()));
    }
    let candidates = build_github_candidates();
    let mut last_error: Option<String> = None;
    for proxy_opt in candidates {
        let target = match &proxy_opt {
            Some(proxy) => apply_github_proxy(url, proxy),
            None => url.to_string(),
        };
        download_log(
            DownloadLogLevel::Debug,
            format!(
                "fetching {} via {}",
                url,
                proxy_opt.as_deref().unwrap_or("direct")
            ),
        );
        let mut req = HTTP_AGENT
            .get(&target)
            .set("User-Agent", "cryosnap/auto-font");
        for (key, value) in &headers {
            req = req.set(key, value);
        }
        match req.call() {
            Ok(resp) => {
                if resp.status() == 304 {
                    if data_path.exists() {
                        let cached = fs::read(&data_path)?;
                        if looks_like_json(&cached) {
                            return Ok(cached);
                        }
                        let _ = fs::remove_file(&data_path);
                        let _ = fs::remove_file(&etag_path);
                    }
                    last_error = Some("font state cache missing".to_string());
                    continue;
                }
                let etag_value = resp.header("ETag").map(|v| v.to_string());
                let mut reader = resp.into_reader();
                let mut buf = Vec::new();
                reader.read_to_end(&mut buf)?;
                if !looks_like_json(&buf) {
                    last_error = Some("invalid response".to_string());
                    continue;
                }
                if let Some(parent) = data_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&data_path, &buf)?;
                if let Some(etag) = etag_value {
                    let _ = fs::write(&etag_path, etag.as_bytes());
                }
                return Ok(buf);
            }
            Err(ureq::Error::Status(304, _)) => {
                if data_path.exists() {
                    let cached = fs::read(&data_path)?;
                    if looks_like_json(&cached) {
                        return Ok(cached);
                    }
                    let _ = fs::remove_file(&data_path);
                    let _ = fs::remove_file(&etag_path);
                }
                last_error = Some("font state cache missing".to_string());
                continue;
            }
            Err(err) => {
                last_error = Some(format!("{err}"));
                download_log(
                    DownloadLogLevel::Debug,
                    format!(
                        "fetch failed via {}: {}",
                        proxy_opt.as_deref().unwrap_or("direct"),
                        last_error.as_deref().unwrap_or("unknown error")
                    ),
                );
                continue;
            }
        }
    }
    if data_path.exists() {
        let cached = fs::read(&data_path)?;
        if looks_like_json(&cached) {
            return Ok(cached);
        }
        let _ = fs::remove_file(&data_path);
        let _ = fs::remove_file(&etag_path);
    }
    Err(Error::Render(format!(
        "download failed: {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    )))
}

pub(crate) fn download_url_with_etag(url: &str, target: &Path, force_update: bool) -> Result<bool> {
    let etag_path = target.with_extension(format!(
        "{}.etag",
        target
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("font")
    ));
    let mut headers: Vec<(&str, &str)> = Vec::new();
    let mut etag_holder: Option<String> = None;
    if target.exists() && !force_update {
        if let Ok(etag) = fs::read_to_string(&etag_path) {
            let tag = etag.trim().to_string();
            if !tag.is_empty() {
                etag_holder = Some(tag);
            }
        }
    }
    if let Some(tag) = etag_holder.as_ref() {
        headers.push(("If-None-Match", tag.as_str()));
    }
    match fetch_with_candidates(url, &headers)? {
        FetchOutcome::Ok(resp, _proxy) => {
            let etag_value = resp.header("ETag").map(|v| v.to_string());
            let mut reader = resp.into_reader();
            let temp = target.with_extension("download");
            let mut file = fs::File::create(&temp)?;
            std::io::copy(&mut reader, &mut file)?;
            file.sync_all()?;
            fs::rename(&temp, target)?;
            if let Some(etag) = etag_value {
                let _ = fs::write(&etag_path, etag.as_bytes());
            }
            Ok(true)
        }
        FetchOutcome::NotModified => Ok(false),
    }
}

pub(crate) fn load_notofonts_state(force_update: bool) -> Result<Arc<NotofontsState>> {
    if !force_update {
        if let Ok(guard) = NOTOFONTS_STATE.lock() {
            if let Some(state) = guard.as_ref() {
                return Ok(state.clone());
            }
        }
    }
    let bytes = fetch_bytes_with_cache(NOTOFONTS_STATE_URL, "notofonts_state.json", force_update)?;
    let state: NotofontsState = serde_json::from_slice(&bytes)
        .map_err(|err| Error::Render(format!("font state parse: {err}")))?;
    let state = Arc::new(state);
    if !force_update {
        if let Ok(mut guard) = NOTOFONTS_STATE.lock() {
            *guard = Some(state.clone());
        }
    }
    Ok(state)
}

pub(crate) struct FontPackage {
    id: &'static str,
    family: &'static str,
    filename: &'static str,
    url: &'static str,
    download_sha256: &'static str,
    file_sha256: &'static str,
    archive_entry: Option<&'static str>,
}

pub(crate) const FONT_PACKAGE_NF: FontPackage = FontPackage {
    id: "symbols-nerd-font-mono",
    family: "Symbols Nerd Font Mono",
    filename: "SymbolsNerdFontMono-Regular.ttf",
    url:
        "https://github.com/ryanoasis/nerd-fonts/releases/download/v3.2.1/NerdFontsSymbolsOnly.zip",
    download_sha256: "bc59c2ea74d022a6262ff9e372fde5c36cd5ae3f82a567941489ecfab4f03d66",
    file_sha256: "6f7e339af33bde250a4d7360a3176ab1ffe4e99c00eef0d71b4c322364c595f3",
    archive_entry: Some("SymbolsNerdFontMono-Regular.ttf"),
};

pub(crate) fn ensure_fonts_available(
    config: &Config,
    needs: &FontFallbackNeeds,
    script_plan: &ScriptFontPlan,
) -> Result<()> {
    if !auto_download_enabled(config) {
        download_log(DownloadLogLevel::Debug, "auto-download disabled");
        return Ok(());
    }
    let force_update = force_update_enabled(config);
    if !needs.needs_nf && !needs.needs_cjk && !needs.needs_emoji && script_plan.downloads.is_empty()
    {
        download_log(DownloadLogLevel::Debug, "no font downloads required");
        return Ok(());
    }
    let mut downloaded_any = false;
    let font_dirs = resolve_font_dirs(config)?;
    let Some(primary_dir) = font_dirs.first() else {
        return Ok(());
    };
    let app_families = load_app_font_families(config).unwrap_or_default();
    let allow_system = !matches!(config.font.system_fallback, FontSystemFallback::Never);
    let system_families = if allow_system {
        load_system_font_families()
    } else {
        HashSet::new()
    };

    fs::create_dir_all(primary_dir)?;

    for download in &script_plan.downloads {
        let app_has = app_families.contains(&family_key(&download.family));
        let system_has = allow_system && system_families.contains(&family_key(&download.family));
        if system_has && !app_has {
            continue;
        }
        if app_has {
            if !force_update {
                continue;
            }
            let target = primary_dir.join(&download.filename);
            if !target.exists() {
                continue;
            }
        }
        match download_notofonts_file(download, primary_dir, force_update) {
            Ok(true) => {
                downloaded_any = true;
                download_log(
                    DownloadLogLevel::Info,
                    format!("downloaded font {}", download.family),
                )
            }
            Ok(false) => download_log(
                DownloadLogLevel::Debug,
                format!("font up-to-date {}", download.family),
            ),
            Err(err) => download_log(
                DownloadLogLevel::Warn,
                format!("font download failed for {}: {}", download.family, err),
            ),
        }
    }

    if needs.needs_nf
        && !any_family_present(&[FONT_PACKAGE_NF.family], &app_families)
        && !(allow_system && any_family_present(&[FONT_PACKAGE_NF.family], &system_families))
    {
        match download_font_package(&FONT_PACKAGE_NF, primary_dir) {
            Ok(true) => {
                downloaded_any = true;
                download_log(
                    DownloadLogLevel::Info,
                    format!("downloaded font {}", FONT_PACKAGE_NF.family),
                )
            }
            Ok(false) => download_log(
                DownloadLogLevel::Debug,
                format!("font up-to-date {}", FONT_PACKAGE_NF.family),
            ),
            Err(err) => download_log(
                DownloadLogLevel::Warn,
                format!("font download failed for {}: {}", FONT_PACKAGE_NF.id, err),
            ),
        }
    }

    if needs.needs_cjk {
        let cjk_regions = collect_cjk_regions(config, needs);
        for region in cjk_regions {
            let families = cjk_region_families(region);
            let app_has = any_family_present(families, &app_families);
            let system_has = allow_system && any_family_present(families, &system_families);
            if system_has && !app_has {
                continue;
            }
            let filename = cjk_region_filename(region);
            let target = primary_dir.join(filename);
            if app_has {
                if !force_update {
                    continue;
                }
                if !target.exists() {
                    continue;
                }
            }
            match download_raw_font(cjk_region_urls(region), primary_dir, filename, force_update) {
                Ok(true) => {
                    downloaded_any = true;
                    download_log(
                        DownloadLogLevel::Info,
                        format!("downloaded font {}", filename),
                    )
                }
                Ok(false) => download_log(
                    DownloadLogLevel::Debug,
                    format!("font up-to-date {}", filename),
                ),
                Err(err) => download_log(
                    DownloadLogLevel::Warn,
                    format!("font download failed for cjk: {}", err),
                ),
            }
        }
    }

    if needs.needs_emoji {
        let app_has = any_family_present(AUTO_FALLBACK_EMOJI, &app_families);
        let system_has = allow_system && any_family_present(AUTO_FALLBACK_EMOJI, &system_families);
        if !system_has || app_has {
            let filename = "NotoColorEmoji.ttf";
            let target = primary_dir.join(filename);
            if !app_has || (force_update && target.exists()) {
                match download_raw_font(NOTO_EMOJI_URLS, primary_dir, filename, force_update) {
                    Ok(true) => {
                        downloaded_any = true;
                        download_log(DownloadLogLevel::Info, "downloaded font Noto Color Emoji")
                    }
                    Ok(false) => {
                        download_log(DownloadLogLevel::Debug, "font up-to-date Noto Color Emoji")
                    }
                    Err(err) => download_log(
                        DownloadLogLevel::Warn,
                        format!("font download failed for emoji: {}", err),
                    ),
                }
            }
        }
    }

    if downloaded_any {
        invalidate_font_caches();
    }
    Ok(())
}

pub(crate) fn any_family_present(families: &[&str], set: &HashSet<String>) -> bool {
    families.iter().any(|name| set.contains(&family_key(name)))
}

pub(crate) fn download_raw_font(
    urls: &[&str],
    dir: &Path,
    filename: &str,
    force_update: bool,
) -> Result<bool> {
    let target = dir.join(filename);
    let mut last_error: Option<Error> = None;
    for url in urls {
        match download_url_with_etag(url, &target, force_update) {
            Ok(downloaded) => return Ok(downloaded),
            Err(err) => {
                last_error = Some(err);
                download_log(
                    DownloadLogLevel::Debug,
                    format!(
                        "download failed from {}: {}",
                        url,
                        last_error.as_ref().unwrap()
                    ),
                );
            }
        }
    }
    Err(last_error
        .unwrap_or_else(|| Error::Render("font download failed: no available urls".to_string())))
}

pub(crate) fn download_notofonts_file(
    download: &ScriptDownload,
    dir: &Path,
    force_update: bool,
) -> Result<bool> {
    let target = dir.join(&download.filename);
    if !force_update && target.exists() {
        return Ok(false);
    }
    let mut refs = vec!["main".to_string(), "master".to_string()];
    if let Some(tag) = &download.tag {
        if !tag.is_empty() {
            refs.push(tag.clone());
        }
    }
    let mut last_error: Option<Error> = None;
    for reference in refs {
        let url = format!(
            "https://raw.githubusercontent.com/{}/{}/{}",
            download.repo, reference, download.file_path
        );
        match download_url_with_etag(&url, &target, force_update) {
            Ok(downloaded) => return Ok(downloaded),
            Err(err) => {
                last_error = Some(err);
                download_log(
                    DownloadLogLevel::Debug,
                    format!(
                        "download failed from {}: {}",
                        url,
                        last_error.as_ref().unwrap()
                    ),
                );
            }
        }
    }
    Err(last_error
        .unwrap_or_else(|| Error::Render("font download failed: no available refs".to_string())))
}

pub(crate) fn download_url_to_file(url: &str, target: &Path) -> Result<()> {
    match fetch_with_candidates(url, &[])? {
        FetchOutcome::Ok(resp, _proxy) => {
            let mut reader = resp.into_reader();
            let mut file = fs::File::create(target)?;
            std::io::copy(&mut reader, &mut file)?;
            file.sync_all()?;
            Ok(())
        }
        FetchOutcome::NotModified => Ok(()),
    }
}

pub(crate) fn download_zip_with_candidates(url: &str, target: &Path) -> Result<()> {
    let candidates = build_github_candidates();
    let mut last_error: Option<String> = None;
    for proxy_opt in candidates {
        let target_url = match &proxy_opt {
            Some(proxy) => apply_github_proxy(url, proxy),
            None => url.to_string(),
        };
        download_log(
            DownloadLogLevel::Debug,
            format!(
                "fetching {} via {}",
                url,
                proxy_opt.as_deref().unwrap_or("direct")
            ),
        );
        let req = HTTP_AGENT
            .get(&target_url)
            .set("User-Agent", "cryosnap/auto-font");
        match req.call() {
            Ok(resp) => {
                let temp = target.with_extension("download");
                let mut reader = resp.into_reader();
                let mut file = fs::File::create(&temp)?;
                std::io::copy(&mut reader, &mut file)?;
                file.sync_all()?;
                if let Err(err) = validate_zip_archive(&temp) {
                    last_error = Some(err.to_string());
                    let _ = fs::remove_file(&temp);
                    continue;
                }
                fs::rename(&temp, target)?;
                return Ok(());
            }
            Err(ureq::Error::Status(status, _)) => {
                last_error = Some(format!("status {status}"));
                download_log(
                    DownloadLogLevel::Debug,
                    format!(
                        "fetch failed via {}: {}",
                        proxy_opt.as_deref().unwrap_or("direct"),
                        last_error.as_deref().unwrap_or("unknown error")
                    ),
                );
                continue;
            }
            Err(err) => {
                last_error = Some(format!("{err}"));
                download_log(
                    DownloadLogLevel::Debug,
                    format!(
                        "fetch failed via {}: {}",
                        proxy_opt.as_deref().unwrap_or("direct"),
                        last_error.as_deref().unwrap_or("unknown error")
                    ),
                );
                continue;
            }
        }
    }
    Err(Error::Render(format!(
        "download failed: {}",
        last_error.unwrap_or_else(|| "unknown error".to_string())
    )))
}

pub(crate) fn validate_zip_archive(path: &Path) -> Result<()> {
    let file = fs::File::open(path)?;
    match zip::ZipArchive::new(file) {
        Ok(_) => Ok(()),
        Err(err) => Err(Error::Render(format!("zip read: {err}"))),
    }
}

pub(crate) fn download_font_package(pkg: &FontPackage, dir: &Path) -> Result<bool> {
    let target = dir.join(pkg.filename);
    if verify_sha256(&target, pkg.file_sha256)? {
        return Ok(false);
    }
    let temp = dir.join(format!("{}.download", pkg.filename));
    if pkg.archive_entry.is_some() {
        download_zip_with_candidates(pkg.url, &temp)?;
    } else {
        download_url_to_file(pkg.url, &temp)?;
    }
    if !verify_sha256(&temp, pkg.download_sha256)? {
        let _ = fs::remove_file(&temp);
        return Err(Error::Render(format!(
            "font checksum mismatch for {}",
            pkg.id
        )));
    }
    if let Some(entry) = pkg.archive_entry {
        extract_zip_entry(&temp, entry, &target)?;
        let _ = fs::remove_file(&temp);
    } else {
        fs::rename(&temp, &target)?;
    }
    if !verify_sha256(&target, pkg.file_sha256)? {
        return Err(Error::Render(format!(
            "font checksum mismatch for {}",
            pkg.id
        )));
    }
    Ok(true)
}

pub(crate) fn extract_zip_entry(archive_path: &Path, entry: &str, target: &Path) -> Result<()> {
    let file = fs::File::open(archive_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|err| Error::Render(format!("zip read: {err}")))?;
    let mut entry_file = archive
        .by_name(entry)
        .map_err(|err| Error::Render(format!("zip entry {entry}: {err}")))?;
    let mut out = fs::File::create(target)?;
    std::io::copy(&mut entry_file, &mut out)?;
    out.sync_all()?;
    Ok(())
}

pub(crate) fn verify_sha256(path: &Path, expected: &str) -> Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    if expected.trim().is_empty() {
        return Ok(true);
    }
    let actual = sha256_hex(path)?;
    Ok(actual.eq_ignore_ascii_case(expected.trim()))
}

pub(crate) fn sha256_hex(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{:02x}", byte));
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) fn set_notofonts_state(
    state: Option<Arc<NotofontsState>>,
) -> Option<Arc<NotofontsState>> {
    let mut guard = NOTOFONTS_STATE.lock().expect("state lock");
    let prev = guard.clone();
    *guard = state;
    prev
}
