use std::{io::Stdout, sync::{Arc, Mutex}};

use crate::{app::config::Config, components::{config_panel::config_list::{ConfigList, ConfigOption}, text_input::TextInput, Action, Component}, history::History};
use ratatui::{layout::{Constraint, Layout, Rect}, prelude::CrosstermBackend, style::{Color, Stylize}, text::Line, widgets::Clear, Frame, Terminal};

use anyhow::Result;

use super::Screen;

pub struct ConfigScreen {
    config: Arc<Mutex<Config>>,
    config_list: ConfigList,
    input_component: TextInput,
    input_active: bool,
    modifying_action: Option<ConfigOption>,
    last_operation_success: Option<bool>,
}

pub enum Outcome {
    Exit,
}

impl ConfigScreen {
    pub fn new(config:  Arc<Mutex<Config>>) -> Self {
        let config_list = ConfigList::new();
        let input_component = TextInput::new("Value: ".to_string());
        Self {
            config,
            config_list,
            input_component,
            input_active: false,
            modifying_action: None,
            last_operation_success: None,
        }
    }

    fn draw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        terminal.draw(|frame| {
            let layout = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .margin(0)
                .constraints(vec![
                    Constraint::Percentage(100),
                ])
                .split(frame.area());
            self.config_list.view(frame, layout[0]);
            let overlay_layout = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints(vec![
                    Constraint::Percentage(90),
                    Constraint::Percentage(10),
                ])
                .split(layout[0]);
            if self.input_active {
                frame.render_widget(Clear, overlay_layout[1]);
                self.input_component.view(frame, overlay_layout[1]);
            }

            match self.last_operation_success {
                Some(true) => {
                    let line = Line::from("Operation successful")
                        .centered()
                        .bg(Color::Green);
                    frame.render_widget(Clear, overlay_layout[1]);
                    frame.render_widget(line, overlay_layout[1]);
                }
                Some(false) => {
                    let line = Line::from("Operation failed").centered().bg(Color::Red);
                    frame.render_widget(Clear, overlay_layout[1]);
                    frame.render_widget(line, overlay_layout[1]);
                }
                None => {}
            }            
            
        })?;
        Ok(())
    }
}

impl Screen<Outcome> for ConfigScreen {
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<Outcome> {
        loop {
            self.draw(terminal)?;
            let event = crossterm::event::read()?;
            if !self.input_active {
                let message = self.config_list.handle_event(event);
                let action = self.config_list.update(message)?;
                match action {
                    Some(Action::Exit) => return Ok(Outcome::Exit),
                    Some(Action::ReturnConfig(option)) => {
                        match option {
                            ConfigOption::ResetRecent => match History::reset() {
                                Ok(_) => {
                                    self.last_operation_success = Some(true);
                                }
                                Err(_e) => {
                                    //eprintln!("Error resetting history: {:?}", e);
                                    self.last_operation_success = Some(false);
                                }
                            },
                            ConfigOption::SetRecentTimeout => {
                                self.modifying_action = Some(ConfigOption::SetRecentTimeout);
                                self.input_active = true;
                                let current_value = self.config.lock().unwrap().get_recent_timeout();
                                self.input_component.set_value(current_value.to_string());
                            }
                        }
                    }
                    None => {
                        self.last_operation_success = None;
                    }
                    _ => {}
                }
            } else {
                let message = self.input_component.handle_event(event);
                let action = self.input_component.update(message)?;
                match action {
                    Some(Action::Exit) => {
                        self.input_active = false;
                    }
                    Some(Action::Return(search)) => {
                        if let Some(ConfigOption::SetRecentTimeout) = self.modifying_action {
                            if let Ok(timeout) = search.parse::<u64>() {
                                self.config
                                    .lock()
                                    .unwrap()
                                    .set_recent_timeout(timeout)?;
                                self.last_operation_success = Some(true);
                            } else {
                                self.last_operation_success = Some(false);
                            }
                        }
                        self.input_active = false;
                    }
                    _ => {}
                }
            }
        }
    }
}