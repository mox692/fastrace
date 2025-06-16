# fastrace-perfetto

* transport fastrace log over network
  * with serialize perfetto format
  * witt text format
* output local file directory


### Concrete examples

**basic usage**
```rust
let consumer = PerfettoReporter::new("./single.log");

initialize(Config::default(), consumer);

/// At this point, `rt_trace::span` actually starts emitting logs
start();

let _guard = rt_trace::span(span::Type::RunTask(RunTask {
    name: Some("task1".to_string()),
    ..Default::default()
}));

if let Err(e) = some_func() {
  /// Only when this `flush` gets called, the trace log gonna be written
  /// in the file.
  flush();

  return
}

/// If you don't call `flush`, the trace result never gonna be written in
/// the file.
```


If you want to store as many logs as possible, you could do something like this:
```rust
let consumer = PerfettoReporter::new("./single.log");

initialize(Config::default(), consumer);

start();

/// Spawn a background thread that canstantly captures the trace logs.
std::thread::spawn(|| {
  loop {
    sleep(10)

    flush()
  }
});

some_work();

```

Of course, even though `rt-trace` is intended for continuous tracing purposes, calling `flush()` frequently could cause perf impact.
