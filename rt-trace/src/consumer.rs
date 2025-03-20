use crate::{
    backend::perfetto::process_descriptor,
    command::Command,
    span::RawSpan,
    utils::spsc::{Receiver, Sender, bounded},
};
use std::{
    cell::UnsafeCell,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

/// TODO: Do we need Send + 'static bound?
pub trait SpanConsumer: Send + 'static {
    /// TODO: Can spans be abstracted?
    fn consume(&mut self, spans: Vec<RawSpan>);
}

static SPSC_RXS: Mutex<Vec<Receiver<Command>>> = Mutex::new(Vec::new());

pub(crate) fn register_receiver(rx: Receiver<Command>) {
    SPSC_RXS.lock().unwrap().push(rx);
}

pub(crate) fn send_command(cmd: Command) {
    COMMAND_SENDER
        .try_with(|sender| unsafe { (*sender.get()).send(cmd).ok() })
        .ok();
}

thread_local! {
    static COMMAND_SENDER: UnsafeCell<Sender<Command>> = {
        let (tx, rx) = bounded(10240);
        register_receiver(rx);
        UnsafeCell::new(tx)
    };
}

/// Mainly used for perfetto tracing, where we need to publish process descriptor first.
static FLUSHED_ONCE: AtomicBool = AtomicBool::new(false);

fn flushed_once() -> bool {
    FLUSHED_ONCE.load(Ordering::Relaxed)
}
fn set_flushed_once(set: bool) {
    FLUSHED_ONCE.store(set, Ordering::Relaxed);
}

pub(crate) static GLOBAL_SPAN_CONSUMER: Mutex<GlobalSpanConsumer> =
    Mutex::new(GlobalSpanConsumer::new());

pub(crate) struct GlobalSpanConsumer {
    pub(crate) consumer: Option<Box<dyn SpanConsumer>>,
}

impl GlobalSpanConsumer {
    const fn new() -> Self {
        Self { consumer: None }
    }

    pub(crate) fn handle_commands(&mut self) {
        let mut guard = SPSC_RXS.lock().unwrap();
        let rxs: Vec<Receiver<Command>> = guard.drain(..).collect();
        drop(guard);

        let mut spans: Vec<RawSpan> = vec![];

        // Required for perfetto tracing.
        // TODO: Can we put this logic elsewhere?
        if !flushed_once() {
            spans.push(process_descriptor());
            set_flushed_once(true);
        }

        for mut rx in rxs {
            while let Ok(Some(Command::SendSpans(span))) = rx.try_recv() {
                spans.extend(span);
            }
        }

        let Some(consumer) = &mut self.consumer else {
            panic!("Consumer should be set");
        };

        consumer.as_mut().consume(spans);
    }
}
