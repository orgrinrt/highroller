//! What a rolled index costs, per mechanism.
//!
//! The crate's own description promises "cheap runtime-unique IDs", and the
//! mechanism under that promise was a global `Mutex` locked once per id. This
//! measures it against the two atomic shapes that could replace it, uncontended
//! and under threads, because a lock's cost is mostly invisible until it is
//! contended.
//!
//! The arms are the alternatives somebody might actually choose, not a strawman:
//!
//! - `mutex` is what the crate shipped.
//! - `fetch_update` is a compare-and-swap loop, and is the arm that keeps the
//!   crate's exact value sequence, including its habit of never handing out MAX.
//! - `fetch_add` is the cheapest thing the hardware offers, and hands out the
//!   full range, so it is a behaviour change rather than a drop-in.
//! - `wide_masked` is the one that was actually adopted. The counter is wider than
//!   the index, so a plain `fetch_add` both detects exhaustion (the wide counter
//!   passes the narrow maximum long before it could wrap) and wraps by masking,
//!   because every index width's range is a power of two. It is a `fetch_add` and
//!   an `and`.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::thread;

static MUTEX_IDX: Mutex<u64> = Mutex::new(0);
static ATOMIC_IDX: AtomicU64 = AtomicU64::new(0);

const MAX: u64 = u64::MAX;

fn mutex_roll() -> u64 {
    let mut this = MUTEX_IDX.lock().unwrap();
    if *this == MAX {
        *this = 0;
    }
    let v = *this;
    *this += 1;
    v
}

fn atomic_fetch_update() -> u64 {
    let prev = ATOMIC_IDX
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
            Some(if v == MAX { 1 } else { v + 1 })
        })
        .expect("the closure returns Some for every input, so this cannot fail");
    if prev == MAX { 0 } else { prev }
}

fn atomic_fetch_add() -> u64 {
    ATOMIC_IDX.fetch_add(1, Ordering::Relaxed)
}

/// A 16-bit index carried in a 64-bit counter, which is the shipped default.
const NARROW_MAX: u64 = u16::MAX as u64;

fn wide_masked() -> u16 {
    // `NARROW_MAX + 1` is a power of two for every index width, so the remainder
    // is an `and` rather than a division.
    (ATOMIC_IDX.fetch_add(1, Ordering::Relaxed) & NARROW_MAX) as u16
}

fn wide_masked_strict() -> u16 {
    let prev = ATOMIC_IDX.fetch_add(1, Ordering::Relaxed);
    if prev > NARROW_MAX {
        // Unreachable in the benchmark; present so the branch is measured.
        return 0;
    }
    prev as u16
}

fn uncontended(c: &mut Criterion) {
    let mut g = c.benchmark_group("one thread");
    g.throughput(Throughput::Elements(1));
    g.bench_function("mutex", |b| b.iter(|| black_box(mutex_roll())));
    g.bench_function("fetch_update", |b| b.iter(|| black_box(atomic_fetch_update())));
    g.bench_function("fetch_add", |b| b.iter(|| black_box(atomic_fetch_add())));
    g.bench_function("wide_masked", |b| b.iter(|| black_box(wide_masked())));
    g.bench_function("wide_masked_strict", |b| b.iter(|| black_box(wide_masked_strict())));
    g.finish();
}

/// Every thread rolls `PER_THREAD` ids, and the whole batch is one sample.
///
/// Timing a single roll under contention would measure the sampler; timing the
/// batch measures what a program actually experiences.
const PER_THREAD: usize = 2_000;

fn contended(c: &mut Criterion) {
    let mut g = c.benchmark_group("threads");
    for threads in [2usize, 4, 8] {
        g.throughput(Throughput::Elements((threads * PER_THREAD) as u64));
        for (name, f) in [
            ("mutex", mutex_roll as fn() -> u64),
            ("fetch_update", atomic_fetch_update as fn() -> u64),
            ("fetch_add", atomic_fetch_add as fn() -> u64),
            ("wide_masked", (|| wide_masked() as u64) as fn() -> u64),
        ] {
            g.bench_with_input(BenchmarkId::new(name, threads), &threads, |b, &n| {
                b.iter(|| {
                    thread::scope(|s| {
                        for _ in 0..n {
                            s.spawn(|| {
                                for _ in 0..PER_THREAD {
                                    black_box(f());
                                }
                            });
                        }
                    });
                })
            });
        }
    }
    g.finish();
}

criterion_group!(benches, uncontended, contended);
criterion_main!(benches);
