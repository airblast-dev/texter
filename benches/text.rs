use criterion::{criterion_group, BatchSize, Criterion};
use texter::{
    change::{Change, GridIndex},
    core::text::Text,
};

fn text(c: &mut Criterion) {
    let text = Text::new(include_str!("sample_file.txt").to_string());
    c.bench_function("delete", |b| {
        b.iter_batched(
            || {
                (
                    text.clone(),
                    vec![
                        Change::Delete {
                            start: GridIndex { row: 602, col: 36 },
                            end: GridIndex { row: 954, col: 0 },
                        },
                        Change::Delete {
                            start: GridIndex { row: 120, col: 0 },
                            end: GridIndex { row: 398, col: 51 },
                        },
                        Change::Delete {
                            start: GridIndex { row: 19, col: 10 },
                            end: GridIndex { row: 40, col: 0 },
                        },
                        Change::Delete {
                            start: GridIndex { row: 0, col: 3 },
                            end: GridIndex { row: 0, col: 8 },
                        },
                    ],
                )
            },
            |(mut t, chs)| {
                for ch in chs {
                    t.update(ch, &mut ())
                }
            },
            BatchSize::SmallInput,
        );
    })
    .bench_function("insert", |b| {
        b.iter_batched(
            || {
                (
                    text.clone(),
                    vec![
                        // Single character case.
                        Change::Insert {
                            at: GridIndex { row: 0, col: 0 },
                            text: "c".to_string(),
                        },
                        Change::Insert {
                            at: GridIndex { row: 0, col: 0 },
                            text: "\n".to_string(),
                        },
                        Change::Insert {
                            at: GridIndex { row: 1, col: 0 },
                            text: "ShortString".to_string(),
                        },
                        Change::Insert {
                            at: GridIndex { row: 398, col: 51 },
                            text: "LargeString\n".repeat(100),
                        },
                        Change::Insert {
                            at: GridIndex { row: 398, col: 51 },
                            text: "MediumString\n".repeat(10),
                        },
                    ],
                )
            },
            |(mut text, chs)| {
                for ch in chs {
                    text.update(ch, &mut ());
                }
            },
            BatchSize::SmallInput,
        );
    })
    .bench_function("replace", |b| {
        b.iter_batched(
            || {
                (
                    text.clone(),
                    vec![
                        Change::Replace {
                            start: GridIndex { row: 0, col: 0 },
                            end: GridIndex { row: 955, col: 0 },
                            text: text.text.clone(),
                        },
                        Change::Replace {
                            start: GridIndex { row: 4, col: 0 },
                            end: GridIndex { row: 6, col: 0 },
                            text: "Shrinking".to_string(),
                        },
                        Change::Replace {
                            start: GridIndex { row: 4, col: 0 },
                            end: GridIndex { row: 6, col: 0 },
                            text: "Growing\n".repeat(20),
                        },
                        Change::Replace {
                            start: GridIndex { row: 6, col: 3 },
                            end: GridIndex { row: 6, col: 5 },
                            text: "Simple".repeat(20),
                        },
                    ],
                )
            },
            |(mut text, chs)| {
                for ch in chs {
                    text.update(ch, &mut ())
                }
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, text);
