// Reference
// * https://perfetto.dev/docs/reference/synthetic-track-event

use super::perfetto_protos;
use super::perfetto_protos::debug_annotation;
use super::perfetto_protos::debug_annotation::Value;
use super::perfetto_protos::DebugAnnotation;
use crate::consumer::SpanConsumer;
use crate::span::ProcessDiscriptor;
use crate::span::RawSpan;
use crate::span::ThreadDiscriptor;
use crate::Type;
use bytes::BytesMut;
use fastant::Anchor;
use fastant::Instant;
use perfetto_protos::{
    trace_packet::{Data, OptionalTrustedPacketSequenceId, OptionalTrustedUid},
    track_descriptor::StaticOrDynamicName,
    track_event::{self, NameField},
    ProcessDescriptor, ThreadDescriptor, Trace, TracePacket, TrackDescriptor, TrackEvent,
};
use prost::Message;
use std::{
    collections::HashSet,
    fs::File,
    io::Write,
    path::Path,
    sync::{OnceLock, RwLock},
};

static INITIALIZED_SET: OnceLock<RwLock<HashSet<u64>>> = OnceLock::new();

/// Reporter implementation for Perfetto tracing.
pub struct PerfettoReporter {
    output: File,
}

impl PerfettoReporter {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            output: File::create(path.as_ref()).expect("Failed to create output file"),
        }
    }
}

/// Docs: https://perfetto.dev/docs/reference/trace-packet-proto#DebugAnnotation
/// TODO: use object pool to reduce the number of allocations.
fn create_debug_annotations() -> Vec<DebugAnnotation> {
    let mut debug_annotation = DebugAnnotation::default();
    let name_field = debug_annotation::NameField::Name("key1".to_string());
    let value = Value::StringValue("value1".to_string());
    debug_annotation.name_field = Some(name_field);
    debug_annotation.value = Some(value);

    // TODO: avoid allocation
    vec![debug_annotation]
}

/// Docs: https://perfetto.dev/docs/reference/trace-packet-proto#TrackEvent
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
fn create_process_descriptor(pid: i32) -> ProcessDescriptor {
    ProcessDescriptor {
        pid: Some(pid),
        ..Default::default()
    }
}

/// Docs https://perfetto.dev/docs/reference/trace-packet-proto#TrackDescriptor
fn create_track_descriptor(
    uuid: u64,
    name: Option<impl AsRef<str>>,
    process: Option<ProcessDescriptor>,
    thread: Option<ThreadDescriptor>,
) -> TrackDescriptor {
    TrackDescriptor {
        uuid: Some(uuid),
        static_or_dynamic_name: name.map(|s| StaticOrDynamicName::Name(s.as_ref().to_string())),
        process,
        thread,
        ..Default::default()
    }
}

fn create_thread_descriptor(pid: i32, thread_id: usize, thread_name: String) -> ThreadDescriptor {
    ThreadDescriptor {
        pid: Some(pid),
        tid: Some(thread_id as i32),
        thread_name: Some(thread_name),
        ..Default::default()
    }
}

fn set_descriptor_initialized(track_uuid: u64) {
    let lock = INITIALIZED_SET.get_or_init(|| RwLock::new(HashSet::new()));
    lock.write().unwrap().insert(track_uuid);
}

/// Appends a thread descriptor packet to the trace if not already sent.
fn append_thread_descriptor(trace: &mut Trace, track_uuid: u64) {
    set_descriptor_initialized(track_uuid);

    // TODO: avoid syscall
    let pid = std::process::id() as i32;

    let thread_descriptor =
        create_thread_descriptor(pid, track_uuid as usize, "thread_name".to_string());
    let track_descriptor = create_track_descriptor(
        track_uuid,
        Some("thread_name".to_string()),
        Some(create_process_descriptor(pid)),
        Some(thread_descriptor),
    );

    let packet = TracePacket {
        data: Some(Data::TrackDescriptor(track_descriptor)),
        optional_trusted_uid: Some(OptionalTrustedUid::TrustedUid(42)),
        ..Default::default()
    };

    // Insert the packet at the beginning if needed
    trace.packet.insert(0, packet);
}

fn append_process_descriptor(trace: &mut Trace, track_uuid: u64) {
    let pid = std::process::id() as i32;
    let process_descriptor = create_process_descriptor(pid);
    let track_descriptor =
        create_track_descriptor(track_uuid, None::<&str>, Some(process_descriptor), None);
    let packet = TracePacket {
        data: Some(Data::TrackDescriptor(track_descriptor)),
        optional_trusted_uid: Some(OptionalTrustedUid::TrustedUid(42)),
        ..Default::default()
    };
    // Insert the packet at the beginning
    trace.packet.insert(0, packet);
}

/// Writes the trace to the output file.
fn write_trace(trace: &Trace, output: &mut File) {
    let mut buf = BytesMut::new();
    trace.encode(&mut buf).unwrap();
    output.write_all(&buf).unwrap();
    output.flush().unwrap()
}

impl SpanConsumer for PerfettoReporter {
    fn consume(&mut self, spans: &[RawSpan]) {
        let mut trace = Trace::default();

        // TODO: avoid syscall
        let pid = std::process::id() as i32;

        // TODO: move to elsewhere.
        let anchor = Anchor::new();

        for span in spans {
            match span.typ {
                Type::ProcessDiscriptor(_) => {
                    append_process_descriptor(&mut trace, span.thread_id);
                }
                Type::ThreadDiscriptor(_) => {
                    append_thread_descriptor(&mut trace, span.thread_id);
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

                    trace.packet.push(start_packet);

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
                    trace.packet.push(end_packet);
                }
                Type::RuntimeStart(_) => {
                    unimplemented!()
                }
                Type::RuntimeTarminate(_) => {
                    unimplemented!()
                }
            };
        }

        write_trace(&trace, &mut self.output);
    }
}

/// This is called when a SpanQueue at local storage gets initialized.
pub(crate) fn thread_descriptor() -> RawSpan {
    RawSpan {
        typ: Type::ThreadDiscriptor(ThreadDiscriptor {}),
        thread_id: thread_id::get() as u64,
        start: Instant::ZERO,
        end: Instant::ZERO,
    }
}

/// This is called when a SpanQueue at local storage gets initialized.
pub(crate) fn process_descriptor() -> RawSpan {
    RawSpan {
        typ: Type::ProcessDiscriptor(ProcessDiscriptor {}),
        thread_id: thread_id::get() as u64,
        start: Instant::ZERO,
        end: Instant::ZERO,
    }
}
