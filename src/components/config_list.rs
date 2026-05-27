use crate::app::config::Config;
use crate::components::Component;
use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Row, Table},
};
use std::sync::{Arc, Mutex};

use super::get_help_styled;
#[derive(Debug, Clone, Copy)]
pub enum ConfigOption {
    ResetRecent,
    SetRecentTimeout,
    ToggleEc2,
    ToggleEcs,
}

const CONFIG_OPTIONS: [ConfigOption; 4] = [
    ConfigOption::ResetRecent,
    ConfigOption::SetRecentTimeout,
    ConfigOption::ToggleEc2,
    ConfigOption::ToggleEcs,
];

#[derive(Debug)]
pub struct ConfigList {
    state: ListState,
    config: Arc<Mutex<Config>>,
}

impl ConfigList {
    pub fn new(config: Arc<Mutex<Config>>) -> ConfigList {
        let mut state = ListState::default();
        state.select(Some(0));
        ConfigList { state, config }
    }

    /// Display label for an option. The mode toggles show their current state.
    fn label(&self, option: ConfigOption) -> String {
        match option {
            ConfigOption::ResetRecent => "Reset Recent Instances".to_string(),
            ConfigOption::SetRecentTimeout => "Set Recent Timeout".to_string(),
            ConfigOption::ToggleEc2 => {
                let enabled = self.config.lock().unwrap().is_ec2_enabled();
                format!("EC2 mode: {}", if enabled { "enabled" } else { "disabled" })
            }
            ConfigOption::ToggleEcs => {
                let enabled = self.config.lock().unwrap().is_ecs_enabled();
                format!("ECS mode: {}", if enabled { "enabled" } else { "disabled" })
            }
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= CONFIG_OPTIONS.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    CONFIG_OPTIONS.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn current(&self) -> Option<ConfigOption> {
        self.state.selected().map(|i| CONFIG_OPTIONS[i])
    }

    fn get_list(&self) -> List<'_> {
        let items: Vec<ListItem> = CONFIG_OPTIONS
            .iter()
            .map(|i| {
                let name = self.label(*i);
                ListItem::new(name)
            })
            .collect();

        // Create a List from all list items and highlight the currently selected one
        List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::LightGreen)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ")
    }

    fn get_help(&self) -> Table<'_> {
        let rows = vec![Row::new(vec![get_help_styled('q', "Exit")])];
        Table::new(rows, vec![Constraint::Min(10)])
    }
}

pub enum ConfigListMessage {
    Exit,
    Up,
    Down,
    Enter,
}

pub enum ConfigListOutputAction {
    Exit,
    ReturnConfig(ConfigOption),
}

impl Component for ConfigList {
    type Message = ConfigListMessage;
    type OutputAction = ConfigListOutputAction;
    fn update(&mut self, msg: Option<ConfigListMessage>) -> Result<Option<Self::OutputAction>> {
        match msg {
            Some(ConfigListMessage::Exit) => Ok(Some(ConfigListOutputAction::Exit)),
            Some(ConfigListMessage::Up) => {
                self.previous();
                Ok(None)
            }
            Some(ConfigListMessage::Down) => {
                self.next();
                Ok(None)
            }
            Some(ConfigListMessage::Enter) => match self.current() {
                Some(option) => Ok(Some(ConfigListOutputAction::ReturnConfig(option))),
                None => Ok(None),
            },
            None => Ok(None),
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let vertical_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Max(1)])
            .split(area);

        let list = self.get_list();
        frame.render_stateful_widget(list, vertical_layout[0], &mut self.state.clone());
        let help = self.get_help();
        frame.render_widget(help, vertical_layout[1]);
    }

    fn handle_event(&self, event: Event) -> Option<Self::Message> {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => Some(ConfigListMessage::Exit),
                KeyCode::Down => Some(ConfigListMessage::Down),
                KeyCode::Up => Some(ConfigListMessage::Up),
                KeyCode::Right | KeyCode::Enter => Some(ConfigListMessage::Enter),
                _ => None,
            },
            _ => None,
        }
    }
}
