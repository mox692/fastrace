// Reference
// * https://perfetto.dev/docs/reference/synthetic-track-event

use crate::config;
use crate::macros;
use crate::utils;
use bytes::BytesMut;
use fastrace::collector::Reporter;
use fastrace::prelude::*;
use perfetto_protos::{
    ProcessDescriptor, ThreadDescriptor, Trace, TracePacket, TrackDescriptor, TrackEvent,
    trace_packet::{Data, OptionalTrustedPacketSequenceId, OptionalTrustedUid},
    track_descriptor::StaticOrDynamicName,
    track_event::{self, NameField},
};
use prost::Message;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fs::File,
    io::Write,
    path::Path,
    sync::{
        OnceLock, RwLock,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use crate::perfetto_protos;

thread_local! {
    /// Unique identifier for the track associated with the current thread.
    static TLS_DATA: TlsData = TlsData::new();

    /// Indicator whether the thread descriptor has been sent.
    static THREAD_DESCRIPTOR_SENT: AtomicBool = AtomicBool::new(false);
}

// A map of track_uuid and `ThreadInfo`.
static TRACK_MAP: OnceLock<RwLock<HashMap<u64, ThreadInfo>>> = OnceLock::new();
static INITIALIZED_SET: OnceLock<RwLock<HashSet<u64>>> = OnceLock::new();
static THREAD_COUNTER: AtomicUsize = AtomicUsize::new(0);

struct ThreadInfo {
    thread_id: usize,
    thread_name: &'static str,
}

// TODO: implement drop to cleanup leaked buffer.
struct TlsData {
    track_uuid_str: &'static str,
}

impl TlsData {
    fn new() -> Self {
        let counter = THREAD_COUNTER.fetch_add(1, Ordering::Relaxed);
        let track_uuid = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos() as u64)
            + counter as u64;
        let track_uuid_str = track_uuid.to_string().leak();
        let thread_name = std::thread::current()
            .name()
            .unwrap_or(format!("thread-{counter}").as_str())
            .to_owned()
            .leak();

        let map = TRACK_MAP.get_or_init(|| RwLock::new(HashMap::new()));
        let mut guard = map.write().unwrap();
        guard.insert(
            track_uuid,
            ThreadInfo {
                thread_id: thread_id::get(),
                thread_name,
            },
        );

        Self { track_uuid_str }
    }
}

fn track_uuid_str() -> &'static str {
    TLS_DATA.with(|data| data.track_uuid_str)
}

/// Static indicator whether the process descriptor has been sent.
static PROCESS_DESCRIPTOR_SENT: AtomicBool = AtomicBool::new(false);

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

pub fn enter_with_local_parent_with_thread_id(name: impl Into<Cow<'static, str>>) -> LocalSpan {
    LocalSpan::enter_with_local_parent(name).with_property(|| ("track_uuid", track_uuid_str()))
}
pub fn root_with_thread_id(name: impl Into<Cow<'static, str>>, ctx: SpanContext) -> Span {
    Span::root("root", ctx).with_property(|| ("track_uuid", track_uuid_str()))
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

fn create_thread_descriptor(pid: i32, thread_id: usize, thread_name: String) -> ThreadDescriptor {
    ThreadDescriptor {
        pid: Some(pid),
        tid: Some(thread_id as i32),
        thread_name: Some(thread_name),
        ..Default::default()
    }
}

fn descriptor_initialized(track_uuid: u64) -> bool {
    let v = INITIALIZED_SET.get_or_init(|| RwLock::new(HashSet::new()));
    let guard = v.read().unwrap();
    guard.get(&track_uuid).is_some()
}

fn set_descriptor_initialized(track_uuid: u64) {
    let v = INITIALIZED_SET.get_or_init(|| RwLock::new(HashSet::new()));
    let mut guard = v.write().unwrap();
    guard.insert(track_uuid);
}

/// Appends a thread descriptor packet to the trace if not already sent.
fn append_thread_descriptor(trace: &mut Trace, track_uuid: u64) {
    if !descriptor_initialized(track_uuid) {
        set_descriptor_initialized(track_uuid);

        // TODO: avoid syscall
        let pid = std::process::id() as i32;

        let map = TRACK_MAP.get().expect("should not be None");
        let guard = map.read().unwrap();
        let thread_info = guard.get(&track_uuid).expect("should not be None");

        let thread_descriptor = create_thread_descriptor(
            pid,
            thread_info.thread_id,
            thread_info.thread_name.to_string(),
        );
        let track_descriptor = create_track_descriptor(
            track_uuid,
            Some(thread_info.thread_name),
            Some(create_process_descriptor(pid)),
            Some(thread_descriptor),
        );

        drop(guard);

        let packet = TracePacket {
            data: Some(Data::TrackDescriptor(track_descriptor)),
            optional_trusted_uid: Some(OptionalTrustedUid::TrustedUid(42)),
            ..Default::default()
        };

        // Insert the packet at the beginning if needed
        trace.packet.insert(0, packet);
    }
}

fn append_process_descriptor(trace: &mut Trace, track_uuid: u64) {
    let descriptor_sent = PROCESS_DESCRIPTOR_SENT.swap(true, Ordering::SeqCst);
    if !descriptor_sent {
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
}

/// Writes the trace to the output file.
fn write_trace(trace: &Trace, output: &mut File) {
    let mut buf = BytesMut::new();
    trace.encode(&mut buf).unwrap();
    output.write_all(&buf).unwrap();
    output.flush().unwrap()
}

fn get_track_uuid_from_span_record(span: &SpanRecord) -> Option<u64> {
    let (_, v) = span
        .properties
        .iter()
        .find(|prop| prop.0.as_ref() == "track_uuid")?;

    v.parse::<u64>().ok()
}

impl Reporter for PerfettoReporter {
    fn report(&mut self, spans: Vec<SpanRecord>) {
        let mut trace = Trace::default();

        // TODO: avoid syscall
        let pid = std::process::id() as i32;

        for span in spans {
            let sequence_id = 42; // Replace with actual sequence ID if available

            let thread_track_uuid = get_track_uuid_from_span_record(&span).expect(
                format!("thread_track_uuid should not be None, span: {:?}", &span).as_str(),
            );

            append_process_descriptor(&mut trace, thread_track_uuid);
            append_thread_descriptor(&mut trace, thread_track_uuid);

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
