use std::collections::HashMap;

use anyhow::Result;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_ec2::{
    Client,
    types::{Filter, Instance},
};

use crate::history::History;

#[derive(Debug, Clone)]
pub struct InstanceInfo {
    region: Region,
    raw_instance_data: Instance,
    last_access: Option<u64>,
}

impl InstanceInfo {
    pub fn get_name(&self) -> String {
        self.get_tags()
            .entry("Name".to_string())
            .or_default()
            .to_owned()
    }

    pub fn get_tags(&self) -> HashMap<String, String> {
        let Some(ref tags) = self.raw_instance_data.tags else {
            return HashMap::new();
        };
        tags.iter()
            .map(|tag| {
                (
                    tag.key.clone().unwrap_or_default(),
                    tag.value.clone().unwrap_or_default(),
                )
            })
            .collect()
    }

    pub fn get_instance_id(&self) -> String {
        self.raw_instance_data
            .instance_id
            .clone()
            .unwrap_or_default()
    }

    pub fn get_public_ip(&self) -> String {
        self.raw_instance_data
            .public_ip_address
            .clone()
            .unwrap_or_default()
    }

    pub fn get_private_ip(&self) -> String {
        self.raw_instance_data
            .private_ip_address
            .clone()
            .unwrap_or_default()
    }

    pub fn get_region(&self) -> Region {
        self.region.clone()
    }

    #[allow(dead_code)]
    pub fn get_raw_instance_data(&self) -> Instance {
        self.raw_instance_data.clone()
    }

    pub fn get_last_access(&self) -> Option<u64> {
        self.last_access
    }
}

pub async fn fetch_instances(region: Region) -> Result<Vec<InstanceInfo>> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region.clone())
        .load()
        .await;
    let client = Client::new(&config);
    let filters = vec![
        Filter::builder()
            .set_name(Some("instance-state-name".to_string()))
            .set_values(Some(vec!["running".to_string()]))
            .build(),
    ];
    let result = client
        .describe_instances()
        .set_filters(Some(filters))
        .send()
        .await?;

    let binding = result.reservations.unwrap();
    let recents = History::read()?;
    let instances: Vec<InstanceInfo> = binding
        .iter()
        .flat_map(|reservation| reservation.instances.clone().unwrap())
        .map(|instance: Instance| {
            let cloned = instance.clone();
            let last_accessed = recents
                .get(&instance.instance_id.clone().unwrap_or_default())
                .map(|entry| entry.get_when());
            InstanceInfo {
                region: region.clone(),
                raw_instance_data: cloned,
                last_access: last_accessed,
            }
        })
        .collect();
    Ok(instances)
}
