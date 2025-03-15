// Reference
// * https://perfetto.dev/docs/reference/synthetic-track-event

use bytes::BytesMut;
use fastrace::collector::Reporter;
use fastrace::prelude::*;
use prost::Message;

pub mod config;
mod perfetto_protos;

use perfetto_protos::{
    trace_packet::{Data, OptionalTrustedPacketSequenceId, OptionalTrustedUid},
    track_descriptor::StaticOrDynamicName,
    track_event::{self, NameField},
    CounterDescriptor, ProcessDescriptor, ThreadDescriptor, Trace, TracePacket, TrackDescriptor,
    TrackEvent,
};
use std::time::{SystemTime, UNIX_EPOCH};

use std::{
    fs::File,
    io::Write,
    path::Path,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

// TODO: comment in
// mod idl {
//     include!(concat!(env!("OUT_DIR"), "/perfetto.protos.rs"));
// }

thread_local! {
    /// Note that this ID will be used throughout the rest of the program,
    /// even if Reporter instance would be changed in the middle.
    static TRACK_UUID: AtomicU64 = {
        let now = SystemTime::now();
        let duration = now.duration_since(UNIX_EPOCH).expect("Time went backwards");
        let seed = duration.as_nanos() as u64;

        AtomicU64::new(seed)
    };
    static THREAD_DESCRIPTOR_SENT: AtomicBool = AtomicBool::new(false);
}

fn get_track_uuid() -> u64 {
    TRACK_UUID.with(|id| id.load(Ordering::Relaxed))
}

static PROCESS_DESCRIPTOR_SENT: AtomicBool = AtomicBool::new(false);

/// PerfettorReporter.
pub struct PerfettorReporter {
    output: File,
}

impl PerfettorReporter {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            output: File::create(path.as_ref()).unwrap(),
        }
    }
}

/// Docs: https://perfetto.dev/docs/reference/trace-packet-proto#TrackEvent
fn create_track_event(
    name: Option<String>,
    track_uuid: u64,
    r#type: Option<track_event::Type>,
) -> TrackEvent {
    let mut event = TrackEvent::default();
    event.track_uuid = Some(track_uuid);
    event.name_field = name.map(NameField::Name);
    event.r#type = r#type.map(|typ| typ.into());

    event
}

/// Docs: https://perfetto.dev/docs/reference/trace-packet-proto#ProcessDescriptor
fn create_process_descriptor(pid: i32) -> ProcessDescriptor {
    let mut process = ProcessDescriptor::default();
    process.pid = Some(pid);

    process
}

/// Docs https://perfetto.dev/docs/reference/trace-packet-proto#TrackDescriptor
fn create_track_descriptor(
    uuid: Option<u64>,
    name: Option<impl AsRef<str>>,
    process: Option<ProcessDescriptor>,
    thread: Option<ThreadDescriptor>,
) -> TrackDescriptor {
    let mut desc = TrackDescriptor::default();
    desc.uuid = uuid;
    desc.static_or_dynamic_name = name
        .map(|s| s.as_ref().to_string())
        .map(StaticOrDynamicName::Name);
    desc.process = process;
    desc.thread = thread;
    desc
}

fn create_thread_descriptor(pid: i32) -> ThreadDescriptor {
    let mut thread = ThreadDescriptor::default();
    thread.pid = Some(pid);
    thread.tid = Some(thread_id::get() as _);
    thread.thread_name = std::thread::current().name().map(|n| n.to_string());
    thread
}

fn append_thread_descriptor(trace: &mut Trace) {
    let thread_first_frame_sent =
        THREAD_DESCRIPTOR_SENT.with(|v| v.fetch_or(true, Ordering::SeqCst));
    if !thread_first_frame_sent {
        let mut packet = TracePacket::default();
        let pid = std::process::id() as i32;
        let thread = create_thread_descriptor(pid).into();
        let track_desc = create_track_descriptor(
            get_track_uuid().into(),
            std::thread::current().name(),
            Some(create_process_descriptor(pid)),
            thread,
        );
        packet.data = Some(Data::TrackDescriptor(track_desc));
        trace.packet.push(packet);
    }
}

fn append_process_descriptor(trace: &mut Trace) {
    let process_first_frame_sent = PROCESS_DESCRIPTOR_SENT.fetch_or(true, Ordering::SeqCst);
    if !process_first_frame_sent {
        let mut packet = TracePacket::default();
        let pid = std::process::id() as i32;
        let process = create_process_descriptor(pid).into();
        let track_desc = create_track_descriptor(Some(32), None::<&str>, process, None);
        packet.data = Some(Data::TrackDescriptor(track_desc));
        trace.packet.push(packet);
    }
}

fn write_log(trace: Trace, output: &mut File) {
    let mut buf = BytesMut::new();

    for p in &trace.packet {
        println!("packet: {:?}", &p);
    }

    let Ok(_) = trace.encode(&mut buf) else {
        return;
    };

    output.write_all(buf.iter().as_slice()).unwrap();
    output.flush().unwrap()
}

impl Reporter for PerfettorReporter {
    fn report(&mut self, spans: Vec<SpanRecord>) {
        let mut trace = Trace::default();

        append_process_descriptor(&mut trace);
        append_thread_descriptor(&mut trace);

        for span in spans {
            //
            // start event
            //

            // serialize
            let mut start_packet = TracePacket::default();

            // common data
            let thread_track_uuid = get_track_uuid();
            let pid = std::process::id() as i32;

            // data
            let event = create_track_event(
                Some(span.name.into_owned()),
                thread_track_uuid,
                Some(track_event::Type::SliceBegin),
            );
            start_packet.data = Some(Data::TrackEvent(event));
            start_packet.trusted_pid = Some(pid);
            start_packet.timestamp = Some(span.begin_time_unix_ns);
            start_packet.optional_trusted_packet_sequence_id =
                Some(OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(
                    // self.sequence_id.get() as _,
                    42,
                ));

            trace.packet.push(start_packet);

            //
            // end event
            //
            // serialize
            let mut end_packet = TracePacket::default();

            // data
            let event =
                create_track_event(None, thread_track_uuid, Some(track_event::Type::SliceEnd));
            end_packet.data = Some(Data::TrackEvent(event));
            end_packet.trusted_pid = Some(pid);
            end_packet.timestamp = Some(span.begin_time_unix_ns + span.duration_ns);
            end_packet.optional_trusted_packet_sequence_id =
                Some(OptionalTrustedPacketSequenceId::TrustedPacketSequenceId(
                    // self.sequence_id.get() as _,
                    42,
                ));

            trace.packet.push(end_packet);
        }

        write_log(trace, &mut self.output);
    }
}
