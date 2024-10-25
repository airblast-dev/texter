use memchr::memmem::Finder;
use std::sync::LazyLock;

pub mod br_indexes;
mod text;

static BR_FINDER: LazyLock<Finder> = LazyLock::new(|| Finder::new("\n"));
