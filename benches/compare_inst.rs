use std::sync::OnceLock;

use fastrace::{collector::Reporter, prelude::SpanRecord};
use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use rt_trace::{config::Config, consumer::SpanConsumer, initialize, start};

#[library_benchmark]
#[bench::first(args = (), setup = init_fastrace)]
fn bench_fastrace(_: ()) {
    let n = 100_000;

    let root = fastrace::Span::root(
        "parent",
        fastrace::prelude::SpanContext::new(
            fastrace::prelude::TraceId(12),
            fastrace::prelude::SpanId::default(),
        ),
    );
    for _ in 0..(n / 1000) {
        // We have to flush spans stored in SpanQueue for every 1000 iteration.
        let _g = root.set_local_parent();
        for _ in 0..1000 {
            let _guard = fastrace::local::LocalSpan::enter_with_local_parent("child");
        }
    }
}

#[library_benchmark]
#[bench::first(args = (), setup = init_rt_trace)]
fn bench_rt_trace(_: ()) {
    let n = 100_000;
    fn dummy_rt_trace(n: usize) {
        for _ in 0..n {
            let _guard = rt_trace::span(rt_trace::span::Type::RuntimeStart(
                rt_trace::span::RuntimeStart {},
            ));
        }
    }
    std::hint::black_box(dummy_rt_trace(n));
}

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

library_benchmark_group!(name = bench_group; benchmarks = bench_rt_trace, bench_fastrace);
main!(library_benchmark_groups = bench_group);
