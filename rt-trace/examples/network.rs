fn main() {
    use std::net::SocketAddr;

    use fastrace::collector::Config;
    use fastrace::prelude::*;

    // // Initialize reporter
    // let reporter =
    //     fastrace_jaeger::JaegerReporter::new("127.0.0.1:6831".parse().unwrap(), "asynchronous")
    //         .unwrap();
    // fastrace::set_reporter(reporter, Config::default());

    // {
    //     // Start tracing
    //     let root = Span::root("root", SpanContext::random());
    // }

    // fastrace::flush();
}
