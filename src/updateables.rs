use tracing::instrument;

use crate::{change::GridIndex, core::br_indexes::BrIndexes};

#[derive(Clone, Debug)]
pub enum ChangeContext<'a> {
    Insert {
        position: GridIndex,
        text: &'a str,
        inserted_br_indexes: &'a [usize],
    },
    Delete {
        start: GridIndex,
        end: GridIndex,
    },
    Replace {
        start: GridIndex,
        end: GridIndex,
        text: &'a str,
        inserted_br_indexes: &'a [usize],
    },
    ReplaceFull {
        text: &'a str,
    },
}

#[derive(Clone, Debug)]
pub struct UpdateContext<'a> {
    /// A context change that is being used to update the [`Text`].
    pub change: ChangeContext<'a>,
    /// The new breakline positions.
    pub breaklines: &'a BrIndexes,
    /// The old breakline positions.
    pub old_breaklines: &'a BrIndexes,
    /// The old string.
    pub old_str: &'a str,
}

pub trait Updateable {
    fn update(&mut self, ctx: UpdateContext);
}

impl Updateable for () {
    fn update(&mut self, _: UpdateContext) {}
}

impl<'a, T> Updateable for T
where
    T: 'a + FnMut(UpdateContext),
{
    #[instrument(skip(self))]
    fn update(&mut self, ctx: UpdateContext) {
        self(ctx)
    }
}

#[cfg(feature = "tree-sitter")]
mod ts {
    use tracing::{info, instrument};
    use tree_sitter::{InputEdit, Point, Tree};

    use super::{ChangeContext, UpdateContext, Updateable};

    impl Updateable for Tree {
        fn update(&mut self, ctx: UpdateContext) {
            self.edit(&edit_from_ctx(ctx));
        }
    }

