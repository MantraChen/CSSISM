//! Benchmarks: insert and query throughput for the concurrent spatial index.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use cssism::{ConcurrentMapIndex, MapPoint};
use rand::Rng;

fn make_points(n: usize, rng: &mut impl Rng) -> Vec<MapPoint> {
    (0..n)
        .map(|i| {
            MapPoint::new(
                rng.gen_range(-10.0..10.0),
                rng.gen_range(-10.0..10.0),
                rng.gen_range(-5.0..5.0),
                (0..32).map(|_| rng.gen()).collect(),
                i as u64,
            )
        })
        .collect()
}

fn bench_insert_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_batch");
    let mut rng = rand::thread_rng();
    for size in [100, 1_000, 10_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            criterion::BenchmarkId::new("batch_size", size),
            &size,
            |b, &size| {
                let index = ConcurrentMapIndex::new();
                let batch = make_points(size, &mut rng);
                b.iter(|| {
                    let batch = batch.clone();
                    index.insert_batch(batch);
                });
            },
        );
    }
    group.finish();
}

fn bench_insert_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert_single");
    let mut rng = rand::thread_rng();
    group.throughput(Throughput::Elements(1));
    group.bench_function("one_point", |b| {
        let index = ConcurrentMapIndex::new();
        b.iter(|| {
            let p = MapPoint::new(
                rng.gen_range(-10.0..10.0),
                rng.gen_range(-10.0..10.0),
                rng.gen_range(-5.0..5.0),
                (0..32).map(|_| rng.gen()).collect(),
                rng.gen(),
            );
            index.insert(p);
        });
    });
    group.finish();
}

fn bench_nearest(c: &mut Criterion) {
    let mut rng = rand::thread_rng();
    let index = ConcurrentMapIndex::new();
    let batch = make_points(50_000, &mut rng);
    index.insert_batch(batch);

    let mut group = c.benchmark_group("nearest");
    for k in [1, 10, 100] {
        group.throughput(Throughput::Elements(k as u64));
        group.bench_with_input(
            criterion::BenchmarkId::new("k", k),
            &k,
            |b, &k| {
                b.iter(|| {
                    let x: f32 = rng.gen_range(-10.0..10.0);
                    let y: f32 = rng.gen_range(-10.0..10.0);
                    let z: f32 = rng.gen_range(-5.0..5.0);
                    let _ = index.nearest(x, y, z, k);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_insert_batch, bench_insert_single, bench_nearest);
criterion_main!(benches);
