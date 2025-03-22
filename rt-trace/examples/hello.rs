use rt_trace::{
    backend::perfetto::PerfettoReporter, config::Config, flush, initialize, span, span::RunTask,
    start,
};

fn main() {
    single_thread();
    // multi_thread();
}

fn single_thread() {
    let consumer = PerfettoReporter::new("./single.log");

    initialize(Config {}, consumer);

    start();

    let jh = std::thread::spawn(|| {
        // Start tracing
        {
            let _guard = span(span::Type::RunTask(RunTask {}), thread_id::get() as u64);
        }
        {
            let _guard = span(span::Type::RunTask(RunTask {}), thread_id::get() as u64);
        }
    });

    jh.join().unwrap();

    flush();
}

fn multi_thread() {}
