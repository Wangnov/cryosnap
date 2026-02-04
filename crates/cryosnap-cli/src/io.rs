use crate::args::FormatArg;
use std::error::Error;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

pub(crate) fn write_output_with_tty(
    result: cryosnap_core::RenderResult,
    output: Option<&PathBuf>,
    _input: Option<&str>,
    format: Option<FormatArg>,
    stdout_is_tty: bool,
) -> Result<(), Box<dyn Error>> {
    if let Some(path) = output {
        std::fs::write(path, result.bytes)?;
        if stdout_is_tty {
            print_wrote(path);
        }
        return Ok(());
    }

    if stdout_is_tty {
        let default_name = match format {
            Some(FormatArg::Png) => "cryosnap.png",
            Some(FormatArg::Webp) => "cryosnap.webp",
            _ => "cryosnap.svg",
        };
        let output_name = default_name.to_string();
        std::fs::write(&output_name, result.bytes)?;
        print_wrote(Path::new(&output_name));
        return Ok(());
    }

    let mut stdout = io::stdout();
    stdout.write_all(&result.bytes)?;
    Ok(())
}

pub(crate) fn print_wrote(path: &Path) {
    println!("WROTE {}", path.display());
}

pub(crate) fn read_stdin() -> Result<String, io::Error> {
    let mut buffer = String::new();
    io::stdin().read_to_string(&mut buffer)?;
    Ok(buffer)
}

pub(crate) fn read_stdin_with(stdin_override: Option<&str>) -> Result<String, io::Error> {
    if let Some(value) = stdin_override {
        return Ok(value.to_string());
    }
    read_stdin()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::FormatArg;
    use crate::test_utils::cwd_lock;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn write_output_to_file() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("out.svg");
        let result = cryosnap_core::RenderResult {
            format: cryosnap_core::OutputFormat::Svg,
            bytes: b"test".to_vec(),
        };
        write_output_with_tty(result, Some(&path), None, None, false).expect("write");
        let contents = fs::read_to_string(&path).expect("read");
        assert_eq!(contents, "test");
    }

    #[test]
    fn write_output_to_file_prints_when_tty() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("out.svg");
        let result = cryosnap_core::RenderResult {
            format: cryosnap_core::OutputFormat::Svg,
            bytes: b"test".to_vec(),
        };
        write_output_with_tty(result, Some(&path), None, None, true).expect("write");
    }

    #[test]
    fn write_output_default_name() {
        let _lock = cwd_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        let cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("chdir");
        let result = cryosnap_core::RenderResult {
            format: cryosnap_core::OutputFormat::Svg,
            bytes: b"test".to_vec(),
        };
        write_output_with_tty(result, None, Some("file.rs"), Some(FormatArg::Png), true)
            .expect("write");
        assert!(dir.path().join("cryosnap.png").exists());
        std::env::set_current_dir(cwd).expect("restore");
    }

    #[test]
    fn write_output_default_name_webp() {
        let _lock = cwd_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        let cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("chdir");
        let result = cryosnap_core::RenderResult {
            format: cryosnap_core::OutputFormat::Webp,
            bytes: b"test".to_vec(),
        };
        write_output_with_tty(result, None, Some("file.rs"), Some(FormatArg::Webp), true)
            .expect("write");
        assert!(dir.path().join("cryosnap.webp").exists());
        std::env::set_current_dir(cwd).expect("restore");
    }

    #[test]
    fn write_output_default_name_svg() {
        let _lock = cwd_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        let cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("chdir");
        let result = cryosnap_core::RenderResult {
            format: cryosnap_core::OutputFormat::Svg,
            bytes: b"test".to_vec(),
        };
        write_output_with_tty(result, None, None, None, true).expect("write");
        assert!(dir.path().join("cryosnap.svg").exists());
        std::env::set_current_dir(cwd).expect("restore");
    }

    #[test]
    fn write_output_stdin_default_name() {
        let _lock = cwd_lock().lock().expect("lock");
        let dir = tempdir().expect("temp dir");
        let cwd = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir.path()).expect("chdir");
        let result = cryosnap_core::RenderResult {
            format: cryosnap_core::OutputFormat::Svg,
            bytes: b"test".to_vec(),
        };
        write_output_with_tty(result, None, Some("-"), Some(FormatArg::Png), true).expect("write");
        assert!(dir.path().join("cryosnap.png").exists());
        std::env::set_current_dir(cwd).expect("restore");
    }

    #[test]
    fn write_output_stdout_branch() {
        let result = cryosnap_core::RenderResult {
            format: cryosnap_core::OutputFormat::Svg,
            bytes: b"test".to_vec(),
        };
        write_output_with_tty(result, None, None, None, false).expect("write");
    }

    #[test]
    fn read_stdin_with_override() {
        let result = read_stdin_with(Some("hello")).expect("read");
        assert_eq!(result, "hello");
    }
}
