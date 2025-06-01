use std::time::Duration;

#[derive(Default)]
pub struct Config {
    pub(crate) consumer_thread_sleep_duration: Option<Duration>,
}
