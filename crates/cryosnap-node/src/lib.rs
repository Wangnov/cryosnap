use cryosnap_core::{Config, InputSource, OutputFormat, RenderRequest};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::PathBuf;

#[napi(object)]
pub struct RenderOptions {
    pub input: String,
    pub input_kind: Option<String>,
    pub config_json: Option<String>,
    pub format: Option<String>,
}

#[napi]
pub fn render(options: RenderOptions) -> Result<Buffer> {
    let config = match options.config_json {
        Some(json) => serde_json::from_str::<Config>(&json)
            .map_err(|err| Error::new(Status::InvalidArg, err.to_string()))?,
        None => Config::default(),
    };

    let input = match options.input_kind.as_deref() {
        Some("file") => InputSource::File(PathBuf::from(options.input)),
        Some("command") => InputSource::Command(options.input),
        _ => InputSource::Text(options.input),
    };

    let format = match options.format.as_deref() {
        Some("png") => OutputFormat::Png,
        Some("webp") => OutputFormat::Webp,
        _ => OutputFormat::Svg,
    };

    let request = RenderRequest {
        input,
        config,
        format,
    };

    let result = cryosnap_core::render(&request)
        .map_err(|err| Error::new(Status::GenericFailure, err.to_string()))?;

    Ok(Buffer::from(result.bytes))
}

#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!(
            "cryosnap-node-{}-{}-{}",
            prefix,
            std::process::id(),
            id
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create dir");
        path
    }

    fn with_auto_download_disabled() -> Option<String> {
        let prev = std::env::var("CRYOSNAP_FONT_AUTO_DOWNLOAD").ok();
        std::env::set_var("CRYOSNAP_FONT_AUTO_DOWNLOAD", "0");
        prev
    }

    fn restore_env(key: &str, value: Option<String>) {
        match value {
            Some(val) => std::env::set_var(key, val),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn version_non_empty() {
        assert!(!version().is_empty());
    }

    #[test]
    fn render_svg_from_text_default() {
        let prev = with_auto_download_disabled();
        let options = RenderOptions {
            input: "hello".to_string(),
            input_kind: None,
            config_json: None,
            format: None,
        };
        let out = render(options).expect("render");
        assert!(out.as_ref().starts_with(b"<svg"));
        restore_env("CRYOSNAP_FONT_AUTO_DOWNLOAD", prev);
    }

    #[test]
    fn render_png_from_file() {
        let prev = with_auto_download_disabled();
        let temp = temp_dir("input");
        let path = temp.join("input.txt");
        std::fs::write(&path, "hello").expect("write");
        let options = RenderOptions {
            input: path.to_string_lossy().to_string(),
            input_kind: Some("file".to_string()),
            config_json: None,
            format: Some("png".to_string()),
        };
        let out = render(options).expect("render");
        assert!(out.as_ref().starts_with(b"\x89PNG"));
        restore_env("CRYOSNAP_FONT_AUTO_DOWNLOAD", prev);
        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn render_webp_from_text() {
        let prev = with_auto_download_disabled();
        let options = RenderOptions {
            input: "hello".to_string(),
            input_kind: None,
            config_json: None,
            format: Some("webp".to_string()),
        };
        let out = render(options).expect("render");
        assert!(out.as_ref().starts_with(b"RIFF"));
        restore_env("CRYOSNAP_FONT_AUTO_DOWNLOAD", prev);
    }

    #[test]
    fn render_rejects_invalid_config_json() {
        let options = RenderOptions {
            input: "hello".to_string(),
            input_kind: None,
            config_json: Some("{bad json}".to_string()),
            format: None,
        };
        let err = render(options).err().expect("expected error");
        assert!(!err.to_string().is_empty());
    }
}
