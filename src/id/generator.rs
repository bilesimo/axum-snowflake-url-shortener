use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio::sync::Mutex;

use crate::{configuration::IdSettings, error::AppError};

type SleepFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

trait TimeSource: std::fmt::Debug + Send + Sync {
    fn now_millis(&self) -> Result<u64, AppError>;
    fn sleep<'a>(&'a self, duration: Duration) -> SleepFuture<'a>;
}

#[derive(Debug)]
struct SystemTimeSource;

impl TimeSource for SystemTimeSource {
    fn now_millis(&self) -> Result<u64, AppError> {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| AppError::Internal(format!("system time error: {error}")))?
            .as_millis() as u64)
    }

    fn sleep<'a>(&'a self, duration: Duration) -> SleepFuture<'a> {
        Box::pin(tokio::time::sleep(duration))
    }
}

#[derive(Clone, Debug)]
pub struct SnowflakeIdGenerator {
    node_id: u16,
    epoch_millis: u64,
    state: Arc<Mutex<SnowflakeState>>,
    time_source: Arc<dyn TimeSource>,
}

#[derive(Debug)]
struct SnowflakeState {
    last_timestamp_millis: u64,
    sequence: u16,
}

impl SnowflakeIdGenerator {
    const NODE_ID_BITS: u64 = 10;
    const SEQUENCE_BITS: u64 = 12;
    const MAX_NODE_ID: u16 = (1 << Self::NODE_ID_BITS) - 1;
    const MAX_SEQUENCE: u16 = (1 << Self::SEQUENCE_BITS) - 1;

    pub fn new(settings: &IdSettings) -> Result<Self, AppError> {
        Self::with_time_source(settings, Arc::new(SystemTimeSource))
    }

    fn with_time_source(
        settings: &IdSettings,
        time_source: Arc<dyn TimeSource>,
    ) -> Result<Self, AppError> {
        if settings.node_id > Self::MAX_NODE_ID {
            return Err(AppError::Configuration(format!(
                "snowflake node_id {} exceeds max {}",
                settings.node_id,
                Self::MAX_NODE_ID
            )));
        }

        Ok(Self {
            node_id: settings.node_id,
            epoch_millis: settings.epoch_millis,
            state: Arc::new(Mutex::new(SnowflakeState {
                last_timestamp_millis: 0,
                sequence: 0,
            })),
            time_source,
        })
    }

    pub async fn next_id(&self) -> Result<i64, AppError> {
        loop {
            let mut state = self.state.lock().await;
            let timestamp = self.time_source.now_millis()?;

            if timestamp < state.last_timestamp_millis {
                let wait_duration = Duration::from_millis(state.last_timestamp_millis - timestamp);
                drop(state);
                self.time_source.sleep(wait_duration).await;
                continue;
            }

            if timestamp == state.last_timestamp_millis {
                if state.sequence == Self::MAX_SEQUENCE {
                    drop(state);
                    self.time_source.sleep(Duration::from_millis(1)).await;
                    continue;
                }

                state.sequence += 1;
            } else {
                state.sequence = 0;
            }

            state.last_timestamp_millis = timestamp;
            return self.build_id(timestamp, state.sequence);
        }
    }

    fn build_id(&self, timestamp: u64, sequence: u16) -> Result<i64, AppError> {
        let relative_timestamp = timestamp.checked_sub(self.epoch_millis).ok_or_else(|| {
            AppError::Internal("snowflake epoch is ahead of current time".to_owned())
        })?;

        let id = (relative_timestamp << (Self::NODE_ID_BITS + Self::SEQUENCE_BITS))
            | ((self.node_id as u64) << Self::SEQUENCE_BITS)
            | sequence as u64;

        Ok(id as i64)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{HashSet, VecDeque},
        sync::{Arc, Mutex as StdMutex},
        time::Duration,
    };

    use crate::configuration::IdSettings;
    use tokio::{sync::Notify, time::timeout};

    use super::{SleepFuture, SnowflakeIdGenerator, TimeSource};

    #[derive(Debug)]
    struct MockTimeSource {
        timestamps: StdMutex<VecDeque<u64>>,
        sleep_calls: StdMutex<Vec<Duration>>,
    }

    impl MockTimeSource {
        fn new(timestamps: impl IntoIterator<Item = u64>) -> Self {
            Self {
                timestamps: StdMutex::new(timestamps.into_iter().collect()),
                sleep_calls: StdMutex::new(Vec::new()),
            }
        }

        fn sleep_calls(&self) -> Vec<Duration> {
            self.sleep_calls.lock().expect("sleep calls mutex").clone()
        }
    }

