// Reference
// * https://perfetto.dev/docs/reference/synthetic-track-event

use bytes::BytesMut;
use fastrace::collector::Reporter;
use fastrace::prelude::*;
use prost::Message;
use std::{
    fs::File,
    io::Write,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
pub mod config;
mod perfetto_protos;
use perfetto_protos::{
    trace_packet::{Data, OptionalTrustedPacketSequenceId, OptionalTrustedUid},
    track_descriptor::StaticOrDynamicName,
    track_event::{self, NameField},
    ProcessDescriptor, ThreadDescriptor, Trace, TracePacket, TrackDescriptor, TrackEvent,
};

thread_local! {
    /// Unique identifier for the track associated with the current thread.
    static TRACK_UUID: u64 = generate_track_uuid();
    /// Indicator whether the thread descriptor has been sent.
    static THREAD_DESCRIPTOR_SENT: AtomicBool = AtomicBool::new(false);
}

/// Static indicator whether the process descriptor has been sent.
static PROCESS_DESCRIPTOR_SENT: AtomicBool = AtomicBool::new(false);

fn generate_track_uuid() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos() as u64
}

fn get_track_uuid() -> u64 {
    TRACK_UUID.with(|id| *id)
}
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

/// Docs: https://perfetto.dev/docs/reference/trace-packet-proto#TrackEvent
fn create_track_event(
    name: Option<String>,
    track_uuid: u64,
    event_type: Option<track_event::Type>,
) -> TrackEvent {
    TrackEvent {
        track_uuid: Some(track_uuid),
        name_field: name.map(NameField::Name),
        r#type: event_type.map(|typ| typ.into()),
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

fn create_thread_descriptor(pid: i32) -> ThreadDescriptor {
    ThreadDescriptor {
        pid: Some(pid),
        tid: Some(thread_id::get() as i32),
        thread_name: std::thread::current().name().map(|n| n.to_string()),
        ..Default::default()
    }
}

/// Appends a thread descriptor packet to the trace if not already sent.
fn append_thread_descriptor(trace: &mut Trace) {
    let descriptor_sent = THREAD_DESCRIPTOR_SENT.with(|v| v.swap(true, Ordering::SeqCst));
    if !descriptor_sent {
        let pid = std::process::id() as i32;
        let track_uuid = get_track_uuid();
        let thread_descriptor = create_thread_descriptor(pid);
        let track_descriptor = create_track_descriptor(
            track_uuid,
            std::thread::current().name(),
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
}

/// Appends a process descriptor packet to the trace if not already sent.
fn append_process_descriptor(trace: &mut Trace) {
    let descriptor_sent = PROCESS_DESCRIPTOR_SENT.swap(true, Ordering::SeqCst);
    if !descriptor_sent {
        let pid = std::process::id() as i32;
        let track_uuid = get_track_uuid();
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
}

/// Writes the trace to the output file.
fn write_trace(trace: &Trace, output: &mut File) {
    let mut buf = BytesMut::new();
    trace.encode(&mut buf).unwrap();
    output.write_all(&buf).unwrap();
    output.flush().unwrap()
}

impl Reporter for PerfettoReporter {
    fn report(&mut self, spans: Vec<SpanRecord>) {
        let mut trace = Trace::default();

        // Ensure process and thread descriptors are sent
        append_process_descriptor(&mut trace);
        append_thread_descriptor(&mut trace);

        let pid = std::process::id() as i32;
        let thread_track_uuid = get_track_uuid();
        for span in spans {
            let sequence_id = 42; // Replace with actual sequence ID if available

            // Start event packet
            let start_event = create_track_event(
                Some(span.name.into_owned()),
                thread_track_uuid,
                Some(track_event::Type::SliceBegin),
            );
            let start_packet = TracePacket {
                data: Some(Data::TrackEvent(start_event)),
                trusted_pid: Some(pid),
                timestamp: Some(span.begin_time_unix_ns),
                optional_trusted_packet_sequence_id: Some(
                    OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(sequence_id),
                ),
                ..Default::default()
            };

            trace.packet.push(start_packet);

            // End event packet
            let end_event =
                create_track_event(None, thread_track_uuid, Some(track_event::Type::SliceEnd));
            let end_packet = TracePacket {
                data: Some(Data::TrackEvent(end_event)),
                trusted_pid: Some(pid),
                timestamp: Some(span.begin_time_unix_ns + span.duration_ns),
                optional_trusted_packet_sequence_id: Some(
                    OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(sequence_id),
                ),
                ..Default::default()
            };

            trace.packet.push(end_packet);
        }

        write_trace(&trace, &mut self.output);
    }
}
