use anyhow::Result;
use cfg_aliases::cfg_aliases;
use vergen::EmitBuilder;

fn main() -> Result<()> {
    cfg_aliases! {
        linux : { target_os = "linux" },
        freebsd: { target_os = "freebsd" },
        openbsd: { target_os = "openbsd" },
        netbsd: { target_os = "netbsd" },
        dragonfly: { target_os = "dragonfly" },
        unix_without_macos: { any(linux, freebsd, openbsd, netbsd, dragonfly) },
    }

    EmitBuilder::builder()
        .rustc_semver()
        .rustc_host_triple()
        .cargo_features()
        .cargo_target_triple()
        .fail_on_error()
        .emit()?;

    EmitBuilder::builder()
        .git_describe(false, false, None)
        .git_commit_date()
        .fail_on_error()
        .emit()
        .unwrap_or_else(|_| {
            println!("cargo::rustc-env=VERGEN_GIT_DESCRIBE=");
            println!("cargo::rustc-env=VERGEN_GIT_COMMIT_DATE=");
        });

    Ok(())
}
