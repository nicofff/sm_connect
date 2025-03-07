use std::io::Stdout;

use aws_config::Region;
use crossterm::event;
use ratatui::{layout::{Constraint, Layout}, prelude::CrosstermBackend, widgets::Clear, Terminal};

use crate::{aws::{fetch_instances, InstanceInfo}, components::{header_tabs::{HeaderTabs, Tab}, instance_table::InstanceTable, text_input::TextInput, Action, Component}};

use super::Screen;
use anyhow::Result;


pub struct InstanceSelectScreen {
    header_tabs_component: HeaderTabs,
    instance_table_component: InstanceTable,
    search_component: TextInput,
    search_active: bool,
}

pub enum Outcome {
    Exit,
    InstanceSelected(InstanceInfo),
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

    pub async fn with_region(&mut self, region: String) -> Result<()> {
        let instances = fetch_instances(Region::new(region)).await?;
        self.instance_table_component.set_instances(instances);
        Ok(())
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
                .constraints(vec![
                    Constraint::Percentage(90),
                    Constraint::Percentage(10),
                ])
                .split(layout[1]);
                frame.render_widget(Clear, search_layout[1]); //this clears out the background
                //TODO: Since we are drawing on top, maybe give it some distinct style?
                self.search_component.view(frame, search_layout[1]);
            }
        })?;
        
        Ok(())
    }
}

impl Screen<Outcome> for InstanceSelectScreen {
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<Outcome> {
        loop {
            self.draw(terminal)?;
            let event = event::read()?;
            if !self.search_active {
                let message = self.instance_table_component.handle_event(event);
                let action = self.instance_table_component.update(message)?;
                match action {
                    Some(Action::Exit) => return Ok(Outcome::Exit),
                    Some(Action::ReturnInstance(instance)) => return Ok(Outcome::InstanceSelected(instance)),
                    Some(Action::Search) => {
                        self.search_active = true;
                    }
                    _ => {}
                }
            } else {
                let message = self.search_component.handle_event(event);
                let action = self.search_component.update(message)?;
                match action {
                    Some(Action::Exit) => {
                        self.search_active = false;
                    }
                    Some(Action::Return(search)) => {
                        self.instance_table_component.apply_filter(search);
                        self.search_active = false;
                    }
                    Some(Action::PartialReturn(search)) => {
                        self.instance_table_component.apply_filter(search);
                    }
                    Some(Action::ReturnWithKeyUp) => {
                        self.instance_table_component.previous();
                        self.search_active = false;
                    }
                    Some(Action::ReturnWithKeyDown) => {
                        self.instance_table_component.next();
                        self.search_active = false;
                    }
                    _ => {}
                }
            }
            
        }
    }
}