// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

use rt_trace::backend::perfetto::PerfettoReporter;
use rt_trace::config::Config;
use rt_trace::consumer::SpanConsumer;
use rt_trace::flush;
use rt_trace::initialize;
use rt_trace::span;
use rt_trace::span::RunTask;
use rt_trace::start;

fn init_rt_trace() {
    let consumer = PerfettoReporter::new("./test.log");
    initialize(Config {}, consumer);
    start();
}

fn rt_trace_harness(n: usize, num_thread: usize) {
    let mut handles = vec![];
    for i in 0..num_thread {
        let h = std::thread::Builder::new()
            .name(format!("thread {i}"))
            .spawn(move || {
                for _ in 0..n / num_thread {
                    let _guard = span(span::Type::RunTask(RunTask::default()));
                }
            });
        handles.push(h);
    }
    for h in handles {
        h.unwrap().join().unwrap()
    }
}

fn main() {
    init_rt_trace();

    let num_spans = 1_000_000;
    let num_threads = 4;
    let now = std::time::Instant::now();
    rt_trace_harness(num_spans, num_threads);
    println!("rt_trace_harness done!");
    flush();
    println!("flush done!");
    let dur = now.elapsed();
    println!("dur: {:?}", &dur);
}
