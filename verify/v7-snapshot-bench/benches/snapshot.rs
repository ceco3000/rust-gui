use criterion::{black_box, criterion_group, criterion_main, Criterion};
use snapshot_bench::*;

criterion_group!(
    benches,
    bench_todo_serde_json,
    bench_crud_serde_json,
    bench_pressure_json,
    bench_todo_postcard,
    bench_crud_postcard,
);
criterion_main!(benches);

// ── JSON ──

fn bench_todo_serde_json(c: &mut Criterion) {
    let s = build_todo();
    c.bench_function("json_serialize_todo", |b| b.iter(|| serde_json::to_string(black_box(&s)).unwrap()));
    let json = serde_json::to_string(&s).unwrap();
    c.bench_function("json_deserialize_todo", |b| b.iter(|| serde_json::from_str::<TodoState>(black_box(&json)).unwrap()));
}

fn bench_crud_serde_json(c: &mut Criterion) {
    let s = build_crud();
    c.bench_function("json_serialize_crud", |b| b.iter(|| serde_json::to_string(black_box(&s)).unwrap()));
    let json = serde_json::to_string(&s).unwrap();
    c.bench_function("json_deserialize_crud", |b| b.iter(|| serde_json::from_str::<CrudState>(black_box(&json)).unwrap()));
}

fn bench_pressure_json(c: &mut Criterion) {
    let s = build_pressure();
    c.bench_function("json_serialize_pressure", |b| b.iter(|| serde_json::to_string(black_box(&s)).unwrap()));
    let json = serde_json::to_string(&s).unwrap();
    c.bench_function("json_deserialize_pressure", |b| b.iter(|| serde_json::from_str::<PressureState>(black_box(&json)).unwrap()));
}

// ── postcard (二进制) ──

fn bench_todo_postcard(c: &mut Criterion) {
    let s = build_todo();
    c.bench_function("postcard_serialize_todo", |b| b.iter(|| postcard::to_allocvec(black_box(&s)).unwrap()));
    let bin = postcard::to_allocvec(&s).unwrap();
    c.bench_function("postcard_deserialize_todo", |b| b.iter(|| postcard::from_bytes::<TodoState>(black_box(&bin)).unwrap()));
}

fn bench_crud_postcard(c: &mut Criterion) {
    let s = build_crud();
    c.bench_function("postcard_serialize_crud", |b| b.iter(|| postcard::to_allocvec(black_box(&s)).unwrap()));
    let bin = postcard::to_allocvec(&s).unwrap();
    c.bench_function("postcard_deserialize_crud", |b| b.iter(|| postcard::from_bytes::<CrudState>(black_box(&bin)).unwrap()));
}
