use crate::{enabled, span_queue::SpanQueue};
use fastant::Instant;
use std::{cell::RefCell, rc::Rc};

pub struct RunTask {}
pub struct RuntimeStart {}
pub struct RuntimeTerminate {}
pub struct ThreadDiscriptor {}
pub struct ProcessDiscriptor {}

fn type_name(typ: Type) -> &'static str {
    match typ {
        Type::RunTask(_) => "run_task",
        Type::RuntimeStart(_) => "runtime_start",
        Type::RuntimeTarminate(_) => "runtime_terminate",
        Type::ThreadDiscriptor(_) => "thread_discriptor",
        Type::ProcessDiscriptor(_) => "process_discriptor",
    }
}

pub enum Type {
    RunTask(RunTask),
    RuntimeStart(RuntimeStart),
    RuntimeTarminate(RuntimeTerminate),
    // perfetto specific
    ThreadDiscriptor(ThreadDiscriptor),
    ProcessDiscriptor(ProcessDiscriptor),
}

pub struct RawSpan {
    pub(crate) typ: Type,
    pub(crate) thread_id: u64,
    pub(crate) start: Instant,
    pub(crate) end: Instant,
}

/// A span that. This should be dropped in the same therad.
pub struct Span {
    pub(crate) inner: Option<RawSpan>,
    pub(crate) span_queue_handle: Rc<RefCell<SpanQueue>>,
}

impl Drop for Span {
    fn drop(&mut self) {
        if !enabled() {
            return;
        }

        let Some(mut span) = self.inner.take() else {
            return;
        };
        span.end = Instant::now();
        self.span_queue_handle.borrow_mut().push(span);
    }
}
