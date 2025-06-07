// // Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

// use std::sync::OnceLock;

// use fastrace::collector::Reporter;
// use fastrace::prelude::SpanRecord;

// fn init_fastrace() {
//     static INIT: OnceLock<()> = OnceLock::new();
//     INIT.get_or_init(|| {
//         struct DummyReporter;
//         impl Reporter for DummyReporter {
//             fn report(&mut self, _spans: Vec<SpanRecord>) {}
//         }
//         let reporter = DummyReporter;
//         fastrace::set_reporter(reporter, fastrace::collector::Config::default());
//     });
// }
// fn fastrace_harness(n: usize) {
//     use fastrace::prelude::*;

//     let root = Span::root("parent", SpanContext::new(TraceId(12), SpanId::default()));
//     let _g = root.set_local_parent();
//     for _ in 0..n {
//         let _guard = LocalSpan::enter_with_local_parent("child");
//     }
// }

// fn main() {
//     init_fastrace();

//     let num_spans = 1_000_000;
//     let now = std::time::Instant::now();
//     fastrace_harness(num_spans);
//     println!("rt_trace_harness done!");
//     let dur = now.elapsed();
//     println!("dur: {:?}", &dur);
// }

fn main() {}
