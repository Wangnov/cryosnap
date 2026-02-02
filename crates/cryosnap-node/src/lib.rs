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
