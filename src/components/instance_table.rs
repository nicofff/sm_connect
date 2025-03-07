use crate::aws::InstanceInfo;
use crossterm::event::{Event, KeyCode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::Span,
    widgets::{Block, Borders, Cell, Row, Table, TableState},
    Frame,
};

use super::{Action, Component, HandleAction, Render, RenderHelp, View};
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

    pub fn with_items(items: Vec<InstanceInfo>) -> InstanceTable {
        let mut state = TableState::default();
        state.select(Some(0));
        InstanceTable {
            state,
            items: items.clone(),
            visible_items: items.clone(),
            filter: String::default(),
            recent_first: false,
        }
    }

    pub fn with_items_and_filter(items: Vec<InstanceInfo>, filter: String) -> InstanceTable {
        let mut table = InstanceTable::with_items(items);
        table.apply_filter(filter);
        table
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
        self.state.select(if self.visible_items.len() != 0 {Some(0)} else {None});

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
            a.get_name().cmp(&b.get_name())
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

    fn perform_key_action(&mut self, action: Option<&str>) -> Action {
        if !self.visible_items.is_empty() {
            match action {
                Some("nextItem") => self.next(),
                Some("previousItem") => self.previous(),
                Some("accessItem") => {
                    return match self.current() {
                        Some(item) => Action::ReturnInstance(item),
                        None => Action::Noop,
                    };
                }
                _ => {}
            }
            match self.current() {
                Some(item) => Action::Select(item),
                None => Action::Noop,
            }
        } else {
            Action::Noop
        }
    }
}

impl HandleAction for InstanceTable {
    fn handle_action(&mut self, action: Event) -> Result<Action> {
        let action = match action {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => Action::Exit,
                KeyCode::Down => self.perform_key_action(Some("nextItem")),
                KeyCode::Up => self.perform_key_action(Some("previousItem")),
                KeyCode::Right | KeyCode::Enter => self.perform_key_action(Some("accessItem")),
                KeyCode::Char('/') => Action::Search,
                KeyCode::Char('i') => Action::ToggleInfoPanel,
                KeyCode::Char('r') => {
                    self.recent_first = !self.recent_first;
                    self.sort_instances();
                    Action::Noop
                }
                _ => Action::Noop,
            },
            _ => Action::Noop,
        };
        Ok(action)
    }
}

#[allow(refining_impl_trait)]
impl View for InstanceTable {
    fn get_widget(&self) -> Table {
        let items: Vec<Row> = self
            .visible_items
            .iter()
            .cloned()
            .map(|i| {
                Row::new(vec![
                    Cell::from(i.get_name()),
                    Cell::from(i.get_instance_id()),
                    Cell::from(i.get_private_ip()),
                    Cell::from(i.get_public_ip()),
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
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ")
            .header(
                Row::new(vec!["Name", "InstanceId", "Private IP", "Public IP"])
                    .style(Style::default().add_modifier(Modifier::BOLD).underlined()),
            )
    }
}

impl Render for InstanceTable {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let vertical_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(90), Constraint::Percentage(10)])
            .split(area);

        let widget = self.get_widget();
        frame.render_stateful_widget(widget, vertical_layout[0], &mut self.state.clone());
        self.render_help(frame, vertical_layout[1]);
    }
}

impl RenderHelp for InstanceTable {
    fn render_help(&mut self, frame: &mut Frame, area: Rect) {
        let rows = vec![Row::new(vec![
            Cell::from(Span::styled(
                "'/' Search",
                Style::default().fg(Color::White),
            )),
            Cell::from(Span::styled("'q' Exit", Style::default().fg(Color::White))),
            Cell::from(Span::styled(
                "'i' Info Panel",
                Style::default().fg(Color::White),
            )),
            Cell::from(Span::styled(
                "'r' Show Recent First",
                Style::default().fg(Color::White),
            )),
        ])];
        let table = Table::new(
            rows,
            vec![
                Constraint::Min(10),
                Constraint::Min(10),
                Constraint::Min(10),
                Constraint::Min(10),
                Constraint::Min(10),
            ],
        );
        frame.render_widget(table, area);
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

impl Component<InstanceTableMessage> for InstanceTable {
    fn update(&mut self, msg: Option<InstanceTableMessage>) -> Result<Option<Action>> {
        let Some(msg) = msg else {
            return Ok(None);
        };
        match msg {
            InstanceTableMessage::Exit => Ok(Some(Action::Exit)),
            InstanceTableMessage::Up => {
                self.previous();
                Ok(None)
            }
            InstanceTableMessage::Down => {
                self.next();
                Ok(None)
            }
            InstanceTableMessage::Enter => {
                match self.current() {
                    Some(item) => Ok(Some(Action::ReturnInstance(item))),
                    None => Ok(None),
                }
            }
            InstanceTableMessage::Search => Ok(Some(Action::Search)),
            InstanceTableMessage::RecentFirst => {
                self.recent_first = !self.recent_first;
                self.sort_instances();
                Ok(None)
            }
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        self.render(frame, area);
    }
    
    fn handle_event(&self,event: Event)-> Option<InstanceTableMessage> {
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