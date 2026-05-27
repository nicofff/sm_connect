use std::collections::HashMap;

use anyhow::Result;
use aws_config::{BehaviorVersion, Region};
use aws_sdk_ec2::{
    Client,
    types::{Filter, Instance},
};
use aws_sdk_ecs::types::DesiredStatus;

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

/// Returns the final `/`-delimited segment of an ARN (cluster name, task id, or
/// `family:revision`). Returns the input unchanged when there is no `/`.
fn short_name_from_arn(arn: &str) -> String {
    arn.rsplit('/').next().unwrap_or(arn).to_string()
}

/// ECS task `group` is `service:<name>` for service-managed tasks. Returns the
/// service name, or an empty string for standalone tasks.
fn service_from_group(group: &str) -> String {
    group.strip_prefix("service:").unwrap_or("").to_string()
}

#[derive(Debug, Clone)]
pub struct EcsTaskInfo {
    region: Region,
    cluster: String,       // raw cluster ARN, passed to `--cluster`
    cluster_name: String,  // short name, for display + search
    task_arn: String,      // raw task ARN, passed to `--task`
    task_id: String,       // short id, for display + search
    task_definition: String,
    service: String,
    last_status: String,
    containers: Vec<String>,
}

impl EcsTaskInfo {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        region: Region,
        cluster_arn: String,
        task_arn: String,
        task_definition_arn: String,
        group: String,
        last_status: String,
        containers: Vec<String>,
    ) -> Self {
        EcsTaskInfo {
            region,
            cluster_name: short_name_from_arn(&cluster_arn),
            cluster: cluster_arn,
            task_id: short_name_from_arn(&task_arn),
            task_arn,
            task_definition: short_name_from_arn(&task_definition_arn),
            service: service_from_group(&group),
            last_status,
            containers,
        }
    }

    pub fn get_region(&self) -> Region {
        self.region.clone()
    }

    pub fn get_cluster(&self) -> &str {
        &self.cluster
    }

    pub fn get_cluster_name(&self) -> &str {
        &self.cluster_name
    }

    pub fn get_task_arn(&self) -> &str {
        &self.task_arn
    }

    pub fn get_task_id(&self) -> &str {
        &self.task_id
    }

    pub fn get_task_definition(&self) -> &str {
        &self.task_definition
    }

    pub fn get_service(&self) -> &str {
        &self.service
    }

    pub fn get_last_status(&self) -> &str {
        &self.last_status
    }

    pub fn get_containers(&self) -> &[String] {
        &self.containers
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

    let binding = result.reservations.unwrap_or_default();
    let recents = History::read()?;
    let instances: Vec<InstanceInfo> = binding
        .iter()
        .flat_map(|reservation| reservation.instances.clone().unwrap_or_default())
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

/// Enumerates all RUNNING ECS tasks across every cluster in the region.
///
/// Steps: list clusters -> for each cluster list RUNNING task ARNs (paginated) ->
/// describe those tasks in batches of <=100 to pull group/definition/status/containers.
pub async fn fetch_ecs_tasks(region: Region) -> Result<Vec<EcsTaskInfo>> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region.clone())
        .load()
        .await;
    let client = aws_sdk_ecs::Client::new(&config);

    // Enumerate clusters, following pagination (ListClusters returns <=100 per page).
    let mut cluster_arns: Vec<String> = Vec::new();
    let mut clusters_token: Option<String> = None;
    loop {
        let resp = client
            .list_clusters()
            .set_next_token(clusters_token)
            .send()
            .await?;
        cluster_arns.extend(resp.cluster_arns.unwrap_or_default());
        clusters_token = resp.next_token;
        if clusters_token.is_none() {
            break;
        }
    }

    let mut tasks: Vec<EcsTaskInfo> = Vec::new();

    for cluster_arn in cluster_arns {
        // Collect RUNNING task ARNs for this cluster, following pagination.
        let mut task_arns: Vec<String> = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            let resp = client
                .list_tasks()
                .cluster(&cluster_arn)
                .desired_status(DesiredStatus::Running)
                .set_next_token(next_token)
                .send()
                .await?;
            task_arns.extend(resp.task_arns.unwrap_or_default());
            next_token = resp.next_token;
            if next_token.is_none() {
                break;
            }
        }

        // describe_tasks accepts at most 100 task ARNs per call.
        for chunk in task_arns.chunks(100) {
            let described = client
                .describe_tasks()
                .cluster(&cluster_arn)
                .set_tasks(Some(chunk.to_vec()))
                .send()
                .await?;
            // `described.failures` is intentionally ignored: a task that stopped
            // between list_tasks and describe_tasks simply won't appear in the list.
            for task in described.tasks.unwrap_or_default() {
                let task_arn = task.task_arn.clone().unwrap_or_default();
                let task_definition_arn = task.task_definition_arn.clone().unwrap_or_default();
                let group = task.group.clone().unwrap_or_default();
                let last_status = task.last_status.clone().unwrap_or_default();
                let containers = task
                    .containers
                    .unwrap_or_default()
                    .into_iter()
                    .filter_map(|c| c.name)
                    .collect();
                tasks.push(EcsTaskInfo::new(
                    region.clone(),
                    cluster_arn.clone(),
                    task_arn,
                    task_definition_arn,
                    group,
                    last_status,
                    containers,
                ));
            }
        }
    }

    Ok(tasks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_name_from_arn_takes_last_segment() {
        assert_eq!(
            short_name_from_arn("arn:aws:ecs:us-east-1:123:cluster/my-cluster"),
            "my-cluster"
        );
        assert_eq!(
            short_name_from_arn("arn:aws:ecs:us-east-1:123:task/my-cluster/abc123"),
            "abc123"
        );
        // No slash: returns the input unchanged.
        assert_eq!(short_name_from_arn("plain"), "plain");
    }

    #[test]
    fn service_from_group_strips_service_prefix() {
        assert_eq!(service_from_group("service:web-api"), "web-api");
        // Standalone tasks have a non-service group -> empty.
        assert_eq!(service_from_group("family:batch-job"), "");
        assert_eq!(service_from_group(""), "");
    }

    #[test]
    fn ecs_task_info_new_parses_display_fields() {
        let task = EcsTaskInfo::new(
            Region::new("us-east-1"),
            "arn:aws:ecs:us-east-1:123:cluster/prod".to_string(),
            "arn:aws:ecs:us-east-1:123:task/prod/deadbeef".to_string(),
            "arn:aws:ecs:us-east-1:123:task-definition/web:7".to_string(),
            "service:web-api".to_string(),
            "RUNNING".to_string(),
            vec!["app".to_string(), "sidecar".to_string()],
        );
        assert_eq!(task.get_cluster_name(), "prod");
        assert_eq!(task.get_task_id(), "deadbeef");
        assert_eq!(task.get_task_definition(), "web:7");
        assert_eq!(task.get_service(), "web-api");
        assert_eq!(task.get_last_status(), "RUNNING");
        assert_eq!(task.get_containers(), &["app".to_string(), "sidecar".to_string()]);
        // The raw cluster / task ARNs are preserved for the CLI command.
        assert_eq!(task.get_cluster(), "arn:aws:ecs:us-east-1:123:cluster/prod");
        assert_eq!(task.get_task_arn(), "arn:aws:ecs:us-east-1:123:task/prod/deadbeef");
    }
}