    pub(super) fn edit_from_ctx(ctx: UpdateContext) -> InputEdit {
        let old_br = ctx.old_breaklines;
        let new_br = ctx.breaklines;
        let ie = match ctx.change {
            ChangeContext::Delete { start, end } => {
                let start_byte = old_br.row_start(start.row) + start.col;
                let end_byte = old_br.row_start(end.row) + end.col;

                InputEdit {
                    start_position: start.into(),
                    old_end_position: end.into(),
                    new_end_position: start.into(),
                    start_byte,
                    old_end_byte: end_byte,
                    new_end_byte: start_byte,
                }
            }
            ChangeContext::Insert {
                inserted_br_indexes,
                position,
                text,
            } => {
                let start_byte = old_br.row_start(position.row) + position.col;
                let new_end_byte = start_byte + text.len();
                InputEdit {
                    start_byte,
                    old_end_byte: start_byte,
                    new_end_byte,
                    start_position: position.into(),
                    old_end_position: position.into(),
                    new_end_position: Point {
                        row: position.row + inserted_br_indexes.len(),
                        // -1 because bri includes the breakline
                        column: inserted_br_indexes
                            .last()
                            .map(|bri| text.len() - (bri - start_byte) - 1)
                            .unwrap_or(text.len() + position.col),
                    },
                }
            }
            ChangeContext::Replace {
                start,
                end,
                text,
                inserted_br_indexes,
            } => {
                let start_byte = old_br.row_start(start.row) + start.col;
                let old_end_byte = old_br.row_start(end.row) + end.col;
                InputEdit {
                    start_byte,
                    start_position: start.into(),
                    old_end_position: end.into(),
                    old_end_byte,
                    new_end_byte: start_byte + text.len(),
                    new_end_position: {
                        if let [.., last] = inserted_br_indexes {
                            Point {
                                row: start.row + inserted_br_indexes.len(),
                                // -1 because last includes the breakline
                                column: text.len() - (last - start_byte) - 1,
                            }
                        } else {
                            Point {
                                row: start.row,
                                column: start.col + text.len(),
                            }
                        }
                    },
                }
            }
            // TODO: probably broken
            ChangeContext::ReplaceFull { text } => InputEdit {
                start_byte: 0,
                old_end_byte: ctx.old_str.len(),
                new_end_byte: text.len(),
                start_position: Point { row: 0, column: 0 },
                old_end_position: Point {
                    row: old_br.row_count() - 1,
                    column: ctx.old_str.len() - old_br.last_row(),
                },
                new_end_position: Point {
                    row: new_br.row_count() - 1,
                    column: text.len() - new_br.last_row(),
                },
            },
        };
        info!("{:?}", ie);
        ie
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "tree-sitter")]
    mod ts {
        use tree_sitter::{InputEdit, Point};

        use crate::{
            change::GridIndex,
            core::br_indexes::BrIndexes,
            updateables::{ts::edit_from_ctx, ChangeContext, UpdateContext},
        };

        #[test]
        fn edit_ctx_delete_across_lines() {
            // old_str: "HelJuice";
            let edit = edit_from_ctx(UpdateContext {
                breaklines: &BrIndexes(vec![0]),
                old_breaklines: &BrIndexes(vec![0, 12, 16, 20]),
                old_str: "Hello World!\n123\nasd\nAppleJuice",
                change: ChangeContext::Delete {
                    start: GridIndex { row: 0, col: 3 },
                    end: GridIndex { row: 3, col: 5 },
                },
            });

            let correct_edit = InputEdit {
                start_byte: 3,
                start_position: Point { row: 0, column: 3 },
                old_end_byte: 26,
                old_end_position: Point { row: 3, column: 5 },
                new_end_byte: 3,
                new_end_position: Point { row: 0, column: 3 },
            };

            assert_eq!(edit, correct_edit);
        }

        #[test]
        fn edit_ctx_delete_in_line_first_row() {
            // let old = "Hello World!\nd\nAppleJuice";
            let edit = edit_from_ctx(UpdateContext {
                breaklines: &BrIndexes(vec![0, 8, 12, 20]),
                old_breaklines: &BrIndexes(vec![0, 12, 16, 20]),
                old_str: "Hello World!\n123\nasd\nAppleJuice",
                change: ChangeContext::Delete {
                    start: GridIndex { row: 0, col: 3 },
                    end: GridIndex { row: 0, col: 7 },
                },
            });

            let correct_edit = InputEdit {
                start_byte: 3,
                start_position: Point { row: 0, column: 3 },
                old_end_byte: 7,
                old_end_position: Point { row: 0, column: 7 },
                new_end_byte: 3,
                new_end_position: Point { row: 0, column: 3 },
            };

            assert_eq!(edit, correct_edit);
        }

        #[test]
        fn edit_ctx_delete_in_line_last_row() {
            // let old = "Hello World!\nd\nAppleJuice";
            let edit = edit_from_ctx(UpdateContext {
                breaklines: &BrIndexes(vec![0, 12, 16, 20]),
                old_breaklines: &BrIndexes(vec![0, 12, 16, 20]),
                old_str: "Hello World!\n123\nasd\nAppleJuice",
                change: ChangeContext::Delete {
                    start: GridIndex { row: 3, col: 3 },
                    end: GridIndex { row: 3, col: 7 },
                },
            });

            let correct_edit = InputEdit {
                start_byte: 24,
                start_position: Point { row: 3, column: 3 },
                old_end_byte: 28,
                old_end_position: Point { row: 3, column: 7 },
                new_end_byte: 24,
                new_end_position: Point { row: 3, column: 3 },
            };

            assert_eq!(edit, correct_edit);
        }

        #[test]
        fn edit_ctx_insert() {
            let edit = edit_from_ctx(UpdateContext {
                breaklines: &BrIndexes(vec![0, 12, 16, 20]),
                old_breaklines: &BrIndexes(vec![0, 12, 14]),
                old_str: "Hello World!\nd\nAppleJuice",
                change: ChangeContext::Insert {
                    inserted_br_indexes: &[16],
                    position: GridIndex { row: 1, col: 0 },
                    text: "123\nas",
                },
            });

            let correct_edit = InputEdit {
                start_byte: 13,
                start_position: Point { row: 1, column: 0 },
                old_end_byte: 13,
                old_end_position: Point { row: 1, column: 0 },
                new_end_byte: 19,
                new_end_position: Point { row: 2, column: 2 },
            };

            assert_eq!(edit, correct_edit);
        }

        #[test]
        fn edit_ctx_replace_shrink() {
            // old = "HelloWelcomedhasgdjh\nAppleJuice";
            let edit = edit_from_ctx(UpdateContext {
                breaklines: &BrIndexes(vec![0, 20]),
                old_breaklines: &BrIndexes(vec![0, 12, 31]),
                old_str: "Hello World!\ndgsadhasgjdhasgdjh\nAppleJuice",
                change: ChangeContext::Replace {
                    start: GridIndex { row: 0, col: 5 },
                    end: GridIndex { row: 1, col: 10 },
                    text: "Welcome",
                    inserted_br_indexes: &[],
                },
            });

            let correct_edit = InputEdit {
                start_byte: 5,
                start_position: Point { row: 0, column: 5 },
                old_end_byte: 23,
                old_end_position: Point { row: 1, column: 10 },
                new_end_byte: 12,
                new_end_position: Point { row: 0, column: 12 },
            };

            assert_eq!(edit, correct_edit);
        }

        #[test]
        fn edit_ctx_replace_grow() {
            //let result = "HelloWelcome\narld!\ndgsadhasgjdhasgdjh\nAppleJuice";
            let edit = edit_from_ctx(UpdateContext {
                breaklines: &BrIndexes(vec![0, 12, 18, 39]),
                old_breaklines: &BrIndexes(vec![0, 12, 21]),
                old_str: "Hello World!\ndgsadhasgjdhasgdjh\nAppleJuice",
                change: ChangeContext::Replace {
                    start: GridIndex { row: 0, col: 5 },
                    end: GridIndex { row: 0, col: 8 },
                    text: "Welcome\na",
                    inserted_br_indexes: &[12],
                },
            });

            let correct_edit = InputEdit {
                start_byte: 5,
                start_position: Point { row: 0, column: 5 },
                old_end_byte: 8,
                old_end_position: Point { row: 0, column: 8 },
                new_end_byte: 14,
                new_end_position: Point { row: 1, column: 1 },
            };

            assert_eq!(edit, correct_edit);
        }

        #[test]
        fn edit_ctx_replace_full() {
            //let result = "HelloWelcome\narld!\ndgsadhasgjdhasgdjh\nAppleJuice";
            let edit = edit_from_ctx(UpdateContext {
                breaklines: &BrIndexes(vec![0, 10, 19, 20, 21, 39]),
                old_breaklines: &BrIndexes(vec![0, 12, 31]),
                old_str: "Hello World!\ndgsadhasgjdhasgdjh\nAppleJuice",
                change: ChangeContext::ReplaceFull {
                    text: "sdghfkjhsd\nasdasdas\n\n\nasdasdasdasdasdas\nasdasd",
                },
            });

            let correct_edit = InputEdit {
                start_byte: 0,
                start_position: Point { row: 0, column: 0 },
                old_end_byte: 42,
                old_end_position: Point { row: 2, column: 10 },
                new_end_byte: 46,
                new_end_position: Point { row: 5, column: 6 },
            };

            assert_eq!(edit, correct_edit);
        }
    }

    #[cfg(feature = "tree-sitter")]
    mod tree_sitter {
        use rstest::{fixture, rstest};
        use tree_sitter::{InputEdit, Parser, Point, Tree};

        use crate::{
            change::{Change, GridIndex},
            core::{br_indexes, text::Text},
        };

        const SAMPLE_HTML: &str = include_str!("sample.html");
        const ATTRIBUTE_NAME_POS: Point = Point { row: 8, column: 57 };

        #[fixture]
        fn parser() -> Parser {
            let mut p = Parser::new();
            p.set_language(&tree_sitter_html::LANGUAGE.into()).unwrap();
            p
        }

        #[fixture]
        fn html_tree(mut parser: Parser) -> Tree {
            parser.parse(SAMPLE_HTML, None).unwrap()
        }

        #[fixture]
        fn blank_tree(mut parser: Parser) -> Tree {
            parser.parse("", None).unwrap()
        }

        #[fixture]
        fn html_text() -> Text {
            Text::new(SAMPLE_HTML.to_string())
        }

        #[fixture]
        fn blank_text() -> Text {
            Text::new("".to_string())
        }

        #[rstest]
        #[case::empty("")]
        #[case::short("some-attr")]
        #[case::long("some-attrasdasdasdasdasdasdasdasdasd")]
        #[case::long_single_br("some-attrasdasdasdasdas\ndasdasdasdasd")]
        #[case::long_multiple_br("some-attrasdas\ndasdasdasdasda\n\n\n\nsdas\n\nda\nsd\n")]
        fn insert(#[case] inserted: &str, mut html_text: Text, mut html_tree: Tree) {
            html_text.update(
                Change::Insert {
                    at: ATTRIBUTE_NAME_POS.into(),
                    text: inserted.to_string(),
                },
                &mut html_tree,
            );

            let mut modified: String = SAMPLE_HTML.to_string();
            modified.insert_str(
                html_text.br_indexes.row_start(ATTRIBUTE_NAME_POS.row) + ATTRIBUTE_NAME_POS.column,
                inserted,
            );

            let modified = Text::new(modified);

            assert_eq!(html_text, modified);
            let mut parser = parser();
            let modified_tree = parser.parse(modified.text.as_str(), None).unwrap();
            let updated_html = parser
                .parse(html_text.text.as_str(), Some(&html_tree))
                .unwrap();
            let mut prev = 0;
            for br in
                (1..html_text.br_indexes.row_count()).map(|i| html_text.br_indexes.row_start(i))
            {
                for i in prev..br {
                    let a = updated_html.root_node().descendant_for_byte_range(i, i);
                    let b = modified_tree.root_node().descendant_for_byte_range(i, i);
                    let (a, b) = match (a, b) {
                        (Some(a), Some(b)) => (a, b),
                        (None, None) => continue,
                        _ => panic!("different result found"),
                    };
                    assert_eq!(a.kind_id(), b.kind_id());
                    assert_eq!(a.is_named(), b.is_named());
                    assert_eq!(
                        a.utf8_text(html_text.text.as_bytes()),
                        b.utf8_text(modified.text.as_bytes())
                    );
                    assert_eq!(a.to_sexp(), b.to_sexp());
                }
                prev = br;
            }

            assert_eq!(prev, modified.text.len());
        }

        #[rstest]
        #[case::in_line(GridIndex { row: 1, col: 7 }, GridIndex {row: 1, col: 15})]
        #[case::across_lines(GridIndex { row: 5, col: 7 }, GridIndex {row: 8, col: 7})]
        fn delete(
            #[case] start: GridIndex,
            #[case] end: GridIndex,
            mut html_text: Text,
            mut html_tree: Tree,
        ) {
            let mut modified: String = SAMPLE_HTML.to_string();
            modified.drain(
                html_text.br_indexes.row_start(start.row) + start.col
                    ..html_text.br_indexes.row_start(end.row) + end.col,
            );

            html_text.update(Change::Delete { start, end }, &mut html_tree);

            let modified = Text::new(modified);

            assert_eq!(html_text, modified);
            let mut parser = parser();
            let modified_tree = parser.parse(modified.text.as_str(), None).unwrap();
            let updated_html = parser
                .parse(html_text.text.as_str(), Some(&html_tree))
                .unwrap();
            let mut prev = 0;
            for br in
                (1..html_text.br_indexes.row_count()).map(|i| html_text.br_indexes.row_start(i))
            {
                for i in prev..br {
                    let a = updated_html.root_node().descendant_for_byte_range(i, i);
                    let b = modified_tree.root_node().descendant_for_byte_range(i, i);
                    let (a, b) = match (a, b) {
                        (Some(a), Some(b)) => (a, b),
                        (None, None) => continue,
                        _ => panic!("different result found"),
                    };
                    assert_eq!(a.kind_id(), b.kind_id());
                    assert_eq!(a.is_named(), b.is_named());
                    assert_eq!(
                        a.utf8_text(html_text.text.as_bytes()),
                        b.utf8_text(modified.text.as_bytes())
                    );
                    assert_eq!(a.to_sexp(), b.to_sexp());
                }
                prev = br;
            }

            assert_eq!(prev, modified.text.len());
        }
    }
}
