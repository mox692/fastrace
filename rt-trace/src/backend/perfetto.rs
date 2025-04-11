// Reference
// * https://perfetto.dev/docs/reference/synthetic-track-event

use super::perfetto_protos;
use super::perfetto_protos::DebugAnnotation;
use super::perfetto_protos::debug_annotation;
use super::perfetto_protos::debug_annotation::Value;
use crate::Type;
use crate::consumer::SpanConsumer;
use crate::span::ProcessDiscriptor;
use crate::span::RawSpan;
use crate::span::ThreadDiscriptor;
use crate::utils::object_pool::Pool;
use crate::utils::object_pool::Puller;
use crate::utils::object_pool::Reusable;
use bytes::BytesMut;
use core::cell::RefCell;
use fastant::Anchor;
use fastant::Instant;
use once_cell::sync::Lazy;
use perfetto_protos::{
    ProcessDescriptor, ThreadDescriptor, TracePacket, TrackDescriptor, TrackEvent,
    trace_packet::{Data, OptionalTrustedPacketSequenceId, OptionalTrustedUid},
    track_descriptor::StaticOrDynamicName,
    track_event::{self, NameField},
};
use prost::Message;
use std::{fs::File, io::Write, path::Path};

static TRACE_PACKETS_POOL: Lazy<Pool<Vec<TracePacket>>> =
    Lazy::new(|| Pool::new(Vec::new, Vec::clear));

thread_local! {
    static TRACE_PACKETS_PULLER: RefCell<Puller<'static, Vec<TracePacket>>> = RefCell::new(TRACE_PACKETS_POOL.puller(2));
}

pub type TracePackets = Reusable<'static, Vec<TracePacket>>;

impl Default for TracePackets {
    fn default() -> Self {
        TRACE_PACKETS_PULLER
            .try_with(|puller| puller.borrow_mut().pull())
            .unwrap_or_else(|_| Reusable::new(&*TRACE_PACKETS_POOL, vec![]))
    }
}

/// Reporter implementation for Perfetto tracing.
pub struct PerfettoReporter {
    pid: i32,
    output: File,
}

impl PerfettoReporter {
    #[inline]
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            pid: std::process::id() as i32,
            output: File::create(path.as_ref()).expect("Failed to create output file"),
        }
    }
}

/// Docs: https://perfetto.dev/docs/reference/trace-packet-proto#DebugAnnotation
#[inline]
fn create_debug_annotations() -> Vec<DebugAnnotation> {
    // TODO: use object pool to reduce the number of allocations.
    let mut debug_annotation = DebugAnnotation::default();
    let name_field = debug_annotation::NameField::Name("key1".to_string());
    let value = Value::StringValue("value1".to_string());
    debug_annotation.name_field = Some(name_field);
    debug_annotation.value = Some(value);

    // TODO: avoid allocation
    vec![debug_annotation]
}

/// Docs: https://perfetto.dev/docs/reference/trace-packet-proto#TrackEvent
#[inline]
fn create_track_event(
    name: Option<String>,
    track_uuid: u64,
    event_type: Option<track_event::Type>,
    debug_annotations: Vec<DebugAnnotation>,
) -> TrackEvent {
    TrackEvent {
        track_uuid: Some(track_uuid),
        name_field: name.map(NameField::Name),
        r#type: event_type.map(|typ| typ.into()),
        debug_annotations,
        ..Default::default()
    }
}

/// Docs: https://perfetto.dev/docs/reference/trace-packet-proto#ProcessDescriptor
#[inline]
fn create_process_descriptor(pid: i32) -> ProcessDescriptor {
    ProcessDescriptor {
        pid: Some(pid),
        ..Default::default()
    }
}

/// Docs https://perfetto.dev/docs/reference/trace-packet-proto#TrackDescriptor
#[inline]
fn create_track_descriptor(
    uuid: u64,
    name: Option<String>,
    process: Option<ProcessDescriptor>,
    thread: Option<ThreadDescriptor>,
) -> TrackDescriptor {
    TrackDescriptor {
        uuid: Some(uuid),
        static_or_dynamic_name: name.map(StaticOrDynamicName::Name),
        process,
        thread,
        ..Default::default()
    }
}

#[inline]
fn create_thread_descriptor(pid: i32, thread_id: usize, thread_name: String) -> ThreadDescriptor {
    ThreadDescriptor {
        pid: Some(pid),
        tid: Some(thread_id as i32),
        thread_name: Some(thread_name),
        ..Default::default()
    }
}
/// Appends a thread descriptor packet to the trace if not already sent.
fn append_thread_descriptor(
    trace: &mut Trace,
    thread_info: &crate::span::ThreadDiscriptor,
    pid: i32,
    track_uuid: u64,
) {
    // TODO: avoid string allocation
    // TODO: get_or_insert thread name to TLS.
    let thread_name = thread_info.thread_name.clone();
    let thread_descriptor = create_thread_descriptor(pid, track_uuid as usize, thread_name.clone());
    let track_descriptor = create_track_descriptor(
        track_uuid,
        // TODO: avoid allocation
        Some(thread_name),
        Some(create_process_descriptor(pid)),
        Some(thread_descriptor),
    );

    let packet = TracePacket {
        data: Some(Data::TrackDescriptor(track_descriptor)),
        optional_trusted_uid: Some(OptionalTrustedUid::TrustedUid(42)),
        ..Default::default()
    };

    // Insert the packet at the beginning if needed
    trace.insert(0, packet);
}

