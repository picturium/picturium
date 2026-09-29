use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::thread::available_parallelism;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OfficeConfig {
    pub conversion_timeout: u64,
    pub kill_timeout: u64,
    pub max_processes: usize,
}

impl Default for OfficeConfig {
    fn default() -> Self {
        Self { conversion_timeout: 30, kill_timeout: 300, max_processes: 0 }
    }
}

impl OfficeConfig {
    pub(super) fn validate(&self) -> Result<()> {
        ensure!(
            self.kill_timeout >= self.conversion_timeout,
            "office.kill_timeout must not be lower than office.conversion_timeout"
        );

        Ok(())
    }

    pub fn process_limit(&self) -> usize {
        match self.max_processes {
            0 => available_parallelism().map_or(1, |threads| threads.get()),
            limit => limit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kill_timeout_must_cover_the_request_timeout() {
        assert!(OfficeConfig::default().validate().is_ok());
        assert!(OfficeConfig { conversion_timeout: 30, kill_timeout: 10, ..Default::default() }.validate().is_err());
    }

    #[test]
    fn zero_processes_means_one_per_cpu_thread() {
        let threads = available_parallelism().unwrap().get();
        assert_eq!(OfficeConfig { max_processes: 0, ..Default::default() }.process_limit(), threads);
        assert_eq!(OfficeConfig { max_processes: 3, ..Default::default() }.process_limit(), 3);
    }
}
