[![crates.io](https://img.shields.io/crates/v/texter)](https://crates.io/crates/texter/0.1.3)
[![docs.rs](https://img.shields.io/docsrs/texter/latest)](https://docs.rs/texter/0.1.3/texter/)
[![tests](https://github.com/airblast-dev/texter/actions/workflows/rust.yml/badge.svg)](https://github.com/airblast-dev/texter/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Texter is a crate that aims to simplify creating an LSP that uses `tree-sitter` and wants to benefit from incremental updates.

## Examples
A list of projects that use `texter`.

### LSP Server for Trunk (trunkls)
An LSP server that provides completions and hover information to editors for `trunk`'s custom HTML attributes. 
The sections where `texter` is used are fairly simple to follow. If you intend to take a look on how `texter` 
can be used in an LSP, this is likely a good starting point as it demonstrates how easy it is to integrate with 
an LSP server.

![image](https://github.com/user-attachments/assets/854b365d-3293-447a-9811-5ec5c8b9c510)

See [trunkls](https://github.com/airblast-dev/trunkls) for more information.

## Performance
`texter` is pretty fast, and not just because it enables incremental updates.
Expensive or hot functions are heavily optimized and leverage SIMD via `memchr`.
To see the exact numbers you can run the benchmarks by running `cargo bench`.

That being said, the library prioritizes ease of use, instead of performance in 
some cases. For files that are under twenty thousand lines `texter` is generally on par
with more advanced string data structures, making it suitable for majority of use cases.


## Design
The interface is designed to be easy to integrate with an LSP server. 
In case you don't use `lsp-types` or `tree-sitter`, you can implement 
`Updateable` for your own type and still benefit from using `texter`.
All of the position encodings that are supported by the LSP specification are 
also supported in `texter` (UTF-8, UTF-16, and UTF-32).

## Testing
Every function in `texter` is tested using ASCII and multibyte unicode characters.
CI is implemented via Github actions to make sure things don't break.

## FAQ

### Can I use this in an editor?
While technically possible, `texter` is not optimized for very large files 
(though it still does have some optimizations compared to calling methods on a `String`). 
The goal of `texter` is to just introduce a high level way to enable incremental updates 
for an LSP server with minimal boilerplate to the code. 

### Why create a library for this?
While attempting to implement an LSP using `tree-sitter`, I had some trouble setting up 
incremental updates in a practical way. Out of curiosity I decided to check out other 
LSP servers implemented in Rust.

I noticed two common patterns in their implementation. They either don't bother with 
incremental changes, and fully update the tree-sitter `Tree` which is innefficient, 
or they attempt to implement incremental changes on a per project basis. 
This first case is functionally good, however in the second case the problems are more subtle.

Since the incremental updates are not the main point of the project when developing 
an LSP server, it often ends up with lesser testing, usability, and extensibility.

While developing my own LSP server I ran into these problems myself, and decided to 
create this library with the goal of being a well tested and high performance solution.
