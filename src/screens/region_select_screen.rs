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

pub struct RegionSelectScreen {
    header_tabs_component: HeaderTabs,
    region_select_component: RegionList,
    loader_component: Loader<Result<Vec<crate::aws::InstanceInfo>>>,
    loader_tx: Option<oneshot::Sender<Result<Vec<crate::aws::InstanceInfo>>>>,
    is_waiting_for_instances: bool
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
        let (tx,rx) = oneshot::channel();
        let loader_component = Loader::new("Loading instances",rx);
        Self {
            header_tabs_component,
            region_select_component,
            loader_component,
            loader_tx: Some(tx),
            is_waiting_for_instances: false
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
            if self.is_waiting_for_instances {
                self.loader_component.view(frame, layout[1]);
            }
        })?;
        Ok(())
    }
}

impl Screen<Outcome> for RegionSelectScreen {
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<Outcome> {
        loop {
            self.draw(terminal)?;
            if self.is_waiting_for_instances {
                let message = if event::poll(Duration::from_millis(50))? {
                    self.loader_component.handle_event(event::read()?)
                } else {
                    None
                };
                let action = self.loader_component.update(message)?;
                match action {
                    Some(LoaderOutputAction::Exit) => return Ok(Outcome::Exit),
                    Some(LoaderOutputAction::Error) => return Ok(Outcome::Exit), // TODO: Error handling?
                    Some(LoaderOutputAction::Return(data)) => {return Ok(Outcome::InstancesFetched(data
                        .context("Failed to fetch instances. Check your AWS credentials.")?))},
                    None => {},
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
                        self.loader_component.set_message(format!("Loading instances for region {}",&region));
                        let cloned_region = region.clone();
                        // we should only enter here once, so this shouldn't panic
                        // TODO: enforce that.
                        let sender = self.loader_tx.take().unwrap(); 
                        tokio::spawn(async {
                            let fetched_instances = crate::aws::fetch_instances(aws_config::Region::new(cloned_region))
                                .await;
                            let _ = sender.send(fetched_instances);
                        });
                        self.is_waiting_for_instances = true;
                    }
                    Some(RegionListOutputAction::OpenConfig) => return Ok(Outcome::OpenConfig),
                    None => {}
                }
            }
            
        }
    }
}
