use std::io::Stdout;

use crossterm::event;
use ratatui::{
    Terminal,
    layout::{Constraint, Layout},
    prelude::CrosstermBackend,
    widgets::Clear,
};

use crate::{
    aws::InstanceInfo,
    components::{
        Component,
        header_tabs::{HeaderTabs, Tab},
        instance_table::{InstanceTable, InstanceTableOutputAction},
        text_input::{TextInput, TextInputOutputAction},
    },
};

use super::Screen;
use anyhow::Result;

pub struct InstanceSelectScreen {
    header_tabs_component: HeaderTabs,
    instance_table_component: InstanceTable,
    search_component: TextInput,
    search_active: bool,
}

pub enum InstanceSelectScreenOutcome {
    Exit,
    Connect(InstanceInfo),
    Tunnel(InstanceInfo),
    FileManager(InstanceInfo),
}

impl InstanceSelectScreen {
    pub fn new() -> InstanceSelectScreen {
        let instance_table_component = InstanceTable::new();
        let search_component = TextInput::default();
        let mut header_tabs_component = HeaderTabs::new();
        header_tabs_component.set_selected(Tab::Instances);
        Self {
            header_tabs_component,
            instance_table_component,
            search_component,
            search_active: false,
        }
    }

    pub fn set_instances(&mut self, instances: Vec<InstanceInfo>) {
        self.instance_table_component.set_instances(instances);
    }

    fn draw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        terminal.draw(|frame| {
            let layout = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .margin(0)
                .constraints([Constraint::Max(3), Constraint::Fill(1)].as_ref())
                .split(frame.area());
            self.header_tabs_component.view(frame, layout[0]);
            self.instance_table_component.view(frame, layout[1]);
            if self.search_active {
                let search_layout = Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints(vec![Constraint::Fill(1), Constraint::Max(3)])
                    .split(layout[1]);
                frame.render_widget(Clear, search_layout[1]); //this clears out the background
                //TODO: Since we are drawing on top, maybe give it some distinct style?
                self.search_component.view(frame, search_layout[1]);
            }
        })?;

        Ok(())
    }
}

impl Screen for InstanceSelectScreen {
    type Outcome = InstanceSelectScreenOutcome;
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<InstanceSelectScreenOutcome> {
        loop {
            self.draw(terminal)?;
            let event = event::read()?;
            if !self.search_active {
                let message = self.instance_table_component.handle_event(event);
                let action = self.instance_table_component.update(message)?;
                match action {
                    Some(InstanceTableOutputAction::Exit) => return Ok(InstanceSelectScreenOutcome::Exit),
                    Some(InstanceTableOutputAction::ReturnInstance(instance)) => {
                        return Ok(InstanceSelectScreenOutcome::Connect(instance));
                    }
                    Some(InstanceTableOutputAction::ReturnInstanceForTunnel(instance)) => {
                        return Ok(InstanceSelectScreenOutcome::Tunnel(instance));
                    }
                    Some(InstanceTableOutputAction::ReturnInstanceForFileManager(instance)) => {
                        return Ok(InstanceSelectScreenOutcome::FileManager(instance));
                    }
                    Some(InstanceTableOutputAction::Search) => {
                        self.search_active = true;
                    }
                    None => {}
                }
            } else {
                let message = self.search_component.handle_event(event);
                let action = self.search_component.update(message)?;
                match action {
                    Some(TextInputOutputAction::Exit) => {
                        self.search_active = false;
                    }
                    Some(TextInputOutputAction::Return(search)) => {
                        self.instance_table_component.apply_filter(search);
                        self.search_active = false;
                    }
                    Some(TextInputOutputAction::PartialReturn(search)) => {
                        self.instance_table_component.apply_filter(search);
                    }
                    Some(TextInputOutputAction::ReturnWithKeyUp) => {
                        self.instance_table_component.previous();
                        self.search_active = false;
                    }
                    Some(TextInputOutputAction::ReturnWithKeyDown) => {
                        self.instance_table_component.next();
                        self.search_active = false;
                    }
                    None => {}
                }
            }
        }
    }
}
