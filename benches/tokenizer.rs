use criterion::{black_box, criterion_group, criterion_main, Criterion, Bencher};

fn bench_tokenizer(c: &mut Criterion) {
    let src = include_str!("../src/server/mod.rs");
    c.bench_function("tokenizer", |b: &mut Bencher| b.iter(|| black_box(codeguards_mcp::analyzer::tokenize_source(src))));
}

criterion_group!(benches, bench_tokenizer);
criterion_main!(benches);