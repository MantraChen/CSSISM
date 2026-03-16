//! Benchmarks: approximate memory usage of the index for a given number of points.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use cssism::{ConcurrentMapIndex, MapPoint};
use rand::Rng;
use std::alloc::System;
use std::sync::atomic::{AtomicUsize, Ordering};

#[global_allocator]
static ALLOC: TrackingAllocator = TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

struct TrackingAllocator;

unsafe impl std::alloc::GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        System.dealloc(ptr, layout);
    }
}

fn reset_alloc_counter() {
    ALLOCATED.store(0, Ordering::Relaxed);
}

fn current_allocated() -> usize {
    ALLOCATED.load(Ordering::Relaxed)
}

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

fn bench_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    group.sample_size(10);
    let mut rng = rand::thread_rng();

    for n in [1_000, 10_000, 50_000, 100_000] {
        group.bench_with_input(
            criterion::BenchmarkId::new("points", n),
            &n,
            |b, &n| {
                b.iter(|| {
                    reset_alloc_counter();
                    let index = ConcurrentMapIndex::new();
                    let batch = make_points(n, &mut rng);
                    let (inserted, _) = index.insert_batch(batch);
                    let bytes = current_allocated();
                    black_box((inserted, bytes));
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_memory);
criterion_main!(benches);
