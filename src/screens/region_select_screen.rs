use std::{
    io::Stdout,
    sync::{Arc, Mutex},
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

pub enum Outcome {
    Exit,
    RegionSelected(String),
    OpenConfig,
}

impl RegionSelectScreen {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        let region_select_component = RegionList::new(config.clone());
        let mut header_tabs_component = HeaderTabs::new();
        header_tabs_component.set_selected(Tab::Region);
        Self {
            header_tabs_component,
            region_select_component,
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
        })?;
        Ok(())
    }
}

impl Screen<Outcome> for RegionSelectScreen {
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<Outcome> {
        loop {
            self.draw(terminal)?;
            let event = event::read()?;
            let message = self.region_select_component.handle_event(event);
            let action = self.region_select_component.update(message)?;
            match action {
                Some(RegionListOutputAction::Exit) => return Ok(Outcome::Exit),
                Some(RegionListOutputAction::Return(region)) => {
                    return Ok(Outcome::RegionSelected(region));
                }
                Some(RegionListOutputAction::OpenConfig) => return Ok(Outcome::OpenConfig),
                None => {}
            }
        }
    }
}
