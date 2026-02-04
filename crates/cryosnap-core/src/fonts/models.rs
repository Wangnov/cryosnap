use std::collections::HashSet;
use unicode_script::Script;

#[derive(Debug, Default, Clone)]
pub(crate) struct FontFallbackNeeds {
    pub(crate) needs_unicode: bool,
    pub(crate) needs_nf: bool,
    pub(crate) needs_cjk: bool,
    pub(crate) needs_emoji: bool,
    pub(crate) scripts: HashSet<Script>,
}

#[derive(Debug, Clone)]
pub(crate) struct FontPlan {
    pub(crate) font_family: String,
    pub(crate) needs_system_fonts: bool,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct ScriptFontPlan {
    pub(crate) families: Vec<String>,
    pub(crate) downloads: Vec<ScriptDownload>,
}

#[derive(Debug, Clone)]
pub(crate) struct ScriptDownload {
    pub(crate) family: String,
    pub(crate) repo: String,
    pub(crate) file_path: String,
    pub(crate) filename: String,
    pub(crate) tag: Option<String>,
}
