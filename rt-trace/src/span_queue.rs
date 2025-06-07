use once_cell::sync::Lazy;
use parking_lot::Mutex;

use crate::{
    backend::perfetto::thread_descriptor, command::Command, consumer::send_command, span::RawSpan,
    thread_id::get,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

pub(crate) const DEFAULT_BATCH_SIZE: usize = 16384 / 16;

thread_local! {
    static SPAN_QUEUE: Rc<RefCell<SpanQueue>> = {
        let mut queue = SpanQueue::new();

        // perfetto specific operation.
        // TODO: Can we put this logic elsewhere?
        queue.push(thread_descriptor());

        Rc::new(RefCell::new(queue))
    };
}

pub(crate) static SPAN_QUEUE_STORE: Lazy<SpanQueueStore> = Lazy::new(|| {
    let mut store = SpanQueueStore::new();
    for _ in 0..16 {
        store.register();
    }
    store
});

pub(crate) struct SpanQueueStore {
    span_queues: Vec<Arc<Mutex<SpanQueue>>>,
}

impl SpanQueueStore {
    pub(crate) fn get(&self, index: usize) -> Arc<Mutex<SpanQueue>> {
        let index = index % 16; // self.span_queues.len();
        self.span_queues.get(index).unwrap().clone()
    }

    pub(crate) fn register(&mut self) {
        let mut queue = SpanQueue::new();
        queue.push(thread_descriptor());
        self.span_queues.push(Arc::new(Mutex::new(queue)));
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
            // flush spans
            let spans = std::mem::replace(&mut self.spans, Vec::with_capacity(DEFAULT_BATCH_SIZE));
            send_command(Command::SendSpans(spans));
        }
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

#[inline]
pub(crate) fn with_span_queue<R>(f: impl FnOnce(Arc<Mutex<SpanQueue>>) -> R) -> R {
    let span_queue = SPAN_QUEUE_STORE.get(get());
    f(span_queue)
}
