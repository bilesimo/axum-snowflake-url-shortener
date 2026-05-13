use std::{
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio::sync::Mutex;

use crate::{configuration::IdSettings, error::AppError};

#[derive(Clone, Debug)]
pub struct SnowflakeIdGenerator {
    node_id: u16,
    epoch_millis: u64,
    state: std::sync::Arc<Mutex<SnowflakeState>>,
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
            state: std::sync::Arc::new(Mutex::new(SnowflakeState {
                last_timestamp_millis: 0,
                sequence: 0,
            })),
        })
    }

    pub async fn next_id(&self) -> Result<i64, AppError> {
        let mut state = self.state.lock().await;
        let mut timestamp = current_timestamp_millis()?;

        if timestamp < state.last_timestamp_millis {
            return Err(AppError::Internal(
                "system clock moved backwards during snowflake generation".to_owned(),
            ));
        }

        if timestamp == state.last_timestamp_millis {
            state.sequence = (state.sequence + 1) & Self::MAX_SEQUENCE;
            if state.sequence == 0 {
                timestamp = wait_for_next_millisecond(state.last_timestamp_millis)?;
            }
        } else {
            state.sequence = 0;
        }

        state.last_timestamp_millis = timestamp;

        let relative_timestamp = timestamp.checked_sub(self.epoch_millis).ok_or_else(|| {
            AppError::Internal("snowflake epoch is ahead of current time".to_owned())
        })?;

        let id = (relative_timestamp << (Self::NODE_ID_BITS + Self::SEQUENCE_BITS))
            | ((self.node_id as u64) << Self::SEQUENCE_BITS)
            | state.sequence as u64;

        Ok(id as i64)
    }
}

fn current_timestamp_millis() -> Result<u64, AppError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AppError::Internal(format!("system time error: {error}")))?
        .as_millis() as u64)
}

fn wait_for_next_millisecond(last_timestamp_millis: u64) -> Result<u64, AppError> {
    loop {
        let timestamp = current_timestamp_millis()?;
        if timestamp > last_timestamp_millis {
            return Ok(timestamp);
        }

        thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use crate::configuration::IdSettings;

    use super::SnowflakeIdGenerator;

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
}
