use criterion::{black_box, criterion_group, BatchSize, Criterion};
use texter::core::text::Text;

fn text_lines(c: &mut Criterion) {
    const SAMPLE_STR: &str = include_str!("sample_file.txt");
    c.bench_function("text_lines_iter", |a| {
        a.iter_batched(
            || Text::new(SAMPLE_STR.to_string()),
            |t| {
                for line in t.lines() {
                    black_box(line);
                }
            },
            BatchSize::SmallInput,
        );
    });
    c.bench_function("text_lines_std_iter", |a| {
        a.iter_batched(
            || Text::new(SAMPLE_STR.to_string()),
            |t| {
                // Not exactly the same as the benchmark above, but still a practical measurment.
                for line in t.text.lines() {
                    black_box(line);
                }
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, text_lines);
