use crate::aws::InstanceInfo;
use crossterm::event::{Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

use super::{Component, get_help_styled};
use anyhow::Result;
#[derive(Debug, Clone)]
pub struct InstanceTable {
    pub state: TableState,
    items: Vec<InstanceInfo>,
    visible_items: Vec<InstanceInfo>,
    filter: String,
    recent_first: bool,
}

impl InstanceTable {
    pub fn new() -> InstanceTable {
        let state = TableState::default();
        InstanceTable {
            state,
            items: vec![],
            visible_items: vec![],
            filter: String::default(),
            recent_first: false,
        }
    }

    pub fn set_instances(&mut self, instances: Vec<InstanceInfo>) {
        self.items = instances.clone();
        self.visible_items = instances.clone();
        self.apply_filter(self.filter.clone());
    }

    pub fn apply_filter(&mut self, filter: String) {
        self.filter = filter;
        self.visible_items = self
            .items
            .iter()
            .filter(|instance| {
                instance
                    .get_name()
                    .to_lowercase()
                    .contains(&self.filter.to_lowercase())
            })
            .cloned()
            .collect();
        self.sort_instances();
        self.state.select(if !self.visible_items.is_empty() {
            Some(0)
        } else {
            None
        });
    }

    fn sort_instances(&mut self) {
        self.visible_items.sort_by(|a, b| {
            if self.recent_first {
                let a_last_access = a.get_last_access();
                let b_last_access = b.get_last_access();
                match (a_last_access, b_last_access) {
                    (None, None) => {}
                    (None, Some(_)) => return std::cmp::Ordering::Greater,
                    (Some(_), None) => return std::cmp::Ordering::Less,
                    (Some(a_time), Some(b_time)) => return b_time.cmp(&a_time),
                }
            }
            a.get_name().cmp(b.get_name())
        });
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.visible_items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.visible_items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn current(&self) -> Option<InstanceInfo> {
        self.state.selected().map(|i| self.visible_items[i].clone())
    }

    fn get_table(&self) -> Table<'_> {
        let items: Vec<Row> = self
            .visible_items
            .iter()
            .cloned()
            .map(|i| {
                Row::new(vec![
                    Cell::from(i.get_name().to_string()),
                    Cell::from(i.get_instance_id().to_string()),
                    Cell::from(i.get_private_ip().to_string()),
                    Cell::from(i.get_public_ip().to_string()),
                ])
                .style(if self.recent_first && i.get_last_access().is_some() {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                })
                .height(1)
            })
            .collect();
        let widths = [
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ];
        // Create a List from all list items and highlight the currently selected one
        Table::new(items, widths)
            .block(Block::default().borders(Borders::ALL))
            .row_highlight_style(
                Style::default()
                    .bg(Color::LightGreen)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ")
            .header(
                Row::new(vec!["Name", "InstanceId", "Private IP", "Public IP"])
                    .style(Style::default().add_modifier(Modifier::BOLD).underlined()),
            )
    }

    fn get_help(&self) -> Table<'_> {
        let rows = vec![Row::new(vec![
            get_help_styled('q', "Exit"),
            get_help_styled('/', "Search"),
            get_help_styled(
                'r',
                if self.recent_first {
                    "Ignore recent"
                } else {
                    "Recent First"
                },
            ),
        ])];
        
        Table::new(
            rows,
            vec![
                Constraint::Min(10),
                Constraint::Min(10),
                Constraint::Min(10),
            ],
        )
    }
}

pub enum InstanceTableMessage {
    Exit,
    Up,
    Down,
    Enter,
    Search,
    RecentFirst,
}
pub enum InstanceTableOutputAction {
    Exit,
    ReturnInstance(InstanceInfo),
    Search,
}

impl Component for InstanceTable {
    type Message = InstanceTableMessage;
    type OutputAction = InstanceTableOutputAction;
    fn update(&mut self, msg: Option<InstanceTableMessage>) -> Result<Option<Self::OutputAction>> {
        let Some(msg) = msg else {
            return Ok(None);
        };
        match msg {
            InstanceTableMessage::Exit => Ok(Some(InstanceTableOutputAction::Exit)),
            InstanceTableMessage::Up => {
                self.previous();
                Ok(None)
            }
            InstanceTableMessage::Down => {
                self.next();
                Ok(None)
            }
            InstanceTableMessage::Enter => match self.current() {
                Some(item) => Ok(Some(InstanceTableOutputAction::ReturnInstance(item))),
                None => Ok(None),
            },
            InstanceTableMessage::Search => Ok(Some(InstanceTableOutputAction::Search)),
            InstanceTableMessage::RecentFirst => {
                self.recent_first = !self.recent_first;
                self.sort_instances();
                Ok(None)
            }
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let vertical_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Max(1)])
            .split(area);

        let widget = self.get_table();
        frame.render_stateful_widget(widget, vertical_layout[0], &mut self.state.clone());
        let help = self.get_help();
        frame.render_widget(help, vertical_layout[1]);
    }

    fn handle_event(&self, event: Event) -> Option<Self::Message> {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => Some(InstanceTableMessage::Exit),
                KeyCode::Down => Some(InstanceTableMessage::Down),
                KeyCode::Up => Some(InstanceTableMessage::Up),
                KeyCode::Right | KeyCode::Enter => Some(InstanceTableMessage::Enter),
                KeyCode::Char('/') => Some(InstanceTableMessage::Search),
                KeyCode::Char('r') => Some(InstanceTableMessage::RecentFirst),
                _ => None,
            },
            _ => None,
        }
    }
}