    impl TimeSource for MockTimeSource {
        fn now_millis(&self) -> Result<u64, crate::error::AppError> {
            self.timestamps
                .lock()
                .expect("timestamps mutex")
                .pop_front()
                .ok_or_else(|| crate::error::AppError::Internal("mock time exhausted".to_owned()))
        }

        fn sleep<'a>(&'a self, duration: Duration) -> SleepFuture<'a> {
            self.sleep_calls
                .lock()
                .expect("sleep calls mutex")
                .push(duration);
            Box::pin(std::future::ready(()))
        }
    }

    #[derive(Debug)]
    struct BlockingSleepTimeSource {
        timestamps: StdMutex<VecDeque<u64>>,
        sleep_calls: StdMutex<Vec<Duration>>,
        sleep_started: Arc<Notify>,
        release_sleep: Arc<Notify>,
    }

    impl BlockingSleepTimeSource {
        fn new(timestamps: impl IntoIterator<Item = u64>) -> Self {
            Self {
                timestamps: StdMutex::new(timestamps.into_iter().collect()),
                sleep_calls: StdMutex::new(Vec::new()),
                sleep_started: Arc::new(Notify::new()),
                release_sleep: Arc::new(Notify::new()),
            }
        }

        async fn wait_for_sleep_started(&self) {
            self.sleep_started.notified().await;
        }

        fn release_sleep(&self) {
            self.release_sleep.notify_one();
        }

        fn sleep_calls(&self) -> Vec<Duration> {
            self.sleep_calls.lock().expect("sleep calls mutex").clone()
        }
    }

    impl TimeSource for BlockingSleepTimeSource {
        fn now_millis(&self) -> Result<u64, crate::error::AppError> {
            self.timestamps
                .lock()
                .expect("timestamps mutex")
                .pop_front()
                .ok_or_else(|| crate::error::AppError::Internal("mock time exhausted".to_owned()))
        }

        fn sleep<'a>(&'a self, duration: Duration) -> SleepFuture<'a> {
            self.sleep_calls
                .lock()
                .expect("sleep calls mutex")
                .push(duration);
            self.sleep_started.notify_one();

            let release_sleep = self.release_sleep.clone();
            Box::pin(async move {
                release_sleep.notified().await;
            })
        }
    }

    #[tokio::test]
    async fn snowflake_ids_are_unique() {
        let generator = SnowflakeIdGenerator::new(&IdSettings {
            node_id: 1,
            epoch_millis: 1704067200000,
        })
        .expect("generator");

        let mut ids = HashSet::new();

        for _ in 0..1024 {
            let id = generator.next_id().await.expect("id");
            assert!(ids.insert(id));
        }
    }

    #[test]
    fn rejects_invalid_node_id() {
        let result = SnowflakeIdGenerator::new(&IdSettings {
            node_id: 2048,
            epoch_millis: 1704067200000,
        });

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn waits_for_clock_recovery_instead_of_returning_an_error() {
        let time_source = Arc::new(MockTimeSource::new([100, 99, 100]));
        let generator = SnowflakeIdGenerator::with_time_source(
            &IdSettings {
                node_id: 1,
                epoch_millis: 0,
            },
            time_source.clone(),
        )
        .expect("generator");

        let first = generator.next_id().await.expect("first id");
        let second = generator.next_id().await.expect("second id");

        assert!(second > first);
        assert_eq!(time_source.sleep_calls(), vec![Duration::from_millis(1)]);
    }

    #[tokio::test]
    async fn releases_the_mutex_while_waiting_for_sequence_rollover() {
        let time_source = Arc::new(BlockingSleepTimeSource::new([500, 501]));
        let generator = SnowflakeIdGenerator::with_time_source(
            &IdSettings {
                node_id: 1,
                epoch_millis: 0,
            },
            time_source.clone(),
        )
        .expect("generator");

        {
            let mut state = generator.state.lock().await;
            state.last_timestamp_millis = 500;
            state.sequence = SnowflakeIdGenerator::MAX_SEQUENCE;
        }

        let waiting_generator = generator.clone();
        let waiting_task = tokio::spawn(async move { waiting_generator.next_id().await });

        time_source.wait_for_sleep_started().await;

        let mutex_guard = timeout(Duration::from_millis(50), generator.state.lock())
            .await
            .expect("mutex should be released while waiting");
        drop(mutex_guard);

        time_source.release_sleep();

        waiting_task
            .await
            .expect("task join")
            .expect("generated id after rollover");
        assert_eq!(time_source.sleep_calls(), vec![Duration::from_millis(1)]);
    }
}
