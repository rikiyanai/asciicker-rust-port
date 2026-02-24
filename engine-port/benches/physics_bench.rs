// Placeholder -- populated in Task 2
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_placeholder(c: &mut Criterion) {
    c.bench_function("placeholder", |b| {
        b.iter(|| {
            criterion::black_box(1 + 1);
        });
    });
}

criterion_group!(benches, bench_placeholder);
criterion_main!(benches);
