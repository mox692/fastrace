use std::{
    io::{self, Write},
    sync::mpsc,
};

use rt_trace::{
    backend::perfetto::PerfettoReporter,
    config::Config,
    flush, initialize,
    span::{self, RunTask},
    start,
};

fn main() {
    let mut all_tx = vec![];
    let consumer = PerfettoReporter::new();

    initialize(Config::default(), consumer);
    start();

    let num_threads = 4;
    let mut handles = vec![];
    for _ in 0..num_threads {
        let (tx, rx) = mpsc::channel::<()>();
        all_tx.push(tx);
        let handle = std::thread::spawn(move || {
            while let Err(_) = rx.recv_timeout(std::time::Duration::from_micros(10)) {
                // Start tracing
                {
                    let _guard = rt_trace::span(span::Type::RunTask(RunTask {
                        name: Some("task1".to_string()),
                        ..Default::default()
                    }));
                    std::thread::sleep(std::time::Duration::from_micros(10));
                    // std::thread::sleep(std::time::Duration::from_secs(1));
                }
                {
                    let _guard = rt_trace::span(span::Type::RunTask(RunTask {
                        name: Some("task2".to_string()),
                        ..Default::default()
                    }));
                    std::thread::sleep(std::time::Duration::from_micros(10));
                    // std::thread::sleep(std::time::Duration::from_secs(1));
                }
            }
        });
        handles.push(handle);
    }

    print!("Flight recording started.\n");
    print!("Enter to terminate the program gracefully: ");
    io::stdout().flush().unwrap();

    let mut buf = String::new();
    io::stdin()
        .read_line(&mut buf)
        .expect("Failed to read line");

    for tx in all_tx {
        tx.send(()).unwrap();
    }

    for handle in handles.into_iter() {
        handle.join().unwrap();
    }

    let mut file = std::fs::File::create("./single.log").unwrap();
    flush(&mut file);

    println!("flush done!");
}
