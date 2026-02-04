use crate::{Config, Error, InputSource, Result};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct LoadedInput {
    pub(crate) text: String,
    pub(crate) path: Option<PathBuf>,
    pub(crate) kind: InputKind,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum InputKind {
    Code,
    Ansi,
}

pub(crate) fn load_input(input: &InputSource, timeout: Duration) -> Result<LoadedInput> {
    match input {
        InputSource::Text(text) => Ok(LoadedInput {
            text: text.clone(),
            path: None,
            kind: InputKind::Code,
        }),
        InputSource::File(path) => {
            let text = std::fs::read_to_string(path)?;
            Ok(LoadedInput {
                text,
                path: Some(path.clone()),
                kind: InputKind::Code,
            })
        }
        InputSource::Command(cmd) => {
            let text = execute_command(cmd, timeout)?;
            Ok(LoadedInput {
                text,
                path: None,
                kind: InputKind::Ansi,
            })
        }
    }
}

pub(crate) fn is_ansi_input(loaded: &LoadedInput, config: &Config) -> bool {
    if let Some(lang) = &config.language {
        if lang.eq_ignore_ascii_case("ansi") {
            return true;
        }
    }
    if matches!(loaded.kind, InputKind::Ansi) {
        return true;
    }
    loaded.text.contains('\u{1b}')
}

pub(crate) fn execute_command(cmd: &str, timeout: Duration) -> Result<String> {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};
    use std::io::Read;
    use std::sync::mpsc;
    use std::thread;

    let args = shell_words::split(cmd)
        .map_err(|err| Error::InvalidInput(format!("command parse: {err}")))?;
    if args.is_empty() {
        return Err(Error::InvalidInput("empty command".to_string()));
    }

    let (cols, rows) = terminal_size::terminal_size()
        .map(|(w, h)| (w.0, h.0))
        .unwrap_or((80, 24));

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|err| Error::Render(format!("open pty: {err}")))?;

    let mut command = CommandBuilder::new(&args[0]);
    if args.len() > 1 {
        command.args(&args[1..]);
    }

    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|err| Error::Render(format!("spawn command: {err}")))?;
    drop(pair.slave);
    let mut killer = child.clone_killer();

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|err| Error::Render(format!("pty reader: {err}")))?;
    drop(pair.master);

    let read_handle = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = reader.read_to_end(&mut buf);
        buf
    });

    let (status_tx, status_rx) = mpsc::channel();
    thread::spawn(move || {
        let status = child.wait();
        let _ = status_tx.send(status);
    });

    let status = match status_rx.recv_timeout(timeout) {
        Ok(status) => status,
        Err(_) => {
            let _ = killer.kill();
            return Err(Error::Timeout);
        }
    };
    let output = read_handle.join().unwrap_or_default();
    let output_str = String::from_utf8_lossy(&output).to_string();

    match status {
        Ok(exit) => {
            if !exit.success() {
                return Err(Error::Render(format!("command exited with {exit}")));
            }
        }
        Err(err) => return Err(Error::Render(format!("command wait: {err}"))),
    }

    if output_str.is_empty() {
        return Err(Error::InvalidInput("no command output".to_string()));
    }

    Ok(output_str)
}
