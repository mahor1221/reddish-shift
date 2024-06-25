#![allow(clippy::unwrap_used)]
use _reddish_shift::cli_args_command;
use anyhow::Result;
use clap_complete::{generate_to, Shell};
use std::{env, path::PathBuf};

fn main() -> Result<()> {
    const NAME: &str = env!("CARGO_PKG_NAME");
    let out = env::var_os("OUT_DIR").unwrap();
    let target = PathBuf::from(&out).ancestors().nth(3).unwrap().to_owned();
    let mut cmd = cli_args_command();

    for shell in [Shell::Bash, Shell::Fish, Shell::Zsh, Shell::Elvish] {
        generate_to(shell, &mut cmd, NAME, &out)?;
    }

    for file in std::fs::read_dir(&out)? {
        let f = file?.path();
        std::fs::rename(&f, target.join(f.file_name().unwrap()))?;
    }

    Ok(())
}
