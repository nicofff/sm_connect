use crossterm::event::{Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Row, Table},
};

use super::{Component, get_help_styled};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct SimpleList {
    state: ListState,
    items: Vec<String>,
}

impl SimpleList {
    pub fn new(items: Vec<String>) -> SimpleList {
        let mut state = ListState::default();
        if !items.is_empty() {
            state.select(Some(0));
        }
        SimpleList { state, items }
    }

    fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) if i >= self.items.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(0) | None => self.items.len() - 1,
            Some(i) => i - 1,
        };
        self.state.select(Some(i));
    }

    fn current(&self) -> Option<String> {
        self.state.selected().map(|i| self.items[i].clone())
    }

    fn get_list(&self) -> List<'_> {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|i| ListItem::new(i.clone()))
            .collect();
        List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(
                Style::default()
                    .bg(Color::LightGreen)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ")
    }

    fn get_help(&self) -> Table<'_> {
        let rows = vec![Row::new(vec![get_help_styled('q', "Exit")])];
        Table::new(rows, vec![Constraint::Min(10)])
    }
}

pub enum SimpleListMessage {
    Exit,
    Up,
    Down,
    Enter,
}

pub enum SimpleListOutputAction {
    Exit,
    Return(String),
}

impl Component for SimpleList {
    type Message = SimpleListMessage;
    type OutputAction = SimpleListOutputAction;

    fn update(&mut self, msg: Option<SimpleListMessage>) -> Result<Option<Self::OutputAction>> {
        match msg {
            Some(SimpleListMessage::Exit) => Ok(Some(SimpleListOutputAction::Exit)),
            Some(SimpleListMessage::Up) => {
                self.previous();
                Ok(None)
            }
            Some(SimpleListMessage::Down) => {
                self.next();
                Ok(None)
            }
            Some(SimpleListMessage::Enter) => match self.current() {
                Some(item) => Ok(Some(SimpleListOutputAction::Return(item))),
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
                KeyCode::Char('q') => Some(SimpleListMessage::Exit),
                KeyCode::Down => Some(SimpleListMessage::Down),
                KeyCode::Up => Some(SimpleListMessage::Up),
                KeyCode::Right | KeyCode::Enter => Some(SimpleListMessage::Enter),
                _ => None,
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_wraps_around() {
        let mut list = SimpleList::new(vec!["a".into(), "b".into()]);
        assert_eq!(list.current(), Some("a".to_string()));
        list.next();
        assert_eq!(list.current(), Some("b".to_string()));
        list.next(); // wraps to start
        assert_eq!(list.current(), Some("a".to_string()));
    }

    #[test]
    fn previous_wraps_around() {
        let mut list = SimpleList::new(vec!["a".into(), "b".into()]);
        list.previous(); // wraps to end
        assert_eq!(list.current(), Some("b".to_string()));
    }

    #[test]
    fn empty_list_has_no_current() {
        let list = SimpleList::new(vec![]);
        assert_eq!(list.current(), None);
    }
}
