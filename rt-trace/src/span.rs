use crate::{enabled, span_queue::SpanQueue};
use fastant::Instant;
use std::{cell::RefCell, rc::Rc, str::FromStr};

#[derive(Debug)]
pub struct RunTask {}
#[derive(Debug)]
pub struct RuntimeStart {}
#[derive(Debug)]
pub struct RuntimeTerminate {}
#[derive(Debug)]
pub struct ThreadDiscriptor {}
#[derive(Debug)]
pub struct ProcessDiscriptor {}

impl Type {
    /// Return `str` representation of this type.
    pub fn type_name_str(&self) -> &'static str {
        match self {
            &Type::RunTask(_) => "run_task",
            &Type::RuntimeStart(_) => "runtime_start",
            &Type::RuntimeTarminate(_) => "runtime_terminate",
            &Type::ThreadDiscriptor(_) => "thread_discriptor",
            &Type::ProcessDiscriptor(_) => "process_discriptor",
        }
    }

    /// Return `String` representation of this type.
    ///
    /// TODO: avoid string allocation for this.
    pub fn type_name_string(&self) -> String {
        String::from_str(self.type_name_str()).unwrap()
    }
}

#[derive(Debug)]
pub enum Type {
    RunTask(RunTask),
    RuntimeStart(RuntimeStart),
    RuntimeTarminate(RuntimeTerminate),
    // perfetto specific
    ThreadDiscriptor(ThreadDiscriptor),
    ProcessDiscriptor(ProcessDiscriptor),
}

#[derive(Debug)]
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
    #[inline]
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
