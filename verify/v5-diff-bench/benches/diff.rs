use criterion::{Criterion, black_box, criterion_group, criterion_main};
use diff_bench::{build_tree, diff, mutate_tree};

fn bench_100(c: &mut Criterion) {
    let old = build_tree(3, 4); // ~85 nodes
    let new = mutate_tree(&old, 10);
    c.bench_function("diff_85nodes", |b| {
        b.iter(|| diff(black_box(&old), black_box(&new)))
    });
}

fn bench_1000(c: &mut Criterion) {
    let old = build_tree(4, 5); // ~780 nodes
    let new = mutate_tree(&old, 50);
    c.bench_function("diff_780nodes", |b| {
        b.iter(|| diff(black_box(&old), black_box(&new)))
    });
}

fn bench_5000(c: &mut Criterion) {
    let old = build_tree(5, 6); // ~9,330 nodes
    let new = mutate_tree(&old, 200);
    c.bench_function("diff_9330nodes", |b| {
        b.iter(|| diff(black_box(&old), black_box(&new)))
    });
}

fn bench_no_change(c: &mut Criterion) {
    let old = build_tree(4, 5);
    c.bench_function("diff_780nodes_no_change", |b| {
        b.iter(|| diff(black_box(&old), black_box(&old)))
    });
}

fn bench_full_replace(c: &mut Criterion) {
    let t1 = build_tree(3, 3);
    let t2 = build_tree(3, 3);
    c.bench_function("diff_full_replace", |b| {
        b.iter(|| diff(black_box(&t1), black_box(&t2)))
    });
}

criterion_group!(
    benches,
    bench_100,
    bench_1000,
    bench_5000,
    bench_no_change,
    bench_full_replace
);
criterion_main!(benches);
