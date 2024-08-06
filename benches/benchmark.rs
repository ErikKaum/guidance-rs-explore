use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::json;

use guidance_rs::guidance::to_regex;

fn benchmark_to_regex(c: &mut Criterion) {
    let json_value = json!({
        "type": "integer"
    });

    c.bench_function("to_regex", |b| {
        b.iter(|| to_regex(black_box(&json_value), None))
    });
}

criterion_group!(benches, benchmark_to_regex);
criterion_main!(benches);
