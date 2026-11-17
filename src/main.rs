//! Dialectic: local Git pull requests. Scaffolded from Rune's `examples/hello`.
//! `dialectic [path]` opens the desktop app; `dialectic pr …` and `dialectic guide` are the terminal CLI.

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if dialectic::cli::is_cli_invocation(&arguments) {
        std::process::exit(dialectic::cli::run(arguments));
    }
    dialectic::ui::run();
}
