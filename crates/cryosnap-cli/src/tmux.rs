use std::error::Error;

pub(crate) fn capture_tmux_output(raw_args: Option<&str>) -> Result<String, Box<dyn Error>> {
    let user_args = normalize_tmux_args(raw_args)?;
    let cmd_args = build_tmux_capture_args(&user_args);
    let output = std::process::Command::new("tmux")
        .args(cmd_args)
        .output()
        .map_err(|err| format!("failed to run tmux: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tmux capture failed: {}", stderr.trim()).into());
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.is_empty() {
        return Err("tmux returned empty output".into());
    }
    Ok(text)
}

pub(crate) fn tmux_title(raw_args: Option<&str>, format: &str) -> Option<String> {
    let format = format.trim();
    if format.is_empty() {
        return None;
    }
    let user_args = normalize_tmux_args(raw_args).ok()?;
    let target = extract_tmux_target(&user_args);
    let mut cmd = std::process::Command::new("tmux");
    cmd.arg("display-message").arg("-p");
    if let Some(target) = target {
        cmd.arg("-t").arg(target);
    }
    cmd.arg(format);
    let output = cmd.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

pub(crate) fn extract_tmux_target(args: &[String]) -> Option<String> {
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "-t" {
            return iter.next().cloned();
        }
        if arg.starts_with("-t") {
            return Some(arg.trim_start_matches("-t").to_string());
        }
    }
    None
}

pub(crate) fn normalize_tmux_args(raw: Option<&str>) -> Result<Vec<String>, Box<dyn Error>> {
    match raw {
        Some(value) => {
            let args =
                shell_words::split(value).map_err(|err| format!("tmux args parse: {err}"))?;
            Ok(args)
        }
        None => Ok(Vec::new()),
    }
}

pub(crate) fn build_tmux_capture_args(user_args: &[String]) -> Vec<String> {
    let has_p = user_args.iter().any(|arg| arg == "-p");
    let has_e = user_args.iter().any(|arg| arg == "-e");
    let mut args = Vec::new();
    args.push("capture-pane".to_string());
    if !has_p {
        args.push("-p".to_string());
    }
    if !has_e {
        args.push("-e".to_string());
    }
    args.extend(user_args.iter().cloned());
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::env_lock;

    #[test]
    fn build_tmux_capture_args_defaults() {
        let args = build_tmux_capture_args(&[]);
        assert!(args.contains(&"capture-pane".to_string()));
        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"-e".to_string()));
    }

    #[test]
    fn build_tmux_capture_args_preserves_flags() {
        let args = build_tmux_capture_args(&[
            "-p".to_string(),
            "-e".to_string(),
            "-t".to_string(),
            "%3".to_string(),
        ]);
        assert_eq!(
            args,
            vec!["capture-pane", "-p", "-e", "-t", "%3",]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn normalize_tmux_args_accepts_raw_string() {
        let args = normalize_tmux_args(Some("-t %3 -S -200 -J")).expect("parse");
        assert_eq!(
            args,
            vec!["-t", "%3", "-S", "-200", "-J"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn extract_tmux_target_from_separate_flag() {
        let args = vec![
            "-t".to_string(),
            "%7".to_string(),
            "-S".to_string(),
            "-200".to_string(),
        ];
        assert_eq!(extract_tmux_target(&args), Some("%7".to_string()));
    }

    #[test]
    fn extract_tmux_target_from_compact_flag() {
        let args = vec!["-t%9".to_string(), "-S".to_string(), "-200".to_string()];
        assert_eq!(extract_tmux_target(&args), Some("%9".to_string()));
    }

    #[test]
    fn tmux_title_empty_format_returns_none() {
        assert!(tmux_title(None, " ").is_none());
    }

    #[test]
    fn tmux_title_missing_tmux_returns_none() {
        let _lock = env_lock().lock().expect("lock");
        let prev_path = std::env::var("PATH").ok();
        std::env::set_var("PATH", "");
        assert!(tmux_title(None, "#{pane_title}").is_none());
        if let Some(path) = prev_path {
            std::env::set_var("PATH", path);
        } else {
            std::env::remove_var("PATH");
        }
    }

    #[test]
    fn capture_tmux_output_missing_tmux_errors() {
        let _lock = env_lock().lock().expect("lock");
        let prev_path = std::env::var("PATH").ok();
        std::env::set_var("PATH", "");
        let err = capture_tmux_output(None).unwrap_err();
        assert!(err.to_string().contains("failed to run tmux"));
        if let Some(path) = prev_path {
            std::env::set_var("PATH", path);
        } else {
            std::env::remove_var("PATH");
        }
    }
}
