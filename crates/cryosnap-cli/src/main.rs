mod args;
mod config;
mod interactive;
mod io;
mod parse;
mod run;
mod tmux;

#[cfg(test)]
mod test_utils;

fn main() {
    if let Err(err) = run::run() {
        eprintln!("ERROR: {err}");
        std::process::exit(1);
    }
}
