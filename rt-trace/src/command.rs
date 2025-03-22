use crate::span::RawSpan;

pub(crate) enum Command {
    SendSpans(Vec<RawSpan>),
}
