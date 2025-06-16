use std::sync::OnceLock;

use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;
use fastrace::collector::Reporter;
use fastrace::prelude::SpanRecord;
use rt_trace::config::Config;
use rt_trace::consumer::SpanConsumer;
use rt_trace::initialize;
use rt_trace::span;
use rt_trace::span::RunTask;
use rt_trace::start;

fn init_fastrace() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        struct DummyReporter;
        impl Reporter for DummyReporter {
            fn report(&mut self, _spans: Vec<SpanRecord>) {}
        }
        let reporter = DummyReporter;
        fastrace::set_reporter(reporter, fastrace::collector::Config::default());
    });
}

fn init_rt_trace() {
    struct DummyReporter;

    impl SpanConsumer for DummyReporter {
        fn consume(&mut self, _spans: &[rt_trace::span::RawSpan]) {}
    }

    initialize(Config::default(), DummyReporter {});
    start();
}

fn fastrace_harness(n: usize) {
    use fastrace::prelude::*;

    let root = Span::root("parent", SpanContext::new(TraceId(12), SpanId::default()));
    for _ in 0..(n / 1000) {
        // We have to flush spans stored in SpanQueue for every 1000 iteration.
        let _g = root.set_local_parent();
        for _ in 0..1000 {
            let _guard = LocalSpan::enter_with_local_parent("child");
        }
    }
}

fn rt_trace_harness(n: usize) {
    fn dummy_rt_trace(n: usize) {
        for _ in 0..n {
            let _guard = span(span::Type::RunTask(RunTask::default()));
        }
    }
    dummy_rt_trace(n);
}

fn tracing_comparison(c: &mut Criterion) {
    init_fastrace();
    init_rt_trace();

    let mut bgroup = c.benchmark_group("compare");

    for n in &[10000, 100000, 1000000] {
        bgroup.bench_function(format!("fastrace/{n}"), |b| b.iter(|| fastrace_harness(*n)));
        bgroup.bench_function(format!("rt_trace/{n}"), |b| b.iter(|| rt_trace_harness(*n)));
    }

    bgroup.finish();
}

criterion_group!(benches, tracing_comparison);
criterion_main!(benches);