fn append_process_descriptor(trace: &mut Trace, pid: i32, track_uuid: u64) {
    let process_descriptor = create_process_descriptor(pid);
    let track_descriptor =
        create_track_descriptor(track_uuid, None, Some(process_descriptor), None);
    let packet = TracePacket {
        data: Some(Data::TrackDescriptor(track_descriptor)),
        optional_trusted_uid: Some(OptionalTrustedUid::TrustedUid(42)),
        ..Default::default()
    };
    // Insert the packet at the beginning
    trace.insert(0, packet);
}
struct Trace {
    pub(self) inner: TracePackets,
}

impl Trace {
    #[inline]
    fn new() -> Self {
        Self {
            inner: TracePackets::default(),
        }
    }

    #[inline]
    fn push(&mut self, packet: TracePacket) {
        self.inner.push(packet);
    }

    #[inline]
    fn insert(&mut self, index: usize, packet: TracePacket) {
        self.inner.insert(index, packet);
    }

    #[inline]
    fn write(&mut self, output: &mut File) {
        // The next pooled object will be temporarily assigned to `self.inner` to avoid borrowing issues.
        let next = TracePackets::default();
        let current = std::mem::replace(&mut self.inner, next);

        let packet = current.into_inner();
        let trace = perfetto_protos::Trace { packet };
        // TODO: use pool
        let mut buf = BytesMut::with_capacity(64);
        trace.encode(&mut buf).unwrap();
        output.write_all(&buf).unwrap();
        output.flush().unwrap();

        // The original `TracePackets` is now stored in `self.inner`, and the temporary pooled object
        // will be dropped (injected to the pool again).
        self.inner = Reusable::new(&*TRACE_PACKETS_POOL, trace.packet);
    }
}

impl SpanConsumer for PerfettoReporter {
    fn consume(&mut self, spans: &[RawSpan]) {
        let mut trace = Trace::new();

        let pid = self.pid;
        // TODO: move to elsewhere?
        let anchor = Anchor::new();

        for span in spans {
            match &span.typ {
                Type::ProcessDiscriptor(_) => {
                    append_process_descriptor(&mut trace, pid, span.thread_id);
                }
                Type::ThreadDiscriptor(d) => {
                    append_thread_descriptor(&mut trace, d, self.pid, span.thread_id);
                }
                Type::RunTask(_) => {
                    // Start event packet
                    let debug_annotations = create_debug_annotations();
                    let start_event = create_track_event(
                        Some(span.typ.type_name_string()),
                        span.thread_id,
                        Some(track_event::Type::SliceBegin),
                        debug_annotations,
                    );
                    let start_packet = TracePacket {
                        data: Some(Data::TrackEvent(start_event)),
                        trusted_pid: Some(pid),
                        timestamp: Some(span.start.as_unix_nanos(&anchor)),
                        optional_trusted_packet_sequence_id: Some(
                            OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(42),
                        ),
                        ..Default::default()
                    };

                    trace.push(start_packet);

                    // End event packet
                    let debug_annotations = create_debug_annotations();
                    let end_event = create_track_event(
                        None,
                        span.thread_id,
                        Some(track_event::Type::SliceEnd),
                        debug_annotations,
                    );
                    let end_packet = TracePacket {
                        data: Some(Data::TrackEvent(end_event)),
                        trusted_pid: Some(pid),
                        timestamp: Some(span.end.as_unix_nanos(&anchor)),
                        optional_trusted_packet_sequence_id: Some(
                            OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(42),
                        ),
                        ..Default::default()
                    };
                    trace.push(end_packet);
                }
                Type::RuntimeStart(_) => {
                    unimplemented!()
                }
                Type::RuntimeTarminate(_) => {
                    unimplemented!()
                }
            };
        }

        trace.write(&mut self.output);
    }
}

/// This is called when a SpanQueue at local storage gets initialized.
pub(crate) fn thread_descriptor() -> RawSpan {
    let thread_id = crate::utils::thread_id::get() as u64;
    RawSpan {
        typ: Type::ThreadDiscriptor(ThreadDiscriptor {
            thread_name: std::thread::current()
                .name()
                .map(|str| str.into())
                .unwrap_or(format!("{thread_id}")),
        }),
        thread_id: crate::utils::thread_id::get() as u64,
        start: Instant::ZERO,
        end: Instant::ZERO,
    }
}

/// This is called when a SpanQueue at local storage gets initialized.
pub(crate) fn process_descriptor() -> RawSpan {
    RawSpan {
        typ: Type::ProcessDiscriptor(ProcessDiscriptor {}),
        thread_id: crate::utils::thread_id::get() as u64,
        start: Instant::ZERO,
        end: Instant::ZERO,
    }
}
