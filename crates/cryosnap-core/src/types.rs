use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::Config;

#[derive(Debug, Clone)]
pub enum InputSource {
    Text(String),
    File(PathBuf),
    Command(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Svg,
    Png,
    Webp,
}

#[derive(Debug, Clone)]
pub struct RenderRequest {
    pub input: InputSource,
    pub config: Config,
    pub format: OutputFormat,
}

#[derive(Debug, Clone)]
pub struct RenderResult {
    pub format: OutputFormat,
    pub bytes: Vec<u8>,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("render error: {0}")]
    Render(String),
    #[error("execution timeout")]
    Timeout,
}

pub type Result<T> = std::result::Result<T, Error>;
