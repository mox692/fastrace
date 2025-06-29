use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use rt_trace::{backend::perfetto::PerfettoReporter, config::Config, initialize, start};

#[library_benchmark]
#[bench::first(args = (), setup = init_rt_trace)]
fn bench_rt_trace(_: ()) {
    let n = 100_000;
    fn dummy_rt_trace_perfetto(n: usize) {
        for _ in 0..n {
            let _guard = rt_trace::span(rt_trace::span::Type::RunTask(
                rt_trace::span::RunTask::default(),
            ));
        }
    }
    std::hint::black_box(dummy_rt_trace_perfetto(n));
}

fn init_rt_trace() {
    let perfetto_reporter = PerfettoReporter::new();
    initialize(Config::default(), perfetto_reporter);
    start();
}

library_benchmark_group!(name = bench_group; benchmarks = bench_rt_trace);
main!(library_benchmark_groups = bench_group);
