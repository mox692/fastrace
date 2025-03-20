use backend::perfetto_protos;
/// * spanにenterしたりexitすると, ログを書き込む
/// * それぞれのスレッドがspanをたくさん持つ (LocalSpans)
/// * 定期的にそのスレッドが, LocalSpansを, globalに転送する
///
/// ### 構造
///
/// |rawSpan|rawSpan|rawSpan|  rawSpan|rawSpan|rawSpan  |
///
/// |       span queue      |         span queue        |
///
/// |       span line       |         span line         |
///
/// |        LocalSpanStack                             |
///
/// * LocalSpanStack
///   * threadが起動された瞬間に作られる
///   * そのスレッドが生きている限りずっとある
///
/// * SpanLine
///   * set_local_parent により生成される, LocalParentGuard と紐づいてる
///     * register_span_line で登録される
///   * LocalParentGuard が drop されると, span_line.collect によってrawSpansが返される
///
/// * SpanQueue
///   * LocalSpan::enter_with_local_parent 等で生成される
///     * ここで, 入るべきSpanLineが決定的に決まることに注意(current_span_lineで撮ってる)
///   * SpanLineのstart_spanが呼ばれる
///   * そのまま, SpqnQueueの start_spanが呼ばれる
///   * SpqnQueueは内部にカウンタを持っていて,start_span を呼ぶたびにincrementされる
///   * LocalSpanがdropされると, end_timeが記録される
///   * LocalParentGuard がdropされると, SpanQueueごとtakeされる
///
/// * ログを吐くと, LocalSpanStackを経由して最終的にspan queueni入れられる
///   * thread localで, 今何番目のspan queueを使っているか, などんの情報がわかる
/// * 末端のspan queueは, object poolを使って, mallocが起こらないように && キャッシュが効くように
/// * LocalParentGuard が drop されると, `collect_spans_and_token` によって
///
///  ### 疑問
///
/// *  
///
///
/// # fastのデザイン
///
/// * sync traceがベース
///   * イベントやスパンは別のスレッドにmoveしない
/// * 親子関係は持たない
///   * spanline, spanqueue等の区別がいらない
/// * 単純な Span 型のみを持つ
///   * 生成時にstart_time
///   * drop時に end_time
/// * fastraceのように, thread local storageを使う
/// * fastraceのように, object storageは積極的に活用する
///   * locality, allocator呼び出しの低減
/// * spanのformatを決める
///   * 動的fieldを極力減らす
///   * enumでtype分けするイメージ
///
/// # TODO
/// * tlsに溜まったログを集めるタイミング
///   * それぞれのスレッドがpushすべきか, reporter threadがpullしに行くか
/// * threadが中断された際のflushの処理
///   * TLSだと, flushが難しい可能性がある -> いや, dropの実装で対応できる
///   * ただ, drop呼び出しはthread終了の順番に依存する模様 (main or consumerが先に終了しちゃうとロスが発生)
///
/// # Arena 自作の可能性
///
/// ### 機能
/// * スレッド固有のメモリ領域を提供する
///   * thread local storageに近いが, 他のthreadのmemory regionにもアクセスができる
/// * threadに固有のキーを割り当て, そのキーによってそのスレッドがどのメモリリージョンを触れるのかを判断
/// * 1つのスレッドが持てるデータ量はハードリミットが決まっている。
/// * しかし, 登録できるスレッドの数は制限しない。
///
/// ### 課題
/// * 新しくthreadが作成された時, どうやってその
/// * 衝突しないようなキーの作成方法
///
///
/// # まとめ
/// * worker threadにthread local storageを置く
/// * runtimeのshutdownの時に, workerは全てshutdownされる
/// * worker threadが消えた後に, consumer thread(こいつはshutdownされても残り続ける!)がflushを同期的に行う
///
use bytes::BytesMut;
use fastrace::collector::Reporter;
use fastrace::prelude::*;
pub mod fast;
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
use utils::spsc::bounded;
pub mod config;
pub mod macros;
mod utils;

pub mod backend;

use crate::utils::spsc::{Receiver, Sender};
use fastant::Instant;
use std::{
    cell::{RefCell, UnsafeCell},
    rc::Rc,
    sync::Mutex,
};

struct RunTask {}
struct RuntimeStart {}
struct RuntimeTerminate {}
struct ThreadDiscriptor {}
struct ProcessDiscriptor {}

fn type_name(typ: Type) -> &'static str {
    match typ {
        Type::RunTask(_) => "run_task",
        Type::RuntimeStart(_) => "runtime_start",
        Type::RuntimeTarminate(_) => "runtime_terminate",
        Type::ThreadDiscriptor(_) => "thread_discriptor",
        Type::ProcessDiscriptor(_) => "process_discriptor",
    }
}

pub enum Type {
    RunTask(RunTask),
    RuntimeStart(RuntimeStart),
    RuntimeTarminate(RuntimeTerminate),
    // perfetto specific
    ThreadDiscriptor(ThreadDiscriptor),
    ProcessDiscriptor(ProcessDiscriptor),
}

pub struct RawSpan {
    typ: Type,
    thread_id: u64,
    start: Instant,
    end: Instant,
}

/// A span. This should be dropped in the same therad.
pub struct Span {
    inner: Option<RawSpan>,
    span_queue_handle: Rc<RefCell<SpanQueue>>,
}

