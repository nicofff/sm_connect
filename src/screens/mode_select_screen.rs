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

const MODE_EC2: &str = "EC2 (Session Manager)";
const MODE_ECS: &str = "ECS (Exec)";

pub struct ModeSelectScreen {
    header_tabs_component: HeaderTabs,
    list: SimpleList,
}

pub enum ModeSelectScreenOutcome {
    Exit,
    Ec2,
    Ecs,
}

impl ModeSelectScreen {
    pub fn new() -> Self {
        let header_tabs_component = HeaderTabs::new(Tab::ec2_flow());
        let list = SimpleList::new(vec![MODE_EC2.to_string(), MODE_ECS.to_string()]);
        Self {
            header_tabs_component,
            list,
        }
    }
}

impl Screen for ModeSelectScreen {
    type Outcome = ModeSelectScreenOutcome;
    fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<ModeSelectScreenOutcome> {
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
                Some(SimpleListOutputAction::Exit) => return Ok(ModeSelectScreenOutcome::Exit),
                Some(SimpleListOutputAction::Return(choice)) => {
                    if choice == MODE_ECS {
                        return Ok(ModeSelectScreenOutcome::Ecs);
                    } else {
                        return Ok(ModeSelectScreenOutcome::Ec2);
                    }
                }
                None => {}
            }
        }
    }
}
