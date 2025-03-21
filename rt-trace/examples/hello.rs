use rt_trace::{config::Config, consumer::SpanConsumer, flush, initialize, span, span::RunTask};

fn main() {
    single_thread();
    // multi_thread();
}

fn single_thread() {
    struct DummyReporter;

    impl SpanConsumer for DummyReporter {
        fn consume(&mut self, spans: &[rt_trace::span::RawSpan]) {
            println!("spans: {:?}", spans);
        }
    }

    initialize(Config {}, DummyReporter {});

    let jh = std::thread::spawn(|| {
        // Start tracing
        {
            let _guard = span(span::Type::RunTask(RunTask {}), 12);
        }
        {
            let _guard = span(span::Type::RunTask(RunTask {}), 12);
        }
    });

    jh.join().unwrap();

    flush();
}

fn multi_thread() {}
