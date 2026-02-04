use crate::{Config, Error, Result};
use std::env;
use std::path::PathBuf;

pub(crate) fn resolve_font_dirs(config: &Config) -> Result<Vec<PathBuf>> {
    let raw = env::var("CRYOSNAP_FONT_DIRS").ok();
    if let Some(raw) = raw {
        return parse_font_dir_list(&raw);
    }
    if !config.font.dirs.is_empty() {
        return Ok(config
            .font
            .dirs
            .iter()
            .filter_map(|value| expand_home_dir(value))
            .collect());
    }
    Ok(vec![default_font_dir()?])
}

pub(crate) fn parse_font_dir_list(raw: &str) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for part in raw.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(path) = expand_home_dir(trimmed) {
            out.push(path);
        }
    }
    Ok(out)
}

pub(crate) fn expand_home_dir(value: &str) -> Option<PathBuf> {
    if value == "~" || value.starts_with("~/") || value.starts_with("~\\") {
        let home = home_dir()?;
        let rest = value.trim_start_matches('~');
        return Some(if rest.is_empty() {
            home
        } else {
            home.join(rest.trim_start_matches(['/', '\\']))
        });
    }
    Some(PathBuf::from(value))
}

pub(crate) fn default_font_dir() -> Result<PathBuf> {
    Ok(default_app_dir()?.join("fonts"))
}

pub(crate) fn default_app_dir() -> Result<PathBuf> {
    if let Ok(path) = env::var("CRYOSNAP_HOME") {
        return Ok(PathBuf::from(path));
    }
    let home = home_dir()
        .ok_or_else(|| Error::InvalidInput("unable to resolve home directory".to_string()))?;
    Ok(home.join(".cryosnap"))
}

pub(crate) fn home_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        if let Some(path) = env::var_os("USERPROFILE") {
            return Some(PathBuf::from(path));
        }
        if let (Some(drive), Some(path)) = (env::var_os("HOMEDRIVE"), env::var_os("HOMEPATH")) {
            return Some(PathBuf::from(drive).join(path));
        }
        None
    } else {
        env::var_os("HOME").map(PathBuf::from)
    }
}
