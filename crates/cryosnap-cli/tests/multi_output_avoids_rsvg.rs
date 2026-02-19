#[cfg(unix)]
#[test]
fn output_pattern_png_webp_ignores_rsvg_convert() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Stdio};

    use tempfile::tempdir;

    let cryosnap = env!("CARGO_BIN_EXE_cryosnap");

    let bin_dir = tempdir().expect("temp bin dir");
    let rsvg_convert = bin_dir.path().join("rsvg-convert");
    std::fs::write(&rsvg_convert, "#!/bin/sh\nprintf 'notpng'\n").expect("write rsvg-convert");
    let mut perms = std::fs::metadata(&rsvg_convert)
        .expect("metadata")
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&rsvg_convert, perms).expect("chmod");

    let out_dir = tempdir().expect("temp out dir");
    let out_pattern = out_dir.path().join("out.{png,webp}");

    let path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", bin_dir.path().to_string_lossy(), path);

    let mut child = Command::new(cryosnap)
        .arg("-o")
        .arg(&out_pattern)
        .env("PATH", new_path)
        .env("CRYOSNAP_FONT_AUTO_DOWNLOAD", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn cryosnap");

    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(b"hello")
        .expect("write stdin");

    let output = child.wait_with_output().expect("wait");
    assert!(
        output.status.success(),
        "cryosnap failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out_dir.path().join("out.png").exists());
    assert!(out_dir.path().join("out.webp").exists());
}
