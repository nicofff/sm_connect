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
    feedback: OperationFeedback,
}

/// Banner state shown at the bottom of the config screen.
enum OperationFeedback {
    None,
    Success,
    Failure(String),
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
            feedback: OperationFeedback::None,
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

            match &self.feedback {
                OperationFeedback::Success => {
                    let line = Paragraph::new(Text::from("Operation successful"))
                        .centered()
                        .bg(Color::Green)
                        .block(Block::default().borders(Borders::ALL));
                    frame.render_widget(Clear, overlay_layout[1]);
                    frame.render_widget(line, overlay_layout[1]);
                }
                OperationFeedback::Failure(message) => {
                    let line = Paragraph::new(Text::from(message.clone()))
                        .centered()
                        .bg(Color::Red)
                        .block(Block::default().borders(Borders::ALL));
                    frame.render_widget(Clear, overlay_layout[1]);
                    frame.render_widget(line, overlay_layout[1]);
                }
                OperationFeedback::None => {}
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
                    Some(ConfigListOutputAction::Exit) => {
                        // Persist on exit. The persist gate rejects a both-modes-off
                        // state, so leaving is blocked (banner shown) until the user
                        // re-enables a mode. Any IO error surfaces the same way.
                        match self.config.lock().unwrap().persist() {
                            Ok(()) => return Ok(Self::Outcome::Exit),
                            Err(e) => self.feedback = OperationFeedback::Failure(e.to_string()),
                        }
                    }
                    Some(ConfigListOutputAction::ReturnConfig(option)) => match option {
                        ConfigOption::ResetRecent => {
                            History::reset().context("Failed to reset history")?;
                            self.feedback = OperationFeedback::Success;
                        }
                        ConfigOption::SetRecentTimeout => {
                            self.modifying_action = Some(ConfigOption::SetRecentTimeout);
                            self.input_active = true;
                            let current_value = self.config.lock().unwrap().get_recent_timeout();
                            self.input_component.set_value(current_value.to_string());
                        }
                        ConfigOption::ToggleEc2 => {
                            // Toggling can't fail; both modes may be off transiently
                            // (enforced on exit). Clear any stale banner.
                            self.config.lock().unwrap().toggle_ec2();
                            self.feedback = OperationFeedback::None;
                        }
                        ConfigOption::ToggleEcs => {
                            self.config.lock().unwrap().toggle_ecs();
                            self.feedback = OperationFeedback::None;
                        }
                    },
                    None => {
                        self.feedback = OperationFeedback::None;
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
                                self.config.lock().unwrap().set_recent_timeout(timeout);
                                self.feedback = OperationFeedback::Success;
                            } else {
                                self.feedback = OperationFeedback::Failure(
                                    "Timeout must be a whole number of seconds".to_string(),
                                );
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
