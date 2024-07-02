#![allow(clippy::unwrap_used)]
use anyhow::Result;
use clap::ValueEnum;
use clap_complete::{generate_to, Shell};
use clap_mangen::Man;
use reddish_shift::cli_args_command;
use std::{env, fs, path::PathBuf};

fn main() -> Result<()> {
    const NAME: &str = "reddish-shift";
    let out = env::var_os("OUT_DIR").unwrap();
    let target = PathBuf::from(&out).ancestors().nth(3).unwrap().to_owned();
    let mut cmd = cli_args_command();

    // generate auto completion scripts
    for &shell in Shell::value_variants() {
        generate_to(shell, &mut cmd, NAME, &out)?;
    }

    for file in fs::read_dir(&out)? {
        let f = file?.path();
        fs::rename(&f, target.join(f.file_name().unwrap()))?;
    }

    // generate man pages
    let mut buffer: Vec<u8> = Default::default();
    for subcmd in cmd.get_subcommands() {
        let subcmd_name = format!("{NAME}-{}", subcmd.get_name());
        Man::new(subcmd.clone().name(&subcmd_name)).render(&mut buffer)?;
        std::fs::write(target.join(format!("{subcmd_name}.1")), &buffer)?;
        buffer.clear();
    }
    Man::new(cmd).render(&mut buffer)?;
    fs::write(target.join(format!("{NAME}.1")), buffer)?;

    Ok(())
}
