use std::{io::Stdout, time::Duration};

use anyhow::{Context, Result};
use crossterm::event;
use ratatui::{Terminal, prelude::CrosstermBackend};
use tokio::sync::oneshot;

use crate::{
    aws::EcsTaskInfo,
    components::{
        Component,
        loader::{Loader, LoaderOutputAction},
    },
};

use super::Screen;

type FetchResult = Result<Vec<EcsTaskInfo>>;

pub struct LoadingTasksScreen {
    loader: Loader<FetchResult>,
}

pub enum LoadingTasksScreenOutcome {
    Cancelled,
    TasksFetched(Vec<EcsTaskInfo>),
}

impl LoadingTasksScreen {
    pub fn new(region: String) -> Self {
        let (tx, rx) = oneshot::channel();
        let message = format!("Loading ECS tasks for region {}", region);
        tokio::spawn(async move {
            let _ = tx.send(crate::aws::fetch_ecs_tasks(aws_config::Region::new(region)).await);
        });
        Self {
            loader: Loader::new(message, rx),
        }
    }
}

impl Screen for LoadingTasksScreen {
    type Outcome = LoadingTasksScreenOutcome;
    fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<LoadingTasksScreenOutcome> {
        loop {
            terminal.draw(|frame| self.loader.view(frame, frame.area()))?;
            let message = if event::poll(Duration::from_millis(50))? {
                self.loader.handle_event(event::read()?)
            } else {
                None
            };
            let action = self
                .loader
                .update(message)
                .context("Unexpected error while fetching ECS tasks")?;
            match action {
                Some(LoaderOutputAction::Exit) => return Ok(LoadingTasksScreenOutcome::Cancelled),
                Some(LoaderOutputAction::Return(data)) => {
                    return Ok(LoadingTasksScreenOutcome::TasksFetched(
                        data.context("Failed to fetch ECS tasks. Check your AWS credentials.")?,
                    ));
                }
                None => {}
            }
        }
    }
}
