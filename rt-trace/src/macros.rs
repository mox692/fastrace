/// aaa
///
/// ```rust
/// let s = local_span!("span_name", "key1" -> "value1", "key2" -> "value2");
/// ```
#[macro_export]
macro_rules! local_span {
    ($name:expr, $($key:literal -> $value:expr),* $(,)?) => {{
        LocalSpan::enter_with_local_parent($name)
            .with_properties(|| [$(($key, $value)),*])
    }};
}
