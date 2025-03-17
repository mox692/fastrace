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
/// * 親子関係は持たない
///   * spanline, spanqueue等の区別がいらない
/// * 単純な Span 型のみを持つ
///   * 生成時にstart_time
///   * drop時に end_time
/// * fastraceのように, object storageは積極的に活用する
///   * locality, allocator呼び出しの低減
/// * spanのformatを決める
///   * 動的fieldを極力減らす
///   * enumでtype分けするイメージ
///
///
///

pub enum SpanType {
    RunTask = 0,
    RuntimeStart = 1,
    RuntimeTarminate = 2,
}

pub struct Span {
    typ: SpanType,
}

/// Each thread has their own `LocalSpans` in TLS.
pub struct LocalSpans {
    spans: Vec<Span>,
}
