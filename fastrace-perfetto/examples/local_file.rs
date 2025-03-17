use fastrace_perfetto::{enter_with_local_parent_with_thread_id, root_with_thread_id};

fn main() {
    // single_thread();
    multi_thread();
}

fn single_thread() {
    use fastrace::collector::Config;
    use fastrace::prelude::*;

    let reporter = fastrace_perfetto::PerfettoReporter::new("./single.log");

    fastrace::set_reporter(reporter, Config::default());

    {
        // Start tracing
        let root = root_with_thread_id("root", SpanContext::random());
        let _g1 = root.set_local_parent();
        {
            let _g2 = enter_with_local_parent_with_thread_id("child1");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        {
            let _g2 = enter_with_local_parent_with_thread_id("child2");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    fastrace::flush();
}

fn multi_thread() {
    use fastrace::collector::Config;
    use fastrace::prelude::*;

    let reporter = fastrace_perfetto::PerfettoReporter::new("./multi.log");

    fastrace::set_reporter(reporter, Config::default());

    let jh = std::thread::spawn(|| {
        // Start tracing
        let root = root_with_thread_id("root", SpanContext::random());
        let _g1 = root.set_local_parent();
        {
            let _g2 = enter_with_local_parent_with_thread_id("child1");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        {
            let _g2 = enter_with_local_parent_with_thread_id("child2");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });

    // Start tracing
    {
        let root = root_with_thread_id("root", SpanContext::random());
        let _g1 = root.set_local_parent();
        {
            let _g2 = enter_with_local_parent_with_thread_id("child1");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        {
            let _g2 = enter_with_local_parent_with_thread_id("child2");
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
    fastrace::flush();

    jh.join().unwrap();
}
