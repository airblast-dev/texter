# Texter
Texter is a crate that aims simplify creating an LSP that uses `tree-sitter` and wants to benefit from incremental updates.

## Examples
A list of projects that use `texter`.

### LSP Server for Trunk (trunkls)
An LSP server that provides completions and hover information to editors for `trunk`'s custom HTML attributes. 
The sections where `texter` is used are fairly simple to follow. If you intend to take a look on how `texter` 
can be used in an LSP, this is likely a good starting point.
See [trunkls](https://github.com/airblast-dev/trunkls) for more information.

## FAQ

### Can I use this in an editor?
While technically possible, `texter` is not optimized for very large files (though it still does have some optimizations compared to calling methods on a `String`). 
The goal of `texter` is to just introduce a high level way to enable incremental updates for an LSP server with minimal boilerplate to the code. 

### Why create a library for this?
While attempting to implement an LSP using `tree-sitter`, I had some trouble setting up increlemental updates in a practical way. Out of curiosity I decided to check out other LSP servers implemented in Rust.

I noticed two common patterns in their implementation. They either don't bother with incremental changes, and fully update the tree-sitter `Tree` which is innefficient, or they attempt to implement incremental changes on a per project basis. This first case is functionally good, however in the second case the problems are more subtle.

Since the incremental updates are not the main point of the project when developing an LSP server, it often ends up with lesser testing, usability, and extensibility.

While developing my own LSP server I ran into these problems myself, and decided to create this library with the goal of being a well tested and high performance solution.
