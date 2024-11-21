/// This is a very simple text editor to show the what can be done using [`texter`].
///
/// The demo is aimed to be similar to the editor nano which many linux users will be familiar
/// with. There is many core text editor features missing in the demo to keep things simple to
/// understand.
///
/// You many notice code using [`texter`] is actually fairly limited in the example. This is somewhat of the goal,
/// removing error prone code that is often rewritten in many similar projects such as LSP's, and
/// text editors and instead providing an interface to an efficient, well tested library that can
/// very easily interface with external tooling (such as LSP <=> Text Editors <=> tree-sitter).
mod cursor;

use std::{collections::HashMap, io::Stdout};

use cursor::Cursor;
use ratatui::{
    crossterm::{
        self,
        event::{Event, KeyCode, KeyEvent, KeyModifiers},
    },
    layout::{Position, Rect},
    prelude::CrosstermBackend,
    style::{Color, Style, Stylize},
    Frame, Terminal,
};
use streaming_iterator::StreamingIterator;
use texter::{actions::DeletePreviousChar, core::text::Text, updateables::Updateable};
use tree_sitter::{Language, Parser, Point, Query, QueryCursor, Tree};
use tree_sitter_rust::LANGUAGE;

fn main() {
    let term = ratatui::init();
    App::new().run(term);
    ratatui::restore();
}

/// A cursor and tree bundled together to be updated by [`texter::core::text::Text::update`].
///
/// The [`Updateable`] trait is already implemented for [`Tree`] when the `tree-sitter` feature is
/// enabled so we just have to implement [`Updateable`] for our [`Cursor`]. 
///
/// See the `cursor.rs` file for its implementation.
#[derive(Clone, Debug)]
struct CursorTree {
    cursor: Cursor,
    tree: Tree,
}

impl Updateable for CursorTree {
    fn update(&mut self, ctx: texter::updateables::UpdateContext) {
        // UpdateContext is cheap to clone, it is literally a bunch of references.
        self.cursor.update(ctx.clone());
        self.tree.update(ctx);
    }
}

struct App {
    /// The content of our editor.
    text: Text,

    /// Our cursor and tree sitter that will be passed to `Text::update`.
    cursor_tree: CursorTree,
    parser: Parser,

    /// The number of rows we should skip before we start rendering.
    ///
    /// This is only used when rendering the text.
    row_offset: usize,

    /// `tree-sitter-rust`'s grammers already provides a neat query for syntax highlighting so we can just use that.
    query: Query,

    /// A bunch of random colors generated at runtime to be used when syntax highlighting.
    ///
    /// The actual colors are generated after parsing the file.
    colors: HashMap<u16, Color>,
    quit: bool,
}

impl App {
    fn new() -> Self {
        const CURSOR_FILE: &str = include_str!("cursor.rs");
        let mut parser = Parser::new();
        let lang = Language::new(LANGUAGE);
        parser.set_language(&lang).unwrap();
        let tree = parser.parse(CURSOR_FILE, None).unwrap();
        Self {
            text: Text::new(CURSOR_FILE.to_string()),
            cursor_tree: CursorTree {
                tree,
                cursor: Cursor::default(),
            },
            parser,
            query: Query::new(&lang, tree_sitter_rust::HIGHLIGHTS_QUERY).unwrap(),
            colors: Default::default(),
            quit: false,
            row_offset: 0,
        }
    }
    fn update(&mut self, event: Event) {
        if let Event::Key(ke) = event {
            self.handle_keyboard_event(ke);
        }
    }

