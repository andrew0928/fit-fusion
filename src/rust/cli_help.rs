//! Shared presentation for the two Rust command trees.
use clap::{Command, CommandFactory};

pub fn command<C: CommandFactory>(details: &str) -> Command {
    fn header(cmd: Command) -> Command {
        cmd.help_template(concat!(
            env!("CARGO_PKG_NAME"),
            " ",
            env!("CARGO_PKG_VERSION"),
            " — {about}\n\n{usage-heading} {usage}\n\n{all-args}{after-help}"
        ))
        .mut_subcommands(header)
    }
    let version = format!(
        "{}\nCompiler  {}\nTarget    {}\nFIT       RustyFIT and wire reader{details}",
        env!("CARGO_PKG_VERSION"),
        env!("FIT_FUSION_COMPILER"),
        env!("FIT_FUSION_TARGET")
    );
    header(C::command()).version(version)
}
