use anyhow::Result;
use home::home_dir;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_string_pretty as to_string};
use std::{
    collections::HashMap,
    io::{Read, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default, Serialize, Deserialize)]
struct RegionConfig {
    hidden: bool,
    favorite: bool,
}

// when it becomes stable as const , switch to Duration::from_days(7).as_secs();
// https://github.com/rust-lang/rust/issues/120301
const DEFAULT_RECENT_TIMEOUT: u64 = 60 * 60 * 24 * 7;

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    recent_timeout: u64,
    regions: HashMap<String, RegionConfig>,
    #[serde(default = "default_true")]
    ec2_enabled: bool,
    #[serde(default = "default_true")]
    ecs_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        let mut regions = HashMap::new();
        for region in DEFAULT_REGIONS {
            regions.insert(region.to_string(), RegionConfig::default());
        }
        Config {
            regions,
            recent_timeout: DEFAULT_RECENT_TIMEOUT,
            ec2_enabled: true,
            ecs_enabled: true,
        }
    }
}

const DEFAULT_REGIONS: &[&str] = &[
    "af-south-1",
    "ap-east-1",
    "ap-northeast-1",
    "ap-northeast-2",
    "ap-northeast-3",
    "ap-south-1",
    "ap-south-2",
    "ap-southeast-1",
    "ap-southeast-2",
    "ap-southeast-3",
    "ap-southeast-4",
    "ca-central-1",
    "ca-west-1",
    "eu-central-1",
    "eu-central-2",
    "eu-north-1",
    "eu-south-1",
    "eu-south-2",
    "eu-west-1",
    "eu-west-2",
    "eu-west-3",
    "il-central-1",
    "me-central-1",
    "me-south-1",
    "sa-east-1",
    "us-east-1",
    "us-east-2",
    "us-west-1",
    "us-west-2",
];

impl Config {
    pub fn new() -> Result<Arc<Mutex<Config>>> {
        let config_path = Config::get_config_path()?;

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(config_path)?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let mut config: Config = match from_str(&contents) {
            Ok(config) => config,
            Err(_) => {
                let config = Config::default();
                config.persist()?;
                config
            }
        };
        // Repair a hand-edited file that disabled both modes.
        if config.heal_disabled_modes() {
            config.persist()?;
        }
        Ok(Arc::new(Mutex::new(config)))
    }

    /// The single invariant gate, enforced on every write: at least one of the
    /// EC2 / ECS modes must remain enabled. Checked before any filesystem IO.
    fn ensure_valid(&self) -> Result<()> {
        if !self.ec2_enabled && !self.ecs_enabled {
            return Err(anyhow::anyhow!(
                "at least one mode (EC2 or ECS) must remain enabled"
            ));
        }
        Ok(())
    }

    pub fn persist(&self) -> Result<()> {
        self.ensure_valid()?;
        let config_path = Config::get_config_path()?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(config_path)?;
        file.write_all(to_string(&self)?.as_bytes())?;
        Ok(())
    }

    fn get_config_path() -> Result<PathBuf> {
        let Some(home_dir) = home_dir() else {
            return Result::Err(anyhow::anyhow!("Could not find home directory"));
        };
        Ok(home_dir.join(".sm_connect.json"))
    }

    pub fn get_visible_regions(&self) -> Vec<String> {
        self.regions
            .iter()
            .filter(|(_, region)| !region.hidden)
            .map(|(region, _)| region.to_string())
            .collect()
    }

    pub fn hide_region(&mut self, region: String) -> Result<()> {
        if let Some(region) = self.regions.get_mut(&region) {
            region.hidden = true;
        }
        self.persist()
    }

    pub fn reset_hidden_regions(&mut self) -> Result<()> {
        for region in self.regions.values_mut() {
            region.hidden = false;
        }
        self.persist()
    }

    pub fn get_favorite_regions(&self) -> Vec<String> {
        self.regions
            .iter()
            .filter(|(_, region)| region.favorite)
            .map(|(region, _)| region.to_string())
            .collect()
    }

    pub fn toggle_favorite_region(&mut self, region: String) -> Result<()> {
        if let Some(region) = self.regions.get_mut(&region) {
            region.favorite = !region.favorite;
        }
        self.persist()
    }
    #[allow(dead_code)]
    pub fn unset_favorite_region(&mut self, region: String) -> Result<()> {
        if let Some(region) = self.regions.get_mut(&region) {
            region.favorite = false;
        }
        self.persist()
    }
    #[allow(dead_code)]
    pub fn favorite_region(&mut self, region: String) -> Result<()> {
        if let Some(region) = self.regions.get_mut(&region) {
            region.favorite = true;
        }
        self.persist()
    }
    #[allow(dead_code)]
    pub fn get_recent_timeout(&self) -> u64 {
        self.recent_timeout
    }
    #[allow(dead_code)]
    pub fn set_recent_timeout(&mut self, timeout: u64) -> Result<()> {
        self.recent_timeout = timeout;
        self.persist()
    }

    pub fn is_ec2_enabled(&self) -> bool {
        self.ec2_enabled
    }

    pub fn is_ecs_enabled(&self) -> bool {
        self.ecs_enabled
    }

    /// Flips the EC2 flag and persists. If the write is rejected (would leave no
    /// enabled mode), the flip is reverted and the error propagated.
    pub fn toggle_ec2(&mut self) -> Result<()> {
        self.ec2_enabled = !self.ec2_enabled;
        if let Err(e) = self.persist() {
            self.ec2_enabled = !self.ec2_enabled;
            return Err(e);
        }
        Ok(())
    }

    /// Flips the ECS flag and persists. If the write is rejected (would leave no
    /// enabled mode), the flip is reverted and the error propagated.
    pub fn toggle_ecs(&mut self) -> Result<()> {
        self.ecs_enabled = !self.ecs_enabled;
        if let Err(e) = self.persist() {
            self.ecs_enabled = !self.ecs_enabled;
            return Err(e);
        }
        Ok(())
    }

    /// Repairs a hand-edited config that disabled both modes by re-enabling both.
    /// Returns true if a change was made (so the caller can persist the fix).
    fn heal_disabled_modes(&mut self) -> bool {
        if !self.ec2_enabled && !self.ecs_enabled {
            self.ec2_enabled = true;
            self.ecs_enabled = true;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_valid_rejects_both_disabled() {
        let mut c = Config::default();
        assert!(c.ensure_valid().is_ok()); // both enabled by default
        c.ec2_enabled = false;
        c.ecs_enabled = false;
        assert!(c.ensure_valid().is_err());
    }

    #[test]
    fn toggle_refuses_to_disable_last_mode_and_reverts() {
        let mut c = Config::default();
        c.ecs_enabled = false; // only EC2 left enabled (direct set, no IO)
        let result = c.toggle_ec2(); // would disable the last mode
        assert!(result.is_err());
        assert!(c.is_ec2_enabled()); // reverted in memory
    }

    #[test]
    fn heal_reenables_both_when_both_disabled() {
        let mut c = Config::default();
        c.ec2_enabled = false;
        c.ecs_enabled = false;
        assert!(c.heal_disabled_modes());
        assert!(c.is_ec2_enabled() && c.is_ecs_enabled());
    }

    #[test]
    fn heal_is_noop_when_a_mode_is_enabled() {
        let mut c = Config::default();
        c.ecs_enabled = false; // ec2 still enabled
        assert!(!c.heal_disabled_modes());
        assert!(c.is_ec2_enabled());
        assert!(!c.is_ecs_enabled());
    }
}
