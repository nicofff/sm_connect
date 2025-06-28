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
    name: String,
    instance_id: String,
    public_ip: String,
    private_ip: String,
    last_access: Option<u64>,
}

impl From<(Instance, Region)> for InstanceInfo {
    fn from(val: (Instance, Region)) -> Self {
        let (instance, region) = val;
        let tags = InstanceInfo::get_tags_map(&instance);
        let name = tags.get("Name").unwrap_or(&"".to_string()).to_string();
        InstanceInfo {
            region,
            name,
            instance_id: instance.instance_id.unwrap_or_default(),
            public_ip: instance.public_ip_address.unwrap_or_default(),
            private_ip: instance.private_ip_address.unwrap_or_default(),
            last_access: None,
        }
    }
}

impl InstanceInfo {
    pub fn new(region: Region, instance_id: String) -> Self {
        InstanceInfo {
            region,
            name: String::new(),
            instance_id,
            public_ip: String::new(),
            private_ip: String::new(),
            last_access: None,
        }
    }

    fn get_tags_map(instance: &Instance) -> HashMap<String, String> {
        let Some(ref tags) = instance.tags else {
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

    pub fn get_region(&self) -> Region {
        self.region.clone()
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_instance_id(&self) -> &str {
        &self.instance_id
    }

    pub fn get_public_ip(&self) -> &str {
        &self.public_ip
    }

    pub fn get_private_ip(&self) -> &str {
        &self.private_ip
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
            let last_accessed = recents
                .get(&instance.instance_id.clone().unwrap_or_default())
                .map(|entry| entry.get_when());
            let mut instance_info: InstanceInfo = (instance, region.clone()).into();
            instance_info.last_access = last_accessed;
            instance_info
        })
        .collect();
    Ok(instances)
}
