//! Dialectic: local Git pull requests. Scaffolded from Rune's `examples/hello`.
//! `dialectic [path]` opens the desktop app; `dialectic pr …` and `dialectic guide` are the terminal CLI.

// Release builds are GUI apps on Windows so no console window opens behind the app.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if dialectic::cli::is_cli_invocation(&arguments) {
        attach_to_parent_console();
        std::process::exit(dialectic::cli::run(arguments));
    }
    dialectic::ui::run();
}

/// A GUI-subsystem exe starts without a console; borrow the launching terminal's so CLI output shows up.
#[cfg(all(windows, not(debug_assertions)))]
fn attach_to_parent_console() {
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    // Fails when there is no parent console (e.g. launched from Explorer); output is then dropped, as intended.
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS) };
}

#[cfg(not(all(windows, not(debug_assertions))))]
fn attach_to_parent_console() {}
