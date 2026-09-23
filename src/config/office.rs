use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OfficeConfig {
    pub conversion_timeout: u64,
    pub kill_timeout: u64,
}

impl Default for OfficeConfig {
    fn default() -> Self {
        Self { conversion_timeout: 30, kill_timeout: 300 }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kill_timeout_must_cover_the_request_timeout() {
        assert!(OfficeConfig::default().validate().is_ok());
        assert!(OfficeConfig { conversion_timeout: 30, kill_timeout: 10 }.validate().is_err());
    }
}
