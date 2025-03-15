// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.

use config::Config;
use fastrace::collector::Reporter;
use fastrace::prelude::*;

pub mod config;

pub struct PerfettorReporter {}

impl PerfettorReporter {
    pub fn new() -> Self {
        Self {}
    }
    pub fn new_with_config(config: Config) -> Self {
        Self {}
    }
}

fn serialize(span_record: &SpanRecord) -> &[u8] {
    todo!()
}

fn write_to_file(bytes: &[u8]) {
    todo!()
}

impl Reporter for PerfettorReporter {
    fn report(&mut self, spans: Vec<SpanRecord>) {
        for span in spans {
            // serialize
            let bytes = serialize(&span);
            write_to_file(bytes);
        }
    }
}
