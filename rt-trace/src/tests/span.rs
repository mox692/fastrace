use std::collections::HashSet;
use std::fmt::Debug;
use std::fs::File;
use std::hash::Hash;
use std::io::Write;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    config::Config,
    consumer::SpanConsumer,
    flush, initialize, span as mk_span,
    span::{RawSpan, RunTask, Type},
    start,
};
struct TestConsumer {
    collect: Arc<Mutex<Vec<RawSpan>>>,
}

impl SpanConsumer for TestConsumer {
    fn consume(&mut self, spans: &[RawSpan], _writer: &mut Box<&mut dyn Write>) {
        let mut collect = self.collect.lock().unwrap();
        collect.extend_from_slice(spans);
    }
}

fn setup() -> (Config, TestConsumer, Arc<Mutex<Vec<RawSpan>>>) {
    let collect = Arc::new(Mutex::new(vec![]));
    let consumer = TestConsumer {
        collect: collect.clone(),
    };
    let mut config = Config::new();
    config.consumer_thread_sleep_duration(Duration::MAX);
    (config, consumer, collect)
}

fn assert_equal_as_set<T: Debug + Eq + Hash + Clone>(a: &[T], b: &[T]) {
    let set_a: HashSet<_> = a.iter().cloned().collect();
    let set_b: HashSet<_> = b.iter().cloned().collect();
    assert_eq!(set_a, set_b);
}

#[test]
fn basic() {
    let (config, consumer, collect) = setup();
    let num_threads = 3;
    let mut handles = vec![];
    let mut file = File::create("./test_basic.log").unwrap();

    initialize(config, consumer);
    start();

    for i in 0..num_threads {
        let handle = std::thread::Builder::new()
            .name(format!("test-thread-{}", i))
            .spawn(move || {
                let _guard = mk_span(Type::RunTask(RunTask {
                    name: Some(format!("task{}", i).to_string()),
                    ..Default::default()
                }));
            })
            .unwrap();

        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    flush(&mut file);

    // this consumer doesn't call drain_descriptors, so we wouldn't get any descriptors.
    let expected = vec![
        Type::RunTask(RunTask {
            name: Some("task0".to_string()),
            ..Default::default()
        }),
        Type::RunTask(RunTask {
            name: Some("task1".to_string()),
            ..Default::default()
        }),
        Type::RunTask(RunTask {
            name: Some("task2".to_string()),
            ..Default::default()
        }),
    ];

    let got: Vec<Type> = collect
        .lock()
        .unwrap()
        .iter()
        .map(|span| span.typ.clone())
        .collect();

    assert_equal_as_set(&got, &expected);
}

#[test]
fn debug_annotation() {}
