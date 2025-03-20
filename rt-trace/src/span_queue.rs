use crate::{
    backend::perfetto::thread_descriptor, command::Command, consumer::send_command, span::RawSpan,
};
use std::{cell::RefCell, rc::Rc};

const DEFAULT_BATCH_SIZE: usize = 16384;

thread_local! {
    static SPAN_QUEUE: Rc<RefCell<SpanQueue>> = {
        let mut queue = SpanQueue::new();

        // perfetto specific operation.
        // TODO: Can we put this logic elsewhere?
        queue.push(thread_descriptor());

        Rc::new(RefCell::new(queue))
    };
}

/// Each thread has their own `LocalSpans` in TLS.
pub(crate) struct SpanQueue {
    spans: Vec<RawSpan>,
}

impl SpanQueue {
    fn new() -> Self {
        Self {
            spans: Vec::with_capacity(DEFAULT_BATCH_SIZE),
        }
    }

    pub(crate) fn push(&mut self, span: RawSpan) {
        if self.spans.len() == DEFAULT_BATCH_SIZE - 1 {
            // flush spans
            let spans: Vec<RawSpan> = self.drain().collect();
            send_command(Command::SendSpans(spans));
            return;
        }
        self.spans.push(span);
    }

    /// Called from the span consumer
    fn drain(&mut self) -> impl Iterator<Item = RawSpan> + '_ {
        self.spans.drain(..)
    }
}

impl Drop for SpanQueue {
    /// When SpanQueue is used as a thread local value, then this drop gets called
    /// at the time when this thread is terminated, making sure all spans would not
    /// be lossed.
    fn drop(&mut self) {
        let command = Command::SendSpans(self.drain().collect());
        send_command(command);
    }
}

pub(crate) fn with_span_queue<R>(f: impl FnOnce(&Rc<RefCell<SpanQueue>>) -> R) -> R {
    SPAN_QUEUE.with(|queue| f(queue))
}
