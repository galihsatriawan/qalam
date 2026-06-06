use clap::CommandFactory;
use clap_complete::{generate, Shell};
use std::io;

pub fn run(shell: Shell) {
    let mut cmd = crate::cli::build_cli();
    generate(shell, &mut cmd, "qalam", &mut io::stdout());
}
