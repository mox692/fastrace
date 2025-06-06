use config::Config;
use consumer::{SpanConsumer, GLOBAL_SPAN_CONSUMER};
use span::{RawSpan, Span, Type};
use span_queue::with_span_queue;
use std::{sync::atomic::AtomicBool, time::Duration};
use utils::thread_id;
pub mod backend;
pub(crate) mod command;
pub mod config;
pub mod consumer;
pub mod span;
pub(crate) mod span_queue;
mod utils;
use fastant::Instant;

#[cfg(test)]
mod tests;

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
pub fn span(typ: Type) -> Span {
    with_span_queue(|span_queue| {
        if enabled() {
            Span {
                inner: Some(RawSpan {
                    typ,
                    thread_id: thread_id::get() as u64,
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
pub fn initialize(config: Config, consumer: impl SpanConsumer + 'static) {
    GLOBAL_SPAN_CONSUMER.lock().unwrap().consumer = Some(Box::new(consumer));

    // spawn consumer thread
    std::thread::Builder::new()
        .name("global-span-consumer".to_string())
        .spawn(move || loop {
            std::thread::sleep(
                config
                    .consumer_thread_sleep_duration
                    .unwrap_or(Duration::from_millis(10)),
            );

            GLOBAL_SPAN_CONSUMER.lock().unwrap().handle_commands();
        })
        .unwrap();
}

/// Flush all spans currently held by the consumer thread.
///
/// This function should be called at the end of the program to ensure no spans
/// held by the consumer thread are missed. Note that this function does not
/// flush spans held in `SpanQueue`. You must drop the `SpanQueue` for each
/// thread and collect the spans into the consumer thread *before* calling this function.
#[inline]
pub fn flush() {
    let handle = std::thread::spawn(|| {
        let mut global_consumer = GLOBAL_SPAN_CONSUMER.lock().unwrap();
        global_consumer.handle_commands();
    });

    handle.join().unwrap()
}
