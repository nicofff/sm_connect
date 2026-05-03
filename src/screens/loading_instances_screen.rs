use std::{io::Stdout, time::Duration};

use anyhow::{Context, Result};
use crossterm::event;
use ratatui::{Terminal, prelude::CrosstermBackend};
use tokio::sync::oneshot;

use crate::{
    aws::InstanceInfo,
    components::{Component, loader::{Loader, LoaderOutputAction}},
};

use super::Screen;

type FetchResult = Result<Vec<InstanceInfo>>;

pub struct LoadingInstancesScreen {
    loader: Loader<FetchResult>,
}

pub enum Outcome {
    Cancelled,
    InstancesFetched(Vec<InstanceInfo>),
}

impl LoadingInstancesScreen {
    pub fn new(region: String, demo: bool) -> Self {
        let (tx, rx) = oneshot::channel();
        let message = format!("Loading instances for region {}", region);
        tokio::spawn(async move {
            if demo {
                tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                let fake = vec![
                    crate::aws::InstanceInfo::new_demo(
                        aws_config::Region::new(region.clone()),
                        "prod-web-01".into(),
                        "i-0a1b2c3d4e5f00001".into(),
                        "54.210.12.34".into(),
                    ),
                    crate::aws::InstanceInfo::new_demo(
                        aws_config::Region::new(region.clone()),
                        "prod-api-01".into(),
                        "i-0a1b2c3d4e5f00002".into(),
                        "54.210.12.35".into(),
                    ),
                    crate::aws::InstanceInfo::new_demo(
                        aws_config::Region::new(region.clone()),
                        "prod-worker-01".into(),
                        "i-0a1b2c3d4e5f00003".into(),
                        "10.0.1.10".into(),
                    ),
                    crate::aws::InstanceInfo::new_demo(
                        aws_config::Region::new(region.clone()),
                        "staging-web-01".into(),
                        "i-0a1b2c3d4e5f00004".into(),
                        "34.205.67.89".into(),
                    ),
                    crate::aws::InstanceInfo::new_demo(
                        aws_config::Region::new(region),
                        "staging-api-01".into(),
                        "i-0a1b2c3d4e5f00005".into(),
                        "34.205.67.90".into(),
                    ),
                ];
                let _ = tx.send(Ok(fake));
            } else {
                let _ = tx.send(crate::aws::fetch_instances(aws_config::Region::new(region)).await);
            }
        });
        Self {
            loader: Loader::new(message, rx),
        }
    }
}

impl Screen<Outcome> for LoadingInstancesScreen {
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<Outcome> {
        loop {
            terminal.draw(|frame| self.loader.view(frame, frame.area()))?;
            let message = if event::poll(Duration::from_millis(50))? {
                self.loader.handle_event(event::read()?)
            } else {
                None
            };
            let action = self.loader.update(message)
                .context("Unexpected error while fetching instances")?;
            match action {
                Some(LoaderOutputAction::Exit) => return Ok(Outcome::Cancelled),
                Some(LoaderOutputAction::Return(data)) => {
                    return Ok(Outcome::InstancesFetched(
                        data.context("Failed to fetch instances. Check your AWS credentials.")?,
                    ))
                }
                None => {}
            }
        }
    }
}
