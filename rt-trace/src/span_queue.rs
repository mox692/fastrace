use once_cell::sync::Lazy;
use parking_lot::Mutex;

use crate::{command::Command, consumer::send_command, span::RawSpan, thread_id::get, SHARD_NUM};
use std::{cell::Cell, collections::HashMap, sync::Arc};

pub(crate) const DEFAULT_BATCH_SIZE: usize = 16384 / 16;

thread_local! {
    pub(crate) static THREAD_INITIALIZED: Cell<bool> = Cell::new(false);
}

pub(crate) static SPAN_QUEUE_STORE: Lazy<SpanQueueStore> = Lazy::new(|| {
    let mut store = SpanQueueStore::new();
    let num_shards = SHARD_NUM.load(std::sync::atomic::Ordering::Relaxed);
    for _ in 0..num_shards {
        store.register();
    }
    store
});

pub(crate) static DESCRIPTORS: Lazy<Arc<Mutex<HashMap<RawSpan, bool>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub(crate) fn add_descriptor(span: RawSpan) {
    let mut guard = DESCRIPTORS.lock();

    if guard.get(&span).is_some() {
        panic!("Descriptor already exists for span: {:?}", span);
    }

    guard.insert(span, false);
}
pub(crate) fn drain_descriptors() -> Vec<RawSpan> {
    let mut descriptors = DESCRIPTORS.lock();

    descriptors
        .iter_mut()
        .filter_map(|(span, flushed)| {
            if !*flushed {
                *flushed = true;
                Some(span.clone())
            } else {
                None
            }
        })
        .collect()
}

pub(crate) struct SpanQueueStore {
    span_queues: Vec<Arc<Mutex<SpanQueue>>>,
}

impl SpanQueueStore {
    pub(crate) fn get(&self, index: usize) -> &Arc<Mutex<SpanQueue>> {
        let index = index % SHARD_NUM.load(std::sync::atomic::Ordering::Relaxed);
        &self.span_queues.get(index).unwrap()
    }

    pub(crate) fn register(&mut self) {
        let queue = SpanQueue::new();
        self.span_queues.push(Arc::new(Mutex::new(queue)));
    }

    pub(crate) fn len(&self) -> usize {
        self.span_queues.len()
    }
}

impl SpanQueueStore {
    fn new() -> SpanQueueStore {
        SpanQueueStore {
            span_queues: Vec::new(),
        }
    }
}
/// Each thread has their own `LocalSpans` in TLS.
#[derive(Debug)]
pub(crate) struct SpanQueue {
    spans: Vec<RawSpan>,
}

impl SpanQueue {
    #[inline]
    fn new() -> Self {
        Self {
            spans: Vec::with_capacity(DEFAULT_BATCH_SIZE),
        }
    }

    #[inline]
    pub(crate) fn push(&mut self, span: RawSpan) {
        self.spans.push(span);
        if self.spans.len() == DEFAULT_BATCH_SIZE {
            self.flush();
        }
    }

    #[inline]
    pub(crate) fn flush(&mut self) {
        // flush spans
        let spans = std::mem::replace(&mut self.spans, Vec::with_capacity(DEFAULT_BATCH_SIZE));
        send_command(Command::SendSpans(spans));
    }
}

impl Drop for SpanQueue {
    // When SpanQueue is used as a thread local value, then this drop gets called
    // at the time when this thread is terminated, making sure all spans would not
    // be lossed.
    fn drop(&mut self) {
        let spans = std::mem::take(&mut self.spans);
        send_command(Command::SendSpans(spans));
    }
}

impl Drop for SpanQueueStore {
    // When SpanQueue is used as a thread local value, then this drop gets called
    // at the time when this thread is terminated, making sure all spans would not
    // be lost.
    fn drop(&mut self) {
        let len = SPAN_QUEUE_STORE.len();
        for index in 0..len {
            SPAN_QUEUE_STORE.get(index).lock().flush();
        }
    }
}

#[inline]
pub(crate) fn with_span_queue<R>(f: impl FnOnce(&Arc<Mutex<SpanQueue>>) -> R) -> R {
    let thread_id = get();
    let span_queue = SPAN_QUEUE_STORE.get(thread_id);
    f(span_queue)
}
