use cryosnap_core::Config;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::{env, fs};

const BASE_CONFIG: &str = include_str!("../configurations/base.json");
const FULL_CONFIG: &str = include_str!("../configurations/full.json");

pub(crate) fn load_config(config_arg: Option<&str>) -> Result<(Config, bool), Box<dyn Error>> {
    let name = config_arg.unwrap_or("default");
    let is_default = name == "default";

    let config = match name {
        "default" | "base" => serde_json::from_str(BASE_CONFIG)?,
        "full" => serde_json::from_str(FULL_CONFIG)?,
        "user" => load_user_config()?,
        _ => {
            let contents = fs::read_to_string(name)?;
            serde_json::from_str(&contents)?
        }
    };
    Ok((config, is_default))
}

pub(crate) fn load_user_config() -> Result<Config, Box<dyn Error>> {
    let path = user_config_path()?;
    if !path.exists() && !env_config_overridden() {
        migrate_legacy_user_config(&path)?;
    }
    if path.exists() {
        let contents = fs::read_to_string(&path)?;
        return Ok(serde_json::from_str(&contents)?);
    }
    serde_json::from_str(BASE_CONFIG).map_err(|err| err.into())
}

pub(crate) fn save_user_config(config: &Config) -> Result<(), Box<dyn Error>> {
    let path = user_config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, serde_json::to_string_pretty(config)?)?;
    Ok(())
}

pub(crate) fn user_config_path() -> Result<PathBuf, Box<dyn Error>> {
    if let Ok(path) = env::var("CRYOSNAP_CONFIG_PATH") {
        return Ok(PathBuf::from(path));
    }
    let config_dir = if let Ok(path) = env::var("CRYOSNAP_CONFIG_DIR") {
        PathBuf::from(path)
    } else {
        default_config_dir()?
    };
    Ok(config_dir.join("user.json"))
}

fn env_config_overridden() -> bool {
    env::var("CRYOSNAP_CONFIG_PATH").is_ok() || env::var("CRYOSNAP_CONFIG_DIR").is_ok()
}

fn default_app_dir() -> Result<PathBuf, Box<dyn Error>> {
    if let Ok(path) = env::var("CRYOSNAP_HOME") {
        return Ok(PathBuf::from(path));
    }
    let home = if cfg!(windows) {
        if let Some(path) = env::var_os("USERPROFILE") {
            PathBuf::from(path)
        } else if let (Some(drive), Some(path)) =
            (env::var_os("HOMEDRIVE"), env::var_os("HOMEPATH"))
        {
            PathBuf::from(drive).join(path)
        } else {
            return Err("unable to resolve home directory".into());
        }
    } else if let Some(path) = env::var_os("HOME") {
        PathBuf::from(path)
    } else {
        return Err("unable to resolve home directory".into());
    };
    Ok(home.join(".cryosnap"))
}

fn default_config_dir() -> Result<PathBuf, Box<dyn Error>> {
    Ok(default_app_dir()?.join("config"))
}

fn legacy_user_config_path() -> Option<PathBuf> {
    let project = directories::ProjectDirs::from("sh", "cryosnap", "cryosnap")?;
    Some(project.config_dir().join("user.json"))
}

fn migrate_legacy_user_config(target_path: &Path) -> Result<(), Box<dyn Error>> {
    if target_path.exists() {
        return Ok(());
    }
    let legacy = legacy_user_config_path();
    let Some(legacy) = legacy else {
        return Ok(());
    };
    if !legacy.exists() {
        return Ok(());
    }
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&legacy, target_path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::env_lock;
    use tempfile::tempdir;

    #[test]
    fn load_config_default() {
        let (cfg, is_default) = load_config(None).expect("load config");
        assert!(is_default);
        assert_eq!(cfg.theme, "charm");
    }

    #[test]
    fn load_config_full() {
        let (cfg, is_default) = load_config(Some("full")).expect("load config");
        assert!(!is_default);
        assert!(cfg.window_controls);
        assert_eq!(cfg.border.radius, 8.0);
    }

    #[test]
    fn load_config_user_fallback() {
        let _lock = env_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        env::set_var("CRYOSNAP_CONFIG_DIR", dir.path());
        let (cfg, _) = load_config(Some("user")).expect("load config");
        assert!(!cfg.window_controls);
        env::remove_var("CRYOSNAP_CONFIG_DIR");
    }

    #[test]
    fn load_config_missing_errors() {
        let err = load_config(Some("does-not-exist")).err();
        assert!(err.is_some());
    }

    #[test]
    fn load_config_from_path() {
        let _lock = env_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("custom.json");
        fs::write(&path, r#"{"theme":"dracula"}"#).expect("write");
        let (cfg, _) = load_config(Some(path.to_str().unwrap())).expect("load config");
        assert_eq!(cfg.theme, "dracula");
    }

    #[test]
    fn save_and_load_user_config() {
        let _lock = env_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        env::set_var("CRYOSNAP_CONFIG_DIR", dir.path());
        let cfg = Config {
            theme: "custom".to_string(),
            ..Config::default()
        };
        save_user_config(&cfg).expect("save");
        let loaded = load_user_config().expect("load");
        assert_eq!(loaded.theme, "custom");
        env::remove_var("CRYOSNAP_CONFIG_DIR");
    }

    #[test]
    fn default_app_dir_uses_env() {
        let _lock = env_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        let prev = env::var("CRYOSNAP_HOME").ok();
        env::set_var("CRYOSNAP_HOME", dir.path());
        let out = default_app_dir().expect("dir");
        assert_eq!(out, dir.path());
        if let Some(value) = prev {
            env::set_var("CRYOSNAP_HOME", value);
        } else {
            env::remove_var("CRYOSNAP_HOME");
        }
    }

    #[test]
    fn migrate_legacy_user_config_copies() {
        let _lock = env_lock().lock().expect("lock");
        let legacy_root = tempdir().expect("temp dir");
        let target_root = tempdir().expect("temp dir");
        let prev = env::var("XDG_CONFIG_HOME").ok();
        env::set_var("XDG_CONFIG_HOME", legacy_root.path());

        let legacy_path = legacy_user_config_path().expect("legacy path");
        if let Some(parent) = legacy_path.parent() {
            fs::create_dir_all(parent).expect("create legacy dir");
        }
        fs::write(&legacy_path, r#"{\"theme\":\"custom\"}"#).expect("write");

        let target_path = target_root.path().join("user.json");
        migrate_legacy_user_config(&target_path).expect("migrate");
        let content = fs::read_to_string(&target_path).expect("read");
        assert!(content.contains("custom"));

        if let Some(value) = prev {
            env::set_var("XDG_CONFIG_HOME", value);
        } else {
            env::remove_var("XDG_CONFIG_HOME");
        }
    }
}
