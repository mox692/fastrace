use fastant::Instant;
use rt_trace::config::Config;
use rt_trace::consumer::SpanConsumer;
use rt_trace::initialize;
use rt_trace::span;
use rt_trace::span::RunTask;
use rt_trace::start;

fn fastrace_harness(n: usize) {
    use fastrace::prelude::*;

    let root = Span::root("parent", SpanContext::new(TraceId(12), SpanId::default()));
    for i in 0..(n / 10000) {
        // We have to flush spans stored in SpanQueue for every 10240 iteration.
        let _g = root.set_local_parent();
        for j in 0..10000 {
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
        fn consume(&mut self, spans: Vec<rt_trace::span::RawSpan>) {}
    }

    initialize(Config {}, DummyReporter {});
    start();
}

fn run_rt_trace(n: usize) {
    init_rt_trace();
    rt_trace_harness(n);
}

fn run_fastrace(n: usize) {
    init_fastrace();
    fastrace_harness(n);
}

fn main() {
    // run_fastrace(1000000);
    run_rt_trace(100000);
}
