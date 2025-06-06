use std::time::Duration;

#[derive(Default, Debug)]
pub struct Config {
    pub(crate) consumer_thread_sleep_duration: Option<Duration>,
}

impl Config {
    /// Create a new configuration with the default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the consumer thread sleep duration.
    pub fn consumer_thread_sleep_duration(&mut self, duration: Duration) -> &mut Self {
        self.consumer_thread_sleep_duration = Some(duration);
        self
    }
}
