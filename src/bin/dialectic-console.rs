//! Console build of Dialectic, shipped on Windows as `dialectic.com` next to `dialectic.exe`.
//! Terminals resolve `dialectic` to the `.com` first, so CLI commands get a console that waits for
//! them; anything else hands off to the GUI `.exe` without blocking the terminal.

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if dialectic::cli::is_cli_invocation(&arguments) {
        std::process::exit(dialectic::cli::run(arguments));
    }
    let gui_executable = std::env::current_exe()
        .expect("cannot locate the running executable")
        .with_file_name(format!("dialectic{}", std::env::consts::EXE_SUFFIX));
    if let Err(error) = std::process::Command::new(&gui_executable).args(&arguments[1..]).spawn() {
        eprintln!("error: cannot start {}: {error}", gui_executable.display());
        std::process::exit(1);
    }
}
