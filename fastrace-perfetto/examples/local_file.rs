fn main() {
    use fastrace::collector::Config;
    use fastrace::prelude::*;

    let reporter = fastrace_perfetto::PerfettorReporter::new("./res.log");

    fastrace::set_reporter(reporter, Config::default());

    {
        // Start tracing
        let root = Span::root("root", SpanContext::random());
        let _g1 = root.set_local_parent();
        {
            let _g2 = LocalSpan::enter_with_local_parent("child1");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        {
            let _g2 = LocalSpan::enter_with_local_parent("child2");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    fastrace::flush();
}
