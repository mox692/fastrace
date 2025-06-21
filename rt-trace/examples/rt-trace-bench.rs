// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

use std::fs::File;
use std::io::Write;

use rt_trace::config::Config;
use rt_trace::consumer::SpanConsumer;
use rt_trace::flush;
use rt_trace::initialize;
use rt_trace::span;
use rt_trace::span::RawSpan;
use rt_trace::span::RunTask;
use rt_trace::start;

fn init_rt_trace() {
    struct DummyConsumer;

    impl SpanConsumer for DummyConsumer {
        fn consume(&mut self, spans: &[RawSpan], writer: &mut Box<&mut dyn Write>) {}
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
    let mut file = File::create("rt_trace_bench.log").expect("Failed to create log file");
    init_rt_trace();

    let num_spans = 1_000_000;
    let now = std::time::Instant::now();
    rt_trace_harness(num_spans);
    println!("rt_trace_harness done!");
    flush(&mut file);
    println!("flush done!");
    let dur = now.elapsed();
    println!("dur: {:?}", &dur);
}
