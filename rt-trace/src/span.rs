use crate::{enabled, span_queue::SpanQueue};
use fastant::Instant;
use parking_lot::Mutex;
use std::{str::FromStr, sync::Arc};

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct RunTask {
    pub id: Option<u64>,
    // TODO: define more efficient string type.
    pub name: Option<String>,
    pub file: Option<&'static str>,
    pub line: Option<u32>,
    pub col: Option<u32>,
    // TODO: define more efficient string type.
    pub backtrace: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeStart {}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeTerminate {}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimePark {}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThreadDiscriptor {
    pub(crate) thread_name: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProcessDiscriptor {}

impl Type {
    /// Return `str` representation of this type.
    #[inline]
    pub fn type_name_str(&self) -> &'static str {
        match self {
            &Type::RunTask(_) => "run_task",
            &Type::RuntimeStart(_) => "runtime_start",
            &Type::RuntimeTerminate(_) => "runtime_terminate",
            &Type::RuntimePark(_) => "runtime_park",
            &Type::ThreadDiscriptor(_) => "thread_discriptor",
            &Type::ProcessDiscriptor(_) => "process_discriptor",
        }
    }

    /// Return `String` representation of this type.
    ///
    /// TODO: avoid string allocation for this.
    #[inline]
    pub fn type_name_string(&self) -> String {
        String::from_str(self.type_name_str()).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    RunTask(RunTask),
    RuntimeStart(RuntimeStart),
    RuntimeTerminate(RuntimeTerminate),
    RuntimePark(RuntimePark),
    // perfetto specific
    ThreadDiscriptor(ThreadDiscriptor),
    ProcessDiscriptor(ProcessDiscriptor),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RawSpan {
    pub(crate) typ: Type,
    pub(crate) thread_id: u64,
    pub(crate) start: Instant,
    pub(crate) end: Instant,
}

/// A span that. This should be dropped in the same thread.
#[derive(Debug)]
pub struct Span {
    pub(crate) inner: Option<RawSpan>,
    pub(crate) span_queue_handle: Arc<Mutex<SpanQueue>>,
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

        self.span_queue_handle.lock().push(span);
    }
}
