use std::{
    io::Stdout,
    sync::{Arc, Mutex},
};

use crate::{
    app::config::Config,
    components::{
        Component,
        config_list::{ConfigList, ConfigListOutputAction, ConfigOption},
        text_input::{TextInput, TextInputOutputAction},
    },
    history::History,
};
use ratatui::{
    Terminal,
    layout::{Constraint, Layout},
    prelude::CrosstermBackend,
    style::{Color, Stylize},
    text::Text,
    widgets::{Block, Borders, Clear, Paragraph},
};

use anyhow::{Context, Result};

use super::Screen;

pub struct ConfigScreen {
    config: Arc<Mutex<Config>>,
    config_list: ConfigList,
    input_component: TextInput,
    input_active: bool,
    modifying_action: Option<ConfigOption>,
    last_operation_success: Option<bool>,
}

pub enum ConfigScreenOutcome {
    Exit,
}

impl ConfigScreen {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        let config_list = ConfigList::new(config.clone());
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
                .constraints(vec![Constraint::Percentage(100)])
                .split(frame.area());
            self.config_list.view(frame, layout[0]);
            let overlay_layout = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints(vec![Constraint::Fill(1), Constraint::Max(3)])
                .split(layout[0]);
            if self.input_active {
                frame.render_widget(Clear, overlay_layout[1]);
                self.input_component.view(frame, overlay_layout[1]);
            }

            match self.last_operation_success {
                Some(true) => {
                    let line = Paragraph::new(Text::from("Operation successful"))
                        .centered()
                        .bg(Color::Green)
                        .block(Block::default().borders(Borders::ALL));
                    frame.render_widget(Clear, overlay_layout[1]);
                    frame.render_widget(line, overlay_layout[1]);
                }
                Some(false) => {
                    let line = Paragraph::new(Text::from("Operation failed"))
                        .centered()
                        .bg(Color::Red)
                        .block(Block::default().borders(Borders::ALL));
                    frame.render_widget(Clear, overlay_layout[1]);
                    frame.render_widget(line, overlay_layout[1]);
                }
                None => {}
            }
        })?;
        Ok(())
    }
}

impl Screen for ConfigScreen {
    type Outcome = ConfigScreenOutcome;
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<Self::Outcome> {
        loop {
            self.draw(terminal)?;
            let event = crossterm::event::read()?;
            if !self.input_active {
                let message = self.config_list.handle_event(event);
                let action = self.config_list.update(message)?;
                match action {
                    Some(ConfigListOutputAction::Exit) => return Ok(Self::Outcome::Exit),
                    Some(ConfigListOutputAction::ReturnConfig(option)) => match option {
                        ConfigOption::ResetRecent => {
                            History::reset().context("Failed to reset history")?;
                            self.last_operation_success = Some(true);
                        }
                        ConfigOption::SetRecentTimeout => {
                            self.modifying_action = Some(ConfigOption::SetRecentTimeout);
                            self.input_active = true;
                            let current_value = self.config.lock().unwrap().get_recent_timeout();
                            self.input_component.set_value(current_value.to_string());
                        }
                        ConfigOption::ToggleEc2 => {
                            // Success is silent (the list label flips); a refused
                            // toggle (would disable the last mode) shows the failure banner.
                            match self.config.lock().unwrap().toggle_ec2() {
                                Ok(()) => self.last_operation_success = None,
                                Err(_) => self.last_operation_success = Some(false),
                            }
                        }
                        ConfigOption::ToggleEcs => {
                            match self.config.lock().unwrap().toggle_ecs() {
                                Ok(()) => self.last_operation_success = None,
                                Err(_) => self.last_operation_success = Some(false),
                            }
                        }
                    },
                    None => {
                        self.last_operation_success = None;
                    }
                }
            } else {
                let message = self.input_component.handle_event(event);
                let action = self.input_component.update(message)?;
                match action {
                    Some(TextInputOutputAction::Exit) => {
                        self.input_active = false;
                    }
                    Some(TextInputOutputAction::Return(search)) => {
                        if let Some(ConfigOption::SetRecentTimeout) = self.modifying_action {
                            if let Ok(timeout) = search.parse::<u64>() {
                                self.config.lock().unwrap().set_recent_timeout(timeout)?;
                                self.last_operation_success = Some(true);
                            } else {
                                self.last_operation_success = Some(false);
                            }
                        }
                        self.input_active = false;
                    }
                    Some(TextInputOutputAction::PartialReturn(_))
                    | Some(TextInputOutputAction::ReturnWithKeyUp)
                    | Some(TextInputOutputAction::ReturnWithKeyDown)
                    | None => {}
                }
            }
        }
    }
}
