// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

use rt_trace::config::Config;
use rt_trace::consumer::SpanConsumer;
use rt_trace::flush;
use rt_trace::initialize;
use rt_trace::span;
use rt_trace::span::RunTask;
use rt_trace::start;

fn init_rt_trace() {
    struct DummyConsumer;

    impl SpanConsumer for DummyConsumer {
        fn consume(&mut self, _spans: &[rt_trace::span::RawSpan]) {}
    }
    let consumer = DummyConsumer;
    initialize(Config::default(), consumer);
    start();
}

fn rt_trace_harness(n: usize) {
    for _ in 0..n {
        let _guard = span(span::Type::RunTask(RunTask::default()));
    }
}

fn main() {
    init_rt_trace();

    let num_spans = 1_000_000;
    let now = std::time::Instant::now();
    rt_trace_harness(num_spans);
    println!("rt_trace_harness done!");
    flush();
    println!("flush done!");
    let dur = now.elapsed();
    println!("dur: {:?}", &dur);
}
