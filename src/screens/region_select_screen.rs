use std::{
    io::Stdout,
    sync::{Arc, Mutex},
    time::Duration,
};

use crossterm::event;
use ratatui::{
    Terminal,
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
};

use crate::{
    app::config::Config,
    components::{
        Component,
        header_tabs::{HeaderTabs, Tab},
        region_list::{RegionList, RegionListOutputAction},
    },
};

use anyhow::Result;

use super::Screen;

pub struct RegionSelectScreen {
    header_tabs_component: HeaderTabs,
    region_select_component: RegionList,
}

pub enum RegionSelectScreenOutcome {
    Exit,
    OpenConfig,
    RegionSelected(String),
}

impl RegionSelectScreen {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        let region_select_component = RegionList::new(config.clone());
        let mut header_tabs_component = HeaderTabs::new(Tab::ec2_flow());
        header_tabs_component.set_selected(Tab::Region);
        Self {
            header_tabs_component,
            region_select_component,
        }
    }
}

impl Screen for RegionSelectScreen {
    type Outcome = RegionSelectScreenOutcome;
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<RegionSelectScreenOutcome> {
        loop {
            terminal.draw(|frame| {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)
                    .constraints([Constraint::Max(3), Constraint::Fill(1)].as_ref())
                    .split(frame.area());
                self.header_tabs_component.view(frame, layout[0]);
                self.region_select_component.view(frame, layout[1]);
            })?;
            let message = if event::poll(Duration::from_millis(50))? {
                self.region_select_component.handle_event(event::read()?)
            } else {
                None
            };
            match self.region_select_component.update(message)? {
                Some(RegionListOutputAction::Exit) => return Ok(RegionSelectScreenOutcome::Exit),
                Some(RegionListOutputAction::Return(region)) => return Ok(RegionSelectScreenOutcome::RegionSelected(region)),
                Some(RegionListOutputAction::OpenConfig) => return Ok(RegionSelectScreenOutcome::OpenConfig),
                None => {}
            }
        }
    }
}
