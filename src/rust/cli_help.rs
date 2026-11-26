//! Shared presentation for the Rust command trees.
use clap::{Command, CommandFactory};

pub fn header(cmd: Command) -> Command {
    cmd.help_template(concat!(
        env!("CARGO_PKG_NAME"),
        " ",
        env!("CARGO_PKG_VERSION"),
        " — {about}\n\n{usage-heading} {usage}\n\n{all-args}{after-help}"
    ))
    .mut_subcommands(header)
}

pub fn command<C: CommandFactory>(fit_engine: &str, details: &str) -> Command {
    let version = format!(
        "{}\nCompiler  {}\nTarget    {}\nFIT       {fit_engine}{details}",
        env!("CARGO_PKG_VERSION"),
        env!("FIT_FUSION_COMPILER"),
        env!("FIT_FUSION_TARGET")
    );
    header(C::command()).version(version)
}
