use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;
use rt_trace::backend::perfetto::PerfettoReporter;
use rt_trace::config::Config;
use rt_trace::initialize;
use rt_trace::span;
use rt_trace::span::RuntimeStart;
use rt_trace::start;

fn init_rt_trace_perfetto() {
    let perfetto_reporter = PerfettoReporter::new("fastrace_perfetto_test");
    initialize(Config::default(), perfetto_reporter);
    start();
}

fn rt_trace_perfetto_harness(n: usize) {
    fn dummy_rt_trace_perfetto(n: usize) {
        for _ in 0..n {
            let _guard = span(span::Type::RuntimeStart(RuntimeStart {}));
        }
    }
    dummy_rt_trace_perfetto(n);
}

fn perfetto(c: &mut Criterion) {
    init_rt_trace_perfetto();

    let mut bgroup = c.benchmark_group("compare");

    for n in &[100, 1000, 10000] {
        bgroup.bench_function(format!("rt_trace_perfetto/{n}"), |b| {
            b.iter(|| rt_trace_perfetto_harness(*n))
        });
    }

    bgroup.finish();
}

criterion_group!(benches, perfetto);
criterion_main!(benches);
