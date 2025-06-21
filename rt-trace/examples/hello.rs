#![allow(unused)]
#![allow(dead_code)]

use std::fs::File;

use rt_trace::{
    backend::perfetto::PerfettoReporter,
    config::Config,
    flush, initialize,
    span::{self, RunTask},
    start,
};

fn main() {
    // single_thread();
    multi_thread();
}

fn single_thread() {
    let mut file = File::create("./pftrace.log").expect("Failed to create log file");
    let consumer = PerfettoReporter::new();

    initialize(Config::default(), consumer);

    start();

    let jh = std::thread::spawn(|| {
        // Start tracing
        {
            let _guard = rt_trace::span(span::Type::RunTask(RunTask {
                name: Some("task1".to_string()),
                ..Default::default()
            }));
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
        {
            let _guard = rt_trace::span(span::Type::RunTask(RunTask {
                name: Some("task2".to_string()),
                ..Default::default()
            }));
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
    });

    jh.join().unwrap();

    flush(&mut file);
}

fn multi_thread() {
    let mut file = File::create("./pftrace.log").expect("Failed to create log file");
    let consumer = PerfettoReporter::new();

    initialize(Config::default(), consumer);

    start();

    let num_threads = 10;
    let mut handles = vec![];
    for _ in 0..num_threads {
        let handle = std::thread::spawn(move || {
            // Start tracing
            {
                let _guard = rt_trace::span(span::Type::RunTask(RunTask {
                    name: Some("task1".to_string()),
                    ..Default::default()
                }));
                std::thread::sleep(std::time::Duration::from_micros(100));
            }
            {
                let _guard = rt_trace::span(span::Type::RunTask(RunTask {
                    name: Some("task2".to_string()),
                    ..Default::default()
                }));
                std::thread::sleep(std::time::Duration::from_micros(100));
            }
        });
        handles.push(handle);
    }

    for handle in handles.into_iter() {
        handle.join().unwrap();
    }

    flush(&mut file);
}
