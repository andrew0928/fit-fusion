use clap::ValueEnum;

#[derive(Clone, Copy, ValueEnum)]
pub enum Skill {
    Index,
    Inspect,
    Raw,
    Roundtrip,
}

macro_rules! page {
    ($file:literal) => {
        include_str!(concat!("../../../python/fit_fusion/guides/", $file))
    };
}

impl Skill {
    pub fn page(self) -> String {
        let text = match self {
            Self::Index => page!("index.md"),
            Self::Inspect => page!("inspect.md"),
            Self::Raw => page!("raw.md"),
            Self::Roundtrip => page!("roundtrip.md"),
        };
        text.replace("{cli}", "fit-inspector")
            .replace(
                "{launch_note}",
                "Run the Rust fit-inspector binary on PATH.",
            )
            .replace(
                "{decoder_note}",
                "The default decoder is RustyFIT. Use --decoder wire for a wire-only inventory.",
            )
            .replace("{sdk_topic}", "")
    }
}
