use std::fmt::Write;
use std::io::Cursor;

use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use typeward::prelude::*;

type ScalarValue = or!(Ws<Null>, Ws<bool>, Ws<f64>);
type IntList = Delimited<Ws<LBracket>, Ws<RBracket>, Separated0<Ws<i64>, Ws<Comma>>>;

fn build_int_list_input(count: usize) -> String {
    let mut input = String::with_capacity(count.saturating_mul(6).saturating_add(2));
    input.push('[');

    for i in 0..count {
        if i > 0 {
            input.push_str(", ");
        }

        let value = (i % 97) as i64;
        let _ = write!(input, "{value}");
    }

    input.push(']');
    input
}

fn bench_scalar_parsers(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalar_parsers");

    let float_input = "   -1234567.890123e-17";
    group.bench_function("ws_f64", |b| {
        b.iter(|| {
            let parsed = parse_complete::<Ws<f64>>(black_box(float_input)).unwrap();
            black_box(parsed.into_inner())
        });
    });

    let alt_input = "  -42.125";
    group.bench_function("or_ws_null_bool_f64_tail_match", |b| {
        b.iter(|| {
            let parsed = parse_complete::<ScalarValue>(black_box(alt_input)).unwrap();
            black_box(parsed)
        });
    });

    group.finish();
}

fn bench_delimited_separated_lists(c: &mut Criterion) {
    let mut group = c.benchmark_group("delimited_separated_list");

    for &count in &[8_usize, 64, 512] {
        let input = build_int_list_input(count);
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(count), &input, |b, input| {
            b.iter(|| {
                let parsed = parse_complete::<IntList>(black_box(input.as_str())).unwrap();
                black_box(parsed.inner.items.len())
            });
        });
    }

    group.finish();
}

fn bench_streaming_read_input(c: &mut Criterion) {
    let bytes = build_int_list_input(512).into_bytes();

    let mut group = c.benchmark_group("stream_input");
    group.throughput(Throughput::Bytes(bytes.len() as u64));

    group.bench_function("int_list_window_16", |b| {
        b.iter_batched(
            || ReadInputStream::<_, 16>::new(Cursor::new(bytes.as_slice())),
            |stream| {
                let parsed = parse_complete_input::<_, IntList>(stream.as_input()).unwrap();
                black_box(parsed.inner.items.len())
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("int_list_window_64", |b| {
        b.iter_batched(
            || ReadInputStream::<_, 64>::new(Cursor::new(bytes.as_slice())),
            |stream| {
                let parsed = parse_complete_input::<_, IntList>(stream.as_input()).unwrap();
                black_box(parsed.inner.items.len())
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    hot_paths,
    bench_scalar_parsers,
    bench_delimited_separated_lists,
    bench_streaming_read_input
);
criterion_main!(hot_paths);
