#![cfg_attr(docsrs, feature(doc_cfg))]

//! # Texter
//!
//! Texter is a crate aiming to simplify writing an LSP using `tree-sitter` whilst benefiting from
//! incremental updates. The main type that you will interacte with is [`Text`][`core::text::Text`].
//!
//! ## Usage with lsp-types
//!
//! [`change::Change`] implements [`From<lsp_types::TextDocumentContentChangeEvent>`] so in
//! most cases calling [`Into::into`] and providing it to [`Text::update`][`core::text::Text::update`] is enough to keep content
//! in sync.
//!
//! ### Selecting an encoding
//!
//! Positions provided from a client may be for different encodings. UTF-8, UTF-16, or
//! UTF-32. When starting up an LSP, the client provides encoding it will use. With that
//! information we can store a function pointer and create new [`Text`][`core::text::Text::update`]'s as needed.
//!
//! The example below works the same way to how it is done in `rust-analyzer`.
//! ```
//! # fn get_client_encoding() -> Option<Vec<PositionEncodingKind>> {None}
//! use texter::core::text::Text;
//! use texter::lsp_types::PositionEncodingKind;
//!
//! fn decide_encoding() -> fn(String) -> Text {
//!     // The type provided in client capabilities.
//!     let encodings: Option<Vec<PositionEncodingKind>> = get_client_encoding();
//!     let Some(encodings) = encodings else {
//!         return Text::new_utf16;
//!     };
//!
//!     // Hope that we can use anything other than UTF-16
//!     for encoding in encodings {
//!         if encoding == PositionEncodingKind::UTF8 {
//!             return Text::new;
//!         } else if encoding == PositionEncodingKind::UTF32 {
//!             return Text::new_utf32;
//!         }
//!     }
//!
//!     // Too bad, UTF-16 it is.
//!     Text::new_utf16
//! }
//! ```
//!
//! ### How to write an LSP using the crate?
//!
//! There is multiple ways to structure your server, `texter` aims to influence the structure as little as possible.
//! For an example you can check out [trunkls](https://github.com/airblast-dev/trunkls).
//!
//! ## Usage with tree-sitter
//!
//! When using a [`Text`][`core::text::Text`] with incremental updates we want to keep using a single
//! [`tree_sitter::Tree`] across edits. To simplify the process [`Updateable`][`updateables::Updateable`]
//! is implemented on [`tree_sitter::Tree`]. So simply providing a mutable reference of the tree to
//! a [`Text`][`core::text::Text`]'s update method is enough to keep the data in sync.
//!
//! In case you want to update a [`tree_sitter::Node`], [`Updateable`][`updateables::Updateable`] is implemented
//! for it as  well.

mod utils;

pub mod change;
pub mod core;
pub mod error;
pub mod querier;
pub mod updateables;

#[cfg_attr(docsrs, doc(cfg(feature = "lsp-types")))]
#[cfg(feature = "lsp-types")]
pub use lsp_types;

#[cfg_attr(docsrs, doc(cfg(feature = "tree-sitter")))]
#[cfg(feature = "tree-sitter")]
pub use tree_sitter;
