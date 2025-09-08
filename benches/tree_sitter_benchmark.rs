use criterion::{Criterion, black_box, criterion_group, criterion_main};

/// Benchmark tree-sitter parsing performance (disabled - module not available)
fn benchmark_tree_sitter_parsing(c: &mut Criterion) {
    // Disabled due to missing tree_sitter module
    let mut group = c.benchmark_group("tree_sitter");
    group.bench_function("disabled", |b| {
        b.iter(|| {
            // Placeholder benchmark
            black_box(42)
        })
    });
    group.finish();
}

criterion_group!(benches, benchmark_tree_sitter_parsing);
criterion_main!(benches);
