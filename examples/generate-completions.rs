//! Generate shell completions for `skm`. Run: `cargo run --example generate-completions`

use std::io::Error;
use std::path::Path;

use clap_complete::generate_to;
use clap_complete::shells::{Bash, Fish, Zsh};
use skill_manager::cli_command;

fn main() -> Result<(), Error> {
    let out_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("completions");
    std::fs::create_dir_all(&out_dir)?;

    let mut cmd = cli_command();
    cmd = cmd.name("skm");

    generate_to(Bash, &mut cmd, "skm", &out_dir)?;
    generate_to(Zsh, &mut cmd, "_skm", &out_dir)?;
    generate_to(Fish, &mut cmd, "skm", &out_dir)?;

    eprintln!("wrote completions to {}", out_dir.display());
    Ok(())
}