/// Create a span.
fn span(typ: Type, thread_id: u64) -> Span {
    with_span_queue(|span_queue| {
        if enabled() {
            Span {
                inner: Some(RawSpan {
                    typ,
                    thread_id,
                    start: Instant::now(),
                    end: Instant::ZERO,
                }),
                span_queue_handle: span_queue.clone(),
            }
        } else {
            Span {
                inner: None,
                span_queue_handle: span_queue.clone(),
            }
        }
    })
}

static ENABLED: AtomicBool = AtomicBool::new(false);

fn enabled() -> bool {
    ENABLED.load(std::sync::atomic::Ordering::Relaxed)
}

fn set_enabled(set: bool) {
    ENABLED.store(set, std::sync::atomic::Ordering::Relaxed);
}

impl Drop for Span {
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

/// Each thread has their own `LocalSpans` in TLS.
pub struct SpanQueue {
    spans: Vec<RawSpan>,
}

const DEFAULT_BATCH_SIZE: usize = 16384;
impl SpanQueue {
    fn new() -> Self {
        Self {
            spans: Vec::with_capacity(DEFAULT_BATCH_SIZE),
        }
    }

    fn push(&mut self, span: RawSpan) {
        if self.spans.len() == DEFAULT_BATCH_SIZE - 1 {
            // flush spans
            let spans: Vec<RawSpan> = self.drain().collect();
            send_command(Command::SendSpans(spans));
            return;
        }
        self.spans.push(span);
    }

    /// Called from the span consumer
    fn drain(&mut self) -> impl Iterator<Item = RawSpan> + '_ {
        self.spans.drain(..)
    }
}

impl Drop for SpanQueue {
    /// When SpanQueue is used as a thread local value, then this drop gets called
    /// at the time when this thread is terminated, making sure all spans would not
    /// be lossed.
    fn drop(&mut self) {
        let command = Command::SendSpans(self.drain().collect());
        send_command(command);
    }
}

/// TODO: Do we need Send + 'static bound?
pub trait SpanConsumer: Send + 'static {
    /// TODO: Can spans be abstracted?
    fn consume(&mut self, spans: Vec<RawSpan>);
}

static GLOBAL_SPAN_CONSUMER: Mutex<GlobalSpanConsumer> = Mutex::new(GlobalSpanConsumer::new());

struct GlobalSpanConsumer {
    consumer: Option<Box<dyn SpanConsumer>>,
}

impl GlobalSpanConsumer {
    const fn new() -> Self {
        Self { consumer: None }
    }

    fn handle_commands(&mut self) {
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

/// Stop tracing.
///
/// This function flushes spans that the consumer thread has, but doesn't against the
/// spans that is owned by worker threads.
pub fn stop() {
    set_enabled(false);
}

/// Start tracing. Before calling this, you have to call `initialize` first.
pub fn start() {
    // TODO: check if `initialize` has been called.
    set_enabled(true)
}

/// Mainly used for perfetto tracing, where we need to publish process descriptor first.
static FLUSHED_ONCE: AtomicBool = AtomicBool::new(false);

fn flushed_once() -> bool {
    FLUSHED_ONCE.load(Ordering::Relaxed)
}
fn set_flushed_once(set: bool) {
    FLUSHED_ONCE.store(set, Ordering::Relaxed);
}

/// Initialize tracing.
pub fn initialize(config: Config, consumer: impl SpanConsumer) {
    // spawn consumer thread

    let mut global_consumer = GLOBAL_SPAN_CONSUMER.lock().unwrap();
    global_consumer.consumer = Some(Box::new(consumer));
    drop(global_consumer);

    std::thread::Builder::new()
        .name("global-span-consumer".to_string())
        .spawn(move || {
            loop {
                let mut global_consumer = GLOBAL_SPAN_CONSUMER.lock().unwrap();
                global_consumer.handle_commands();
                drop(global_consumer);

                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        })
        .unwrap();
}

pub struct Config {}

static SPSC_RXS: Mutex<Vec<Receiver<Command>>> = Mutex::new(Vec::new());

enum Command {
    SendSpans(Vec<RawSpan>),
}

fn register_receiver(rx: Receiver<Command>) {
    SPSC_RXS.lock().unwrap().push(rx);
}

fn send_command(cmd: Command) {
    COMMAND_SENDER
        .try_with(|sender| unsafe { (*sender.get()).send(cmd).ok() })
        .ok();
}

/// This is called when a SpanQueue at local storage gets initialized.
fn thread_descriptor() -> RawSpan {
    RawSpan {
        typ: Type::ThreadDiscriptor(ThreadDiscriptor {}),
        thread_id: thread_id::get() as u64,
        start: Instant::ZERO,
        end: Instant::ZERO,
    }
}

/// This is called when a SpanQueue at local storage gets initialized.
fn process_descriptor() -> RawSpan {
    RawSpan {
        typ: Type::ProcessDiscriptor(ProcessDiscriptor {}),
        thread_id: thread_id::get() as u64,
        start: Instant::ZERO,
        end: Instant::ZERO,
    }
}

thread_local! {
    static SPAN_QUEUE: Rc<RefCell<SpanQueue>> = {
        let mut queue = SpanQueue::new();

        // perfetto specific operation.
        // TODO: Can we put this logic elsewhere?
        queue.push(thread_descriptor());

        Rc::new(RefCell::new(queue))
    };

    static COMMAND_SENDER: UnsafeCell<Sender<Command>> = {
        let (tx, rx) = bounded(10240);
        register_receiver(rx);
        UnsafeCell::new(tx)
    };
}

fn with_span_queue<R>(f: impl FnOnce(&Rc<RefCell<SpanQueue>>) -> R) -> R {
    SPAN_QUEUE.with(|queue| f(queue))
}
