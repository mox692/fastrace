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
///
pub mod object_pool;
use fastant::Instant;
use object_pool::{Pool, Puller, Reusable};
use once_cell::sync::Lazy;
use std::cell::RefCell;

pub enum SpanType {
    RunTask = 0,
    RuntimeStart = 1,
    RuntimeTarminate = 2,
}

struct RunTask {}
struct RuntimeStart {}
struct RuntimeTerminate {}

fn type_name(typ: Type) -> &'static str {
    match typ {
        Type::RunTask(_) => "run_task",
        Type::RuntimeStart(_) => "runtime_start",
        Type::RuntimeTarminate(_) => "runtime_terminate",
    }
}

enum Type {
    RunTask(RunTask),
    RuntimeStart(RuntimeStart),
    RuntimeTarminate(RuntimeTerminate),
}

pub struct RawSpan {
    typ: Type,
    thread_id: u64,
    start: Instant,
    end: Instant,
}

/// A span. This should be dropped in the same therad.
pub struct Span {
    inner: RawSpan,
    // handle: LocalSpansHandle,
}

fn span(typ: Type, thread_id: u64) -> Span {
    Span {
        inner: RawSpan {
            typ,
            thread_id,
            start: Instant::now(),
            end: Instant::ZERO,
        },
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        self.inner.end = Instant::now();
    }
}

/// Each thread has their own `LocalSpans` in TLS.
pub struct SpanQueue {
    spans: RawSpans,
}

impl SpanQueue {
    fn new() -> Self {
        Self {
            spans: RawSpans::default(),
        }
    }
}

thread_local! {
    pub static SPAN_QUEUE: Rc<RefCell<SpanQueue>> = Rc::new(RefCell::new(SpanQueue::new()));
}

static RAW_SPANS_POOL: Lazy<Pool<Vec<RawSpan>>> = Lazy::new(|| Pool::new(Vec::new, Vec::clear));

thread_local! {
    static RAW_SPANS_PULLER: RefCell<Puller<'static, Vec<RawSpan>>> = RefCell::new(RAW_SPANS_POOL.puller(512));
}

pub type RawSpans = Reusable<'static, Vec<RawSpan>>;

impl Default for RawSpans {
    fn default() -> Self {
        RAW_SPANS_PULLER
            .try_with(|puller| puller.borrow_mut().pull())
            .unwrap_or_else(|_| Reusable::new(&*RAW_SPANS_POOL, vec![]))
    }
}

impl FromIterator<RawSpan> for RawSpans {
    fn from_iter<T: IntoIterator<Item = RawSpan>>(iter: T) -> Self {
        let mut raw_spans = RawSpans::default();
        raw_spans.extend(iter);
        raw_spans
    }
}