    fn handle_keyboard_event(&mut self, event: KeyEvent) {
        if event.modifiers == KeyModifiers::CONTROL && event.code == KeyCode::Char('q') {
            self.quit = true;
            return;
        }

        let cursor = &mut self.cursor_tree.cursor;
        match event {
            KeyEvent {
                code: KeyCode::Char(c),
                ..
            } => {
                // This call will process the new changes, and provide an `UpdateContext` to
                // `TreeCursor::update` which we can use to update our cursor.
                self.text
                    .insert(&c.to_string(), cursor.byte_pos(), &mut self.cursor_tree);
            }
            KeyEvent {
                code: KeyCode::Enter,
                ..
            } => {
                self.text
                    .insert("\n", cursor.byte_pos(), &mut self.cursor_tree);
            }
            KeyEvent {
                code: KeyCode::Backspace,
                ..
            } => {
                // Here we use a pre defined action from `texter`.
                // It simply deletes the previous character, and moves the row up if it was a line
                // seperator (AKA it is at the start of the line).
                self.text.update_with_action(
                    &mut DeletePreviousChar(cursor.byte_pos()),
                    &mut self.cursor_tree,
                );
            }
            KeyEvent {
                code: KeyCode::Right,
                ..
            } => {
                let row = self.text.get_row(cursor.row);

                // The cursor is already at the end, nothing to do here.
                if row.len() <= cursor.col_cursor {
                    return;
                }

                cursor.col_cursor += 1;

                // Move the byte offset by one characters byte count.
                // This could be replaced with an +=1 if we only want to support ASCII.
                cursor.col_byte += row[cursor.col_byte..]
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or_default();
            }
            KeyEvent {
                code: KeyCode::Left,
                ..
            } => {
                let row = self.text.get_row(cursor.row);

                // The cursor is at the start of the line, we cannot move it further so we return.
                if cursor.col_cursor == 0 {
                    return;
                }

                cursor.col_cursor -= 1;
                // Subtract the chars length in bytes from the cursor position.
                // Same as KeyCode::Right, we could just -=1 if we only want to support ASCII.
                cursor.col_byte -= row[..cursor.col_byte]
                    .chars()
                    .next_back()
                    .map(|c| c.len_utf8())
                    .unwrap_or_default();
            }
            KeyEvent {
                code: KeyCode::Up, ..
            } => {
                // Attempt to move the cursor up and clamp the column positions if it is shorter
                // than the previous column.
                cursor.row = cursor.row.saturating_sub(1);
                let row = self.text.get_row(cursor.row);
                cursor.col_byte = cursor.col_byte.min(row.len());
                cursor.col_cursor = cursor.col_cursor.min(row.chars().count());
            }
            KeyEvent {
                code: KeyCode::Down,
                ..
            } => {
                let row_count = self.text.br_indexes.row_count();

                // Same as KeyCode::Up, the values are clamped so the cursor doesn't point to the
                // middle of knowhere.

                cursor.row = (cursor.row + 1).min(row_count - 1);

                let row = self.text.get_row(cursor.row);
                cursor.col_byte = cursor.col_byte.min(row.len());
                cursor.col_cursor = cursor.col_cursor.min(row.chars().count());
            }
            _ => {}
        }
    }

    fn row_offset(&mut self, f: &mut Frame) -> usize {
        let area = f.area();
        let height = area.height;
        let cursor = &self.cursor_tree.cursor;
        // move the offset down as needed to make the cursor position visible.
        while self.row_offset + (height as usize) <= cursor.row {
            self.row_offset += 1;
        }
        while self.row_offset > cursor.row {
            self.row_offset -= 1;
        }

        area.x as usize + self.row_offset
    }

    fn draw(&mut self, f: &mut Frame) {
        let row_offset = self.row_offset(f);
        f.set_cursor_position(Position {
            x: self.cursor_tree.cursor.col_cursor as u16,
            y: (self.cursor_tree.cursor.row - row_offset) as u16,
        });

        self.cursor_tree.tree = self
            .parser
            .parse(self.text.text.as_str(), Some(&self.cursor_tree.tree))
            .unwrap();

        let area = f.area();
        let buf = f.buffer_mut();
        for (row, line) in area.rows().zip(self.text.lines().skip(row_offset)) {
            buf.set_string(row.x, row.y, line, Style::new().blue());
        }
        buf.content.iter_mut().for_each(|cell| {
            // reset the cell where it is empty to avoid coloring the cursor when it is not over a
            // node.
            if cell.symbol() == " " {
                cell.reset();
            }
        });

        let mut qc = QueryCursor::new();
        let mut captures = qc.captures(
            &self.query,
            self.cursor_tree.tree.root_node(),
            self.text.text.as_bytes(),
        );
        captures.set_point_range(std::ops::Range {
            start: Point {
                row: row_offset,
                column: 0,
            },
            end: Point {
                row: row_offset + (area.height as usize) + 1,
                column: 0,
            },
        });

        captures.for_each(|(capture, _)| {
            for cap in capture.captures {
                let start = cap.node.start_position();
                let end = cap.node.end_position();
                if cap.node.is_extra() || !cap.node.is_named() {
                    continue;
                }
                // We only style nodes that don't span beyond across multiple lines.
                //
                // This is done intentionaly to simplify the example.
                if start.row != end.row {
                    continue;
                }
                let id = cap.node.kind_id();

                // Get the color for the node kind, or if not found add a new color for the node
                // kind.
                let color = if let Some(color) = self.colors.get(&id) {
                    color
                } else {
                    let color = Color::Rgb(rand::random(), rand::random(), rand::random());
                    self.colors.insert(id, color);
                    self.colors.get(&id).unwrap()
                };

                let row = self.text.get_row(start.row);

                let first_row_start_col = row
                    .char_indices()
                    .skip_while(|(i, _)| *i < start.column)
                    .take_while(|(i, _)| *i < end.column)
                    .count();

                buf.set_style(
                    Rect {
                        x: start.column as u16,
                        width: first_row_start_col as u16,
                        y: start.row as u16 - row_offset as u16,
                        height: 1,
                    },
                    *color,
                );
            }
        });
    }

    fn run(&mut self, mut term: Terminal<CrosstermBackend<Stdout>>) {
        loop {
            term.draw(|f| {
                self.draw(f);
            })
            .unwrap();

            self.update(crossterm::event::read().unwrap());

            if self.quit {
                return;
            }
        }
    }
}
