use config::Config;
use consumer::{GLOBAL_SPAN_CONSUMER, SpanConsumer};
use span::{RawSpan, Span, Type};
use span_queue::with_span_queue;
use std::sync::atomic::AtomicBool;
pub mod backend;
pub(crate) mod command;
pub mod config;
pub mod consumer;
pub(crate) mod macros;
pub mod span;
pub(crate) mod span_queue;
mod utils;
use fastant::Instant;

/// Whether tracing is enabled or not.
static ENABLED: AtomicBool = AtomicBool::new(false);

#[inline]
pub fn enabled() -> bool {
    ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}

#[inline]
fn set_enabled(set: bool) {
    ENABLED.store(set, std::sync::atomic::Ordering::Relaxed);
}

/// Create a span.
#[inline]
pub fn span(typ: Type, thread_id: u64) -> Span {
    with_span_queue(|span_queue| {
        if enabled() {
            Span {
                inner: Some(RawSpan {
                    typ,
                    thread_id,
                    start: Instant::now(),
                    end: Instant::ZERO,
                }),
                span_queue_handle: span_queue.clone(),
            }
        } else {
            Span {
                inner: None,
                span_queue_handle: span_queue.clone(),
            }
        }
    })
}

/// Stop tracing.
///
/// This function flushes spans that the consumer thread has, but doesn't against the
/// spans that is owned by worker threads.
#[inline]
pub fn stop() {
    set_enabled(false);
}

/// Start tracing. Before calling this, you have to call `initialize` first.
#[inline]
pub fn start() {
    // TODO: check if `initialize` has been called.
    set_enabled(true)
}

/// Initialize tracing.
#[inline]
pub fn initialize(_config: Config, consumer: impl SpanConsumer) {
    // spawn consumer thread

    let mut global_consumer = GLOBAL_SPAN_CONSUMER.lock().unwrap();
    global_consumer.consumer = Some(Box::new(consumer));
    drop(global_consumer);

    std::thread::Builder::new()
        .name("global-span-consumer".to_string())
        .spawn(move || {
            let mut vec = vec![];
            loop {
                let mut global_consumer = GLOBAL_SPAN_CONSUMER.lock().unwrap();
                global_consumer.handle_commands(&mut vec);
                drop(global_consumer);

                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        })
        .unwrap();
}
