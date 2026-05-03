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
    pub fn new(region: String) -> Self {
        let (tx, rx) = oneshot::channel();
        let message = format!("Loading instances for region {}", region);
        tokio::spawn(async move {
            let _ = tx.send(crate::aws::fetch_instances(aws_config::Region::new(region)).await);
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
