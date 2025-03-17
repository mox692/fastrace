/// * spanにenterしたりexitすると, ログを書き込む
/// * それぞれのスレッドがspanをたくさん持つ (LocalSpans)
/// * 定期的にそのスレッドが, LocalSpansを, globalに転送する

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
