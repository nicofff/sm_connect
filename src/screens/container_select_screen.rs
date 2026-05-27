use std::io::Stdout;

use crossterm::event;
use ratatui::{
    Terminal,
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
};

use crate::components::{
    Component,
    header_tabs::{HeaderTabs, Tab},
    simple_list::{SimpleList, SimpleListOutputAction},
};

use super::Screen;
use anyhow::Result;

pub struct ContainerSelectScreen {
    header_tabs_component: HeaderTabs,
    list: SimpleList,
}

pub enum ContainerSelectScreenOutcome {
    Exit,
    Return(String),
}

impl ContainerSelectScreen {
    pub fn new(containers: Vec<String>) -> Self {
        let mut header_tabs_component = HeaderTabs::new(Tab::ecs_flow());
        header_tabs_component.set_selected(Tab::Container);
        let list = SimpleList::new(containers);
        Self {
            header_tabs_component,
            list,
        }
    }
}

impl Screen for ContainerSelectScreen {
    type Outcome = ContainerSelectScreenOutcome;
    fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<ContainerSelectScreenOutcome> {
        loop {
            terminal.draw(|frame| {
                let layout = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)
                    .constraints([Constraint::Max(3), Constraint::Fill(1)].as_ref())
                    .split(frame.area());
                self.header_tabs_component.view(frame, layout[0]);
                self.list.view(frame, layout[1]);
            })?;
            let message = self.list.handle_event(event::read()?);
            match self.list.update(message)? {
                Some(SimpleListOutputAction::Exit) => {
                    return Ok(ContainerSelectScreenOutcome::Exit);
                }
                Some(SimpleListOutputAction::Return(container)) => {
                    return Ok(ContainerSelectScreenOutcome::Return(container));
                }
                None => {}
            }
        }
    }
}
