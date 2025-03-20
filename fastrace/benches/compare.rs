// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use rt_trace::config::Config;
use rt_trace::consumer::SpanConsumer;
use rt_trace::initialize;
use rt_trace::span;
use rt_trace::span::RunTask;
use rt_trace::start;

fn init_opentelemetry() {
    use tracing_subscriber::prelude::*;

    let opentelemetry = tracing_opentelemetry::layer();
    tracing_subscriber::registry()
        .with(opentelemetry)
        .try_init()
        .unwrap();
}

fn init_fastrace() {
    struct DummyReporter;

    impl fastrace::collector::Reporter for DummyReporter {
        fn report(&mut self, _spans: Vec<fastrace::prelude::SpanRecord>) {}
    }

    fastrace::set_reporter(DummyReporter, fastrace::collector::Config::default());
}

fn init_rt_trace() {
    struct DummyReporter;

    impl SpanConsumer for DummyReporter {
        fn consume(&mut self, spans: &[rt_trace::span::RawSpan]) {}
    }

    initialize(Config {}, DummyReporter {});
    start();
}

fn opentelemetry_harness(n: usize) {
    fn dummy_opentelementry(n: usize) {
        for _ in 0..n {
            let child = tracing::span!(tracing::Level::TRACE, "child");
            let _enter = child.enter();
        }
    }

    let root = tracing::span!(tracing::Level::TRACE, "parent");
    let _enter = root.enter();

    dummy_opentelementry(n);
}

fn rustracing_harness(n: usize) {
    fn dummy_rustracing(n: usize, span: &rustracing::span::Span<()>) {
        for _ in 0..n {
            let _child_span = span.child("child", |c| c.start_with_state(()));
        }
    }

    let (span_tx, span_rx) = crossbeam::channel::bounded(1000);

    {
        let tracer = rustracing::Tracer::with_sender(rustracing::sampler::AllSampler, span_tx);
        let parent_span = tracer.span("parent").start_with_state(());
        dummy_rustracing(n, &parent_span);
    }

    let _r = span_rx.iter().collect::<Vec<_>>();
}

fn fastrace_harness(n: usize) {
    use fastrace::prelude::*;

    let root = Span::root("parent", SpanContext::new(TraceId(12), SpanId::default()));
    for _ in 0..(n / 10000) {
        // We have to flush spans stored in SpanQueue for every 10240 iteration.
        let _g = root.set_local_parent();
        for _ in 0..10000 {
            let _guard = LocalSpan::enter_with_local_parent("child");
        }
    }
}

fn rt_trace_harness(n: usize) {
    fn dummy_fastrace(n: usize) {
        for _ in 0..n {
            let _guard = span(span::Type::RunTask(RunTask {}), 12);
        }
    }
    dummy_fastrace(n);
}

fn tracing_comparison(c: &mut Criterion) {
    init_opentelemetry();
    init_fastrace();
    init_rt_trace();

    let mut bgroup = c.benchmark_group("compare");

    for n in &[10000, 100000, 1000000] {
        // bgroup.bench_function(format!("Tokio Tracing/{n}"), |b| {
        //     b.iter(|| opentelemetry_harness(*n))
        // });
        // bgroup.bench_function(format!("Rustracing/{n}"), |b| {
        //     b.iter(|| rustracing_harness(*n))
        // });
        bgroup.bench_function(format!("fastrace/{n}"), |b| b.iter(|| fastrace_harness(*n)));
        bgroup.bench_function(format!("rt_trace/{n}"), |b| b.iter(|| rt_trace_harness(*n)));
    }

    bgroup.finish();
}

criterion_group!(benches, tracing_comparison);
criterion_main!(benches);
