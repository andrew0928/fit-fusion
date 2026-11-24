use clap::ValueEnum;

#[derive(Clone, Copy, ValueEnum)]
pub enum Skill {
    Index,
    Config,
    Sources,
    Selection,
    Merge,
    Results,
    Publish,
}

impl Skill {
    pub fn page(self) -> &'static str {
        match self {
            Self::Index => include_str!("../guides/index.md"),
            Self::Config => include_str!("../guides/config.md"),
            Self::Sources => include_str!("../guides/sources.md"),
            Self::Selection => include_str!("../guides/selection.md"),
            Self::Merge => include_str!("../guides/merge.md"),
            Self::Results => include_str!("../guides/results.md"),
            Self::Publish => include_str!("../guides/publish.md"),
        }
    }
}
