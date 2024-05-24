use anyhow::Result;
use vergen::EmitBuilder;

fn main() -> Result<()> {
    EmitBuilder::builder()
        .fail_on_error()
        .git_describe(false, false, None)
        .git_commit_date()
        .rustc_semver()
        .rustc_host_triple()
        .cargo_features()
        .cargo_target_triple()
        .emit()?;

    Ok(())
}
