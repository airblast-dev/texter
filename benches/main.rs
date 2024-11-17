mod text;
mod text_lines;

use criterion::criterion_main;

criterion_main!(text::benches, text_lines::benches);
