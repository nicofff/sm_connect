use crate::components::{Action, Component, HandleAction, Render, RenderHelp, View};
use crossterm::event::{Event, KeyCode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Row, Table},
    Frame,
};
use anyhow::Result;
#[derive(Debug, Clone, Copy)]
pub enum ConfigOption {
    ResetRecent,
    SetRecentTimeout,
}

impl From<ConfigOption> for String {
    fn from(option: ConfigOption) -> String {
        match option {
            ConfigOption::ResetRecent => "Reset Recent Instances".to_string(),
            ConfigOption::SetRecentTimeout => "Set Recent Timeout".to_string(),
        }
    }
}

const CONFIG_OPTIONS: [ConfigOption; 2] =
    [ConfigOption::ResetRecent, ConfigOption::SetRecentTimeout];
    
#[derive(Debug)]
pub struct ConfigList {
    state: ListState,
}

impl ConfigList {
    pub fn new() -> ConfigList {
        let mut state = ListState::default();
        state.select(Some(0));
        ConfigList { state }
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
}

impl HandleAction for ConfigList {
    fn handle_action(&mut self, action: Event) -> Result<Action> {
        match action {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => Ok(Action::Exit),
                KeyCode::Down => {
                    self.next();
                    Ok(Action::Noop)
                }
                KeyCode::Up => {
                    self.previous();
                    Ok(Action::Noop)
                }
                KeyCode::Right | KeyCode::Enter => match self.current() {
                    Some(option) => Ok(Action::ReturnConfig(option)),
                    None => Ok(Action::Noop),
                },
                _ => Ok(Action::Noop),
            },
            _ => Ok(Action::Noop),
        }
    }
}

#[allow(refining_impl_trait)]
impl View for ConfigList {
    fn get_widget(&self) -> List {
        let items: Vec<ListItem> = CONFIG_OPTIONS
            .iter()
            .map(|i| {
                let name: String = (*i).into();
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
}

impl Render for ConfigList {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let vertical_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(90),
                Constraint::Percentage(10),
            ])
            .split(area);

        let widget = self.get_widget();
        frame.render_stateful_widget(widget, vertical_layout[0], &mut self.state.clone());
        self.render_help(frame, vertical_layout[1]);
    }
}

impl RenderHelp for ConfigList {
    fn render_help(&mut self, frame: &mut Frame, area: Rect) {
        let rows = vec![Row::new(vec![Cell::from(Span::styled(
            "'q' Exit",
            Style::default().fg(Color::White),
        ))])];
        let table = Table::new(rows, vec![Constraint::Min(10)]);
        frame.render_widget(table, area);
    }
}


pub enum ConfigListMessage {
    Exit,
    Up,
    Down,
    Enter,
}

impl Component<ConfigListMessage> for ConfigList {
    fn update(&mut self, msg: Option<ConfigListMessage>) -> Result<Option<Action>> {
        match msg {
            Some(ConfigListMessage::Exit) => Ok(Some(Action::Exit)),
            Some(ConfigListMessage::Up) => {
                self.previous();
                Ok(None)
            }
            Some(ConfigListMessage::Down) => {
                self.next();
                Ok(None)
            }
            Some(ConfigListMessage::Enter) => match self.current() {
                Some(option) => Ok(Some(Action::ReturnConfig(option))),
                None => Ok(None),
            },
            None => Ok(None),
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.render(frame, area);
    }

    fn handle_event(&self,event: Event)-> Option<ConfigListMessage> {
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