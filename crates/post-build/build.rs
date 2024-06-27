#![allow(clippy::unwrap_used)]
use anyhow::Result;
use clap::ValueEnum;
use clap_complete::{generate_to, Shell};
use reddish_shift::cli_args_command;
use std::{env, path::PathBuf};

fn main() -> Result<()> {
    // generate auto completion scripts
    const NAME: &str = "reddish-shift";
    let out = env::var_os("OUT_DIR").unwrap();
    let target = PathBuf::from(&out).ancestors().nth(3).unwrap().to_owned();
    let mut cmd = cli_args_command();

    for &shell in Shell::value_variants() {
        generate_to(shell, &mut cmd, NAME, &out)?;
    }

    for file in std::fs::read_dir(&out)? {
        let f = file?.path();
        std::fs::rename(&f, target.join(f.file_name().unwrap()))?;
    }

    Ok(())
}
