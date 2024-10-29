pub mod br_indexes;
mod encodings;
mod text;

use memchr::memmem::Finder;
use std::sync::LazyLock;

/// A line searcher that does not care if you use an old apple device.
static BR_FINDER: LazyLock<Finder> = LazyLock::new(|| Finder::new("\n"));
// TODO: the lsp protocol says that ["\r", "\n", "\r\n"] are valid End of lines.
// we should eventually do proper support for all 3 cases. Assuming all text sent
// abides by the protocol things should still be fine.
