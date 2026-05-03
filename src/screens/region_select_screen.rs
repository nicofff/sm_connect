use std::{
    io::Stdout,
    sync::{Arc, Mutex}, time::Duration,
};

use crossterm::event;
use ratatui::{
    Terminal,
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
};
use tokio::sync::oneshot;

use crate::{
    app::config::Config, aws::InstanceInfo, components::{
        Component, header_tabs::{HeaderTabs, Tab}, loader::{Loader, LoaderOutputAction}, region_list::{RegionList, RegionListOutputAction}
    }
};

use anyhow::{Context, Result};

use super::Screen;

type FetchResult = Result<Vec<InstanceInfo>>;

pub struct RegionSelectScreen {
    header_tabs_component: HeaderTabs,
    region_select_component: RegionList,
    loader_component: Option<Loader<FetchResult>>,
}

pub enum Outcome {
    Exit,
    OpenConfig,
    InstancesFetched(Vec<InstanceInfo>)
}

impl RegionSelectScreen {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        let region_select_component = RegionList::new(config.clone());
        let mut header_tabs_component = HeaderTabs::new();
        header_tabs_component.set_selected(Tab::Region);
        Self {
            header_tabs_component,
            region_select_component,
            loader_component: None,
        }
    }

    fn draw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        terminal.draw(|frame| {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints([Constraint::Max(3), Constraint::Fill(1)].as_ref())
                .split(frame.area());
            self.header_tabs_component.view(frame, layout[0]);
            self.region_select_component.view(frame, layout[1]);
            if let Some(loader) = self.loader_component.as_mut() {
                loader.view(frame, layout[1]);
            }
        })?;
        Ok(())
    }
}

impl Screen<Outcome> for RegionSelectScreen {
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<Outcome> {
        loop {
            self.draw(terminal)?;
            if let Some(loader) = self.loader_component.as_mut() {
                let message = if event::poll(Duration::from_millis(50))? {
                    loader.handle_event(event::read()?)
                } else {
                    None
                };
                let action = loader.update(message)
                    .context("Unexpected error while fetching instances")?;
                match action {
                    Some(LoaderOutputAction::Exit) => {
                        self.loader_component = None;
                    }
                    Some(LoaderOutputAction::Return(data)) => {
                        return Ok(Outcome::InstancesFetched(data
                            .context("Failed to fetch instances. Check your AWS credentials.")?))
                    }
                    None => {}
                }
            } else {
                let message = if event::poll(Duration::from_millis(50))? {
                    self.region_select_component.handle_event(event::read()?)
                } else {
                    None
                };
                let action = self.region_select_component.update(message)?;
                match action {
                    Some(RegionListOutputAction::Exit) => return Ok(Outcome::Exit),
                    Some(RegionListOutputAction::Return(region)) => {
                        let (tx, rx) = oneshot::channel();
                        let cloned_region = region.clone();
                        tokio::spawn(async move {
                            let _ = tx.send(crate::aws::fetch_instances(aws_config::Region::new(cloned_region)).await);
                        });
                        let loader = Loader::new(format!("Loading instances for region {}", region), rx);
                        self.loader_component = Some(loader);
                    }
                    Some(RegionListOutputAction::OpenConfig) => return Ok(Outcome::OpenConfig),
                    None => {}
                }
            }
        }
    }
}
