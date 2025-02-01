// Copyright 2024 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// This file is derived from [1] under the original license header:
// Copyright 2020 TiKV Project Authors. Licensed under Apache-2.0.
// [1]: https://github.com/tikv/minitrace-rust/blob/v0.6.4/minitrace/examples/asynchronous.rs

#![allow(clippy::new_without_default)]

use std::borrow::Cow;
use std::time::Duration;

use fastrace::collector::Config;
use fastrace::collector::Reporter;
use fastrace::prelude::*;
use opentelemetry_otlp::WithExportConfig;

fn parallel_job() -> Vec<tokio::task::JoinHandle<()>> {
    let mut v = Vec::with_capacity(4);
    for i in 0..4 {
        v.push(tokio::spawn(
            iter_job(i).in_span(Span::enter_with_local_parent("iter job")),
        ));
    }
    v
}

async fn iter_job(iter: u64) {
    std::thread::sleep(std::time::Duration::from_millis(iter * 10));
    tokio::task::yield_now().await;
    other_job().await;
}

#[trace(enter_on_poll = true)]
async fn other_job() {
    for i in 0..20 {
        if i == 10 {
            tokio::task::yield_now().await;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

#[tokio::main]
async fn main() {
    fastrace::set_reporter(MyReporter::new(), Config::default());

    {
        let span = Span::root("root", SpanContext::random());

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        drop(span);
    }

    fastrace::flush();
}

async fn simple() {
    fastrace::set_reporter(MyReporter::new(), Config::default());

    {
        let span = Span::root("root", SpanContext::random());

        // execution
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        drop(span);
    }

    fastrace::flush();
}

async fn local_span() {
    fastrace::set_reporter(MyReporter::new(), Config::default());

    {
        let root = Span::root("root", SpanContext::random());

        let _g = root.set_local_parent();

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    fastrace::flush();
}

pub struct MyReporter {}
impl MyReporter {
    fn new() -> Self {
        Self {}
    }
}
impl Reporter for MyReporter {
    fn report(&mut self, spans: Vec<SpanRecord>) {
        println!("spans: {:?}", &spans)
    }
}
