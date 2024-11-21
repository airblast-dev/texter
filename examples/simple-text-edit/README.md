The goal of the example is to implement a very simple editor that provides syntax highlighting and uses incremental parsing with the help of `texter`.

For a more fully featured editor that makes use of `texter` see (actually have to make an editor first)
(TODO add images)

## Running the Example
Running `cargo run --example simple-text-edit --features tree-sitter` will start the demo.

## Notes:
To keep the example simple, many common editor features are not implemented.
The main point is to display how little you have to interact with `texter` when benefitting from the 
performance benefits of incremental parsing and optimized `String` operations (see documentation for more information on the optimized `String` operations).
