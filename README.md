# Texter
Texter is a crate that simplifies text modifications/updates from various sources such as an LSP and text editor in an efficient manner. It supports positions encoded for UTF-8, UTF-16 and UTF-32.

## Examples
The more simpler examples are included in this repository. For more complex projects check the end of this section.

### Text Editor (simple-text-edit)
A very simple text editor similar to `nano` that incrementally updates a `tree-sitter` `Tree` to be then used for syntax highlighting.
(TODO add images)

### LSP Server (trunkls)
An LSP server that provides completions and hover information to editors powered by `tree-sitter`'s incremental updates via `texter`.
See (TODO add link once uploaded)

## FAQ
### Why create a library for this?
While attempting to implement an LSP using `tree-sitter`, I had some trouble setting up increlemental updates in a practical way. Out of curiosity I decided to check out other LSP servers implemented in Rust.

In noticed two common patterns in their implementation. They either don't bother with incremental changes, and fully update the tree-sitter `Tree` which is innefficient, or they attempt to implement incremental changes on a per project basis. This first case is functionally good, however in the second case the problems are more subtle.

Since the incremental updates are not the main point of the project when developing an LSP server, it often ends up with lesser testing, usability, and extensibility.

While developing my own LSP server I ran into these problems myself, and decided to create this library with the goal of being well tested and performing solution.
