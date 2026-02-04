use super::constants::NOTOFONTS_FILES_REPO;
use super::download::{
    force_update_enabled, load_notofonts_state, NotofontsFamily, NotofontsState,
};
use super::models::{FontFallbackNeeds, ScriptDownload, ScriptFontPlan};
use crate::{Config, Result};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use unicode_script::Script;

#[derive(Debug, Clone, Copy)]
pub(crate) enum FontStylePreference {
    Sans,
    Serif,
}

pub(crate) fn fallback_style_preference(config: &Config) -> FontStylePreference {
    let family = config.font.family.trim().to_ascii_lowercase();
    if matches!(family.as_str(), "serif") || family.contains("serif") {
        FontStylePreference::Serif
    } else {
        FontStylePreference::Sans
    }
}

pub(crate) fn normalize_repo_key(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

pub(crate) fn build_repo_key_index(state: &NotofontsState) -> HashMap<String, String> {
    let mut index = HashMap::new();
    for key in state.0.keys() {
        index.insert(normalize_repo_key(key), key.clone());
    }
    index
}

pub(crate) fn is_cjk_script(script: Script) -> bool {
    matches!(
        script,
        Script::Han | Script::Hiragana | Script::Katakana | Script::Hangul | Script::Bopomofo
    )
}

pub(crate) fn script_repo_key(script: Script, index: &HashMap<String, String>) -> Option<String> {
    match script {
        Script::Common | Script::Inherited | Script::Unknown => return None,
        Script::Latin | Script::Greek | Script::Cyrillic => {
            return Some("latin-greek-cyrillic".to_string())
        }
        _ => {}
    }
    if is_cjk_script(script) {
        return None;
    }
    let name = script.full_name();
    let normalized = normalize_repo_key(name);
    index.get(&normalized).cloned()
}

pub(crate) fn choose_family_name(
    families: &HashMap<String, NotofontsFamily>,
    style: FontStylePreference,
) -> Option<String> {
    let candidates = families
        .iter()
        .filter(|(_, info)| info.latest_release.is_some() || !info.files.is_empty())
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }
    let mut best: Option<(i32, &String)> = None;
    for name in &candidates {
        let score = score_family_name(name, style);
        match best {
            None => best = Some((score, name)),
            Some((best_score, best_name)) => {
                if score > best_score
                    || (score == best_score && name.len() < best_name.len())
                    || (score == best_score && name.len() == best_name.len() && name < best_name)
                {
                    best = Some((score, name));
                }
            }
        }
    }
    best.map(|(_, name)| name.clone())
}

pub(crate) fn tag_from_release_url(url: &str) -> Option<String> {
    url.rsplit('/').next().map(|v| v.to_string())
}

pub(crate) fn score_family_name(name: &str, style: FontStylePreference) -> i32 {
    let lower = name.to_ascii_lowercase();
    let mut score = 0;
    match style {
        FontStylePreference::Sans => {
            if lower.contains("noto sans") {
                score += 300;
            } else if lower.contains("sans") {
                score += 200;
            }
            if lower.contains("kufi") {
                score += 120;
            }
        }
        FontStylePreference::Serif => {
            if lower.contains("noto serif") {
                score += 300;
            } else if lower.contains("serif") {
                score += 200;
            }
            if lower.contains("naskh") {
                score += 120;
            }
        }
    }
    if lower.contains("supplement") {
        score -= 200;
    }
    if lower.contains("looped") {
        score -= 120;
    }
    if lower.contains("display") {
        score -= 40;
    }
    if lower.contains("ui") {
        score -= 20;
    }
    score
}

pub(crate) fn repo_from_release_url(url: &str) -> Option<String> {
    let suffix = url.split("github.com/").nth(1)?;
    let mut parts = suffix.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if owner.is_empty() || repo.is_empty() {
        return None;
    }
    Some(format!("{owner}/{repo}"))
}

pub(crate) fn score_font_path(path: &str) -> Option<i32> {
    let lower = path.to_ascii_lowercase();
    let ext_score = if lower.ends_with(".ttf") {
        100
    } else if lower.ends_with(".otf") {
        80
    } else {
        return None;
    };
    let mut score = ext_score;
    if lower.contains("/full/") {
        score += 60;
    }
    if lower.contains("/hinted/") {
        score += 45;
    }
    if lower.contains("/googlefonts/") {
        score += 30;
    }
    if lower.contains("/unhinted/") {
        score += 10;
    }
    if lower.contains("regular") {
        score += 200;
    }
    if lower.contains("italic") {
        score -= 120;
    }
    if lower.contains("variable") || lower.contains('[') {
        score -= 20;
    }
    if lower.contains("slim") {
        score -= 10;
    }
    Some(score)
}

pub(crate) fn pick_best_font_file(files: &[String]) -> Option<String> {
    let mut best: Option<(i32, &String)> = None;
    for file in files {
        let Some(score) = score_font_path(file) else {
            continue;
        };
        match best {
            None => best = Some((score, file)),
            Some((best_score, best_file)) => {
                if score > best_score || (score == best_score && file.len() < best_file.len()) {
                    best = Some((score, file));
                }
            }
        }
    }
    best.map(|(_, file)| file.clone())
}

pub(crate) fn resolve_script_font_plan(
    config: &Config,
    needs: &FontFallbackNeeds,
) -> Result<ScriptFontPlan> {
    if needs.scripts.is_empty() || !needs.needs_unicode {
        return Ok(ScriptFontPlan::default());
    }
    let state = load_notofonts_state(force_update_enabled(config))?;
    let index = build_repo_key_index(&state);
    let style = fallback_style_preference(config);
    let mut families = Vec::new();
    let mut downloads = Vec::new();
    let mut seen_repo = HashSet::new();
    let mut seen_family = HashSet::new();
    let mut seen_download = HashSet::new();

    let mut scripts = needs.scripts.iter().copied().collect::<Vec<_>>();
    scripts.sort_by_key(|script| script.full_name().to_string());
    for script in scripts {
        let Some(repo_key) = script_repo_key(script, &index) else {
            continue;
        };
        if !seen_repo.insert(repo_key.clone()) {
            continue;
        }
        let Some(repo) = state.0.get(&repo_key) else {
            continue;
        };
        let Some(family) = choose_family_name(&repo.families, style) else {
            continue;
        };
        let Some(family_info) = repo.families.get(&family) else {
            continue;
        };
        let Some(file_path) = pick_best_font_file(&family_info.files) else {
            continue;
        };
        let repo_name = if file_path.starts_with("fonts/") {
            NOTOFONTS_FILES_REPO.to_string()
        } else {
            family_info
                .latest_release
                .as_ref()
                .and_then(|release| repo_from_release_url(&release.url))
                .unwrap_or_else(|| format!("notofonts/{repo_key}"))
        };
        let raw_name = Path::new(&file_path)
            .file_name()
            .and_then(|value| value.to_str());
        let Some(raw_name) = raw_name else {
            continue;
        };
        let filename = format!("{}__{}", repo_name.replace('/', "_"), raw_name);
        let tag = if repo_name == NOTOFONTS_FILES_REPO {
            None
        } else {
            family_info
                .latest_release
                .as_ref()
                .and_then(|release| tag_from_release_url(&release.url))
        };
        if seen_family.insert(family.clone()) {
            families.push(family.clone());
        }
        let download_key = format!("{repo_name}|{file_path}");
        if seen_download.insert(download_key) {
            downloads.push(ScriptDownload {
                family,
                repo: repo_name,
                file_path,
                filename,
                tag,
            });
        }
    }

    Ok(ScriptFontPlan {
        families,
        downloads,
    })
}
