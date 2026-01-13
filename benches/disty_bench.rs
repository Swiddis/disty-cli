use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use disty_cli::{kde::KDE, parsing, stats::Stats};
use std::io::Write as IoWrite;
use tempfile::NamedTempFile;

fn generate_test_file(n: usize) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    for i in 1..=n {
        writeln!(file, "{}", i).unwrap();
    }
    file.flush().unwrap();
    file
}

fn bench_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing");

    for size in [1_000, 10_000, 100_000, 1_000_000] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("read_file_mmap", size), &size, |b, &size| {
            let temp_file = generate_test_file(size);
            b.iter(|| {
                let file = temp_file.reopen().unwrap();
                let data = parsing::read_file_mmap(&file, None);
                black_box(data)
            });
        });
    }

    group.finish();
}

fn bench_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("stats");

    for size in [1_000, 10_000, 100_000, 1_000_000] {
        let data: Vec<f64> = (1..=size).map(|i| i as f64).collect();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("stats_new", size), &data, |b, data| {
            b.iter(|| {
                let stats = Stats::new(black_box(data.clone()));
                black_box(stats)
            });
        });

        let stats = Stats::new(data.clone());

        group.bench_with_input(
            BenchmarkId::new("quantile_median", size),
            &stats,
            |b, stats| {
                b.iter(|| black_box(stats.quantile(black_box(0.5))));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("quantile_99th", size),
            &stats,
            |b, stats| {
                b.iter(|| black_box(stats.quantile(black_box(0.99))));
            },
        );
    }

    group.finish();
}

fn bench_kde(c: &mut Criterion) {
    let mut group = c.benchmark_group("kde");

    for size in [1_000, 10_000, 100_000] {
        let data: Vec<f64> = (1..=size).map(|i| i as f64).collect();

        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(BenchmarkId::new("kde_new", size), &data, |b, data| {
            b.iter(|| {
                let kde = KDE::new(black_box(data));
                black_box(kde)
            });
        });

        let kde = KDE::new(&data);

        group.bench_with_input(BenchmarkId::new("kde_pdf_center", size), &kde, |b, kde| {
            let center = (size / 2) as f64;
            b.iter(|| black_box(kde.pdf(black_box(center))));
        });

        group.bench_with_input(BenchmarkId::new("kde_pdf_edge", size), &kde, |b, kde| {
            let edge = 1.0;
            b.iter(|| black_box(kde.pdf(black_box(edge))));
        });

        // Benchmark multiple PDF evaluations (like plotting would do)
        group.bench_with_input(
            BenchmarkId::new("kde_pdf_160_points", size),
            &kde,
            |b, kde| {
                let (min, max) = kde.bounds();
                b.iter(|| {
                    for i in 0..160 {
                        let x = min + (max - min) * (i as f64 / 159.0);
                        black_box(kde.pdf(black_box(x)));
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");

    for size in [1_000, 10_000, 100_000, 1_000_000] {
        group.throughput(Throughput::Elements(size as u64));

        group.bench_with_input(
            BenchmarkId::new("parse_and_stats", size),
            &size,
            |b, &size| {
                let temp_file = generate_test_file(size);
                b.iter(|| {
                    let file = temp_file.reopen().unwrap();
                    let data = parsing::read_file_mmap(&file, None);
                    let stats = Stats::new(data);
                    black_box(stats)
                });
            },
        );
    }

    // Only benchmark smaller sizes for full pipeline with KDE (it's expensive)
    for size in [1_000, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("parse_stats_and_kde", size),
            &size,
            |b, &size| {
                let temp_file = generate_test_file(size);
                b.iter(|| {
                    let file = temp_file.reopen().unwrap();
                    let data = parsing::read_file_mmap(&file, None);
                    let stats = Stats::new(data);
                    let kde = KDE::new(&stats.data);
                    // Evaluate PDF at one point to ensure KDE is fully used
                    let result = kde.pdf((size / 2) as f64);
                    black_box(result)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parsing,
    bench_stats,
    bench_kde,
    bench_full_pipeline
);
criterion_main!(benches);
