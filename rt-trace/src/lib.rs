use config::Config;
use consumer::{SpanConsumer, GLOBAL_SPAN_CONSUMER};
use span::{RawSpan, Span, Type};
use std::{
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::Duration,
};
use utils::thread_id;
pub mod backend;
pub(crate) mod command;
pub mod config;
pub mod consumer;
pub mod span;
pub(crate) mod span_queue;
mod utils;
use fastant::Instant;

use crate::{
    backend::perfetto::thread_descriptor,
    span_queue::{with_span_queue, SPAN_QUEUE_STORE, THREAD_INITIALIZED},
};

#[cfg(test)]
mod tests;

/// This should be set at the initialization of the library.
pub(crate) static SHARD_NUM: AtomicUsize = AtomicUsize::new(DEFAULT_NUM_SHARD);

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
            THREAD_INITIALIZED.with(|current| {
                // Is this the first time this thread is creating a span?
                if !current.get() {
                    span_queue.lock().push(thread_descriptor());
                    current.replace(true);
                }

                Span {
                    inner: Some(RawSpan {
                        typ,
                        thread_id: thread_id::get() as u64,
                        start: Instant::now(),
                        end: Instant::ZERO,
                    }),
                    span_queue_handle: span_queue.clone(),
                }
            })
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

const DEFAULT_NUM_SHARD: usize = 16;

/// Initialize tracing.
#[inline]
pub fn initialize(config: Config, consumer: impl SpanConsumer + 'static) {
    SHARD_NUM.store(
        config.num_shard.unwrap_or(DEFAULT_NUM_SHARD),
        Ordering::Relaxed,
    );

    GLOBAL_SPAN_CONSUMER.lock().consumer = Some(Box::new(consumer));

    // spawn consumer thread
    std::thread::Builder::new()
        .name("global-span-consumer".to_string())
        .spawn(move || loop {
            std::thread::sleep(
                config
                    .consumer_thread_sleep_duration
                    .unwrap_or(Duration::from_millis(10)),
            );

            GLOBAL_SPAN_CONSUMER.lock().collect_and_push_commands();
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
    // 1. flush the local span queue.
    stop();
    let len = SPAN_QUEUE_STORE.len();
    for index in 0..len {
        // Note: Without stopping the span emitting, this lock acquisition could contend.
        SPAN_QUEUE_STORE.get(index).lock().flush();
    }
    start();

    // 2. flush the global span queue.
    let mut global_consumer = GLOBAL_SPAN_CONSUMER.lock();
    global_consumer.collect_and_push_commands();
    global_consumer.flush();
}
