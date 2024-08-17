use anyhow::Result;
use configparser::ini::Ini;
use home::home_dir;
use std::path::PathBuf;
#[derive(Default, Debug, Clone)]
pub(super) struct Config {
    internal: Ini,
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
    pub fn new() -> Config {
        let Ok(config_path) = Config::get_config_path() else {
            return Config {
                internal: Ini::new(),
            };
        };

        if config_path.exists() {
            let mut config = Ini::new();
            config.load(config_path).unwrap();
            Config { internal: config }
        } else {
            Config::default()
        }
    }

    pub fn persist(&self) {
        let Ok(config_path) = Config::get_config_path() else {
            return;
        };

        self.internal.write(config_path).unwrap();
    }

    fn get_config_path() -> Result<PathBuf> {
        let Some(home_dir) = home_dir() else {
            return Result::Err(anyhow::anyhow!("Could not find home directory"));
        };
        Ok(home_dir.join(".sm_connect"))
    }

    fn get_default_regions() -> Vec<String> {
        DEFAULT_REGIONS.iter().map(|s| s.to_string()).collect()
    }

    pub fn get_visible_regions(&self) -> Vec<String> {
        let Some(hidden_regions) = self.internal.get("regions", "hidden") else {
            return Self::get_default_regions();
        };
        let hidden_regions: Vec<String> =
            hidden_regions.split(",").map(|s| s.to_string()).collect();
        Self::get_default_regions()
            .into_iter()
            .filter(|r| !hidden_regions.contains(r))
            .collect()
    }

    pub fn hide_region(&mut self, region: String) {
        match self.internal.get("regions", "hidden") {
            Some(hidden_regions) => {
                self.internal.set(
                    "regions",
                    "hidden",
                    format!("{},{}", hidden_regions, region).into(),
                );
            }
            None => {
                self.internal.set("regions", "hidden", region.into());
            }
        }
        self.persist();
    }

    pub fn reset_hidden_regions(&mut self) {
        self.internal
            .set("regions", "hidden", "".to_string().into());
        self.persist();
    }

    pub fn get_favorite_regions(&self) -> Vec<String> {
        let Some(visible_regions) = self.internal.get("regions", "favorite") else {
            return Vec::new();
        };
        visible_regions.split(",").map(|s| s.to_string()).collect()
    }

    pub fn toggle_favorite_region(&mut self, region: String) {
        let favorite_regions = self.get_favorite_regions();
        if favorite_regions.contains(&region) {
            self.unset_favorite_region(region)
        } else {
            self.favorite_region(region);
        }
        self.persist();
    }

    pub fn unset_favorite_region(&mut self, region: String) {
        let favorite_regions = self.get_favorite_regions();
        if favorite_regions.contains(&region) {
            let new_favs: Vec<String> = favorite_regions
                .iter()
                .filter(|r| r != &&region)
                .map(|s| s.to_string())
                .collect();
            self.internal.set(
                "regions",
                "favorite",
                new_favs.join(",").to_string().into(),
            );
        }
    }

    pub fn favorite_region(&mut self, region: String) {
        match self.internal.get("regions", "favorite") {
            Some(favorite_regions) => {
                self.internal.set(
                    "regions",
                    "favorite",
                    format!("{},{}", favorite_regions, region).into(),
                );
            }
            None => {
                self.internal.set("regions", "favorite", region.into());
            }
        }
    }
}
