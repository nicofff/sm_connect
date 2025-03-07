use std::sync::Arc;
use std::sync::Mutex;

use crate::app::config::Config;

use super::{Action, Component, HandleAction, Render, RenderHelp, View};
use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, List, ListItem, ListState, Row, Table},
};
#[derive(Default, Debug, Clone)]
pub struct RegionList {
    state: ListState,
    items: Vec<String>,
    favorites: Vec<String>,
    config: Arc<Mutex<Config>>,
}

impl RegionList {
    pub fn new(config: Arc<Mutex<Config>>) -> RegionList {
        let mut state = ListState::default();
        state.select(Some(0));
        let unlocked = config.lock().unwrap();
        let items = unlocked.get_visible_regions();
        let favorites = unlocked.get_favorite_regions();
        RegionList {
            state,
            items,
            favorites,
            config: config.clone(),
        }
    }

    pub fn update_items(&mut self, items: Vec<String>) {
        self.items = items;
        if let Some(i) = self.state.selected_mut() {
            if *i >= self.items.len() {
                *i = self.items.len() - 1;
            }
        }
        self.sort_list();
    }

    pub fn set_favorites(&mut self, favorites: Vec<String>) {
        self.favorites = favorites;
        self.sort_list();
    }

    fn sort_list(&mut self) {
        self.items.sort_by(|a, b| {
            if self.favorites.contains(a) && !self.favorites.contains(b) {
                return std::cmp::Ordering::Less;
            }
            if !self.favorites.contains(a) && self.favorites.contains(b) {
                return std::cmp::Ordering::Greater;
            }
            a.cmp(b)
        });
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
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
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn current(&self) -> Option<String> {
        self.state.selected().map(|i| self.items[i].clone())
    }

    fn get_list(&self) -> List {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|i| {
                let prefix = if self.favorites.contains(i) {
                    "★"
                } else {
                    ""
                };
                ListItem::new(format!("{} {}", prefix, i))
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

    fn get_help(&self) -> Table {
        let rows = vec![
            Row::new(vec![
                Cell::from(Span::styled("'q' Exit", Style::default().fg(Color::White))),
                Cell::from(Span::styled("'h' Hide", Style::default().fg(Color::White))),
                Cell::from(Span::styled(
                    "'r' Reset regions",
                    Style::default().fg(Color::White),
                )),
            ]),
            Row::new(vec![
                Cell::from(Span::styled(
                    "'*' Toggle Favorite",
                    Style::default().fg(Color::White),
                )),
                Cell::from(Span::styled(
                    "'c' to open configuration",
                    Style::default().fg(Color::White),
                )),
            ]),
        ];
        Table::new(
            rows,
            vec![
                Constraint::Min(10),
                Constraint::Min(10),
                Constraint::Min(10),
                Constraint::Min(10),
            ],
        )
    }
}

pub enum RegionListEvent {
    Exit,
    HideRegion,
    OpenConfig,
    Reset,
    ToggleFavorite,
    Up,
    Down,
    Enter,
}

impl Component<RegionListEvent> for RegionList {
    fn handle_event(&self, event: Event) -> Option<RegionListEvent> {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => Some(RegionListEvent::Exit),
                KeyCode::Char('h') => Some(RegionListEvent::HideRegion),
                KeyCode::Char('r') => Some(RegionListEvent::Reset),
                KeyCode::Char('c') => Some(RegionListEvent::OpenConfig),
                KeyCode::Char('*') => Some(RegionListEvent::ToggleFavorite),
                KeyCode::Down => Some(RegionListEvent::Down),
                KeyCode::Up => Some(RegionListEvent::Up),
                KeyCode::Right | KeyCode::Enter => Some(RegionListEvent::Enter),
                _ => None,
            },
            _ => None,
        }
    }

    fn update(&mut self, msg: Option<RegionListEvent>) -> Result<Option<Action>> {
        let Some(msg) = msg else {
            return Ok(None);
        };
        match msg {
            RegionListEvent::Exit => Ok(Some(Action::Exit)),
            RegionListEvent::HideRegion => {
                let Some(region) = self.current() else {
                    return Ok(None);
                };
                let mut config = self.config.lock().unwrap();
                config.hide_region(region)?;
                Ok(None)
            },
            RegionListEvent::Reset => {
                let mut config = self.config.lock().unwrap();
                config.reset_hidden_regions()?;
                Ok(None)
            },
            RegionListEvent::OpenConfig => Ok(Some(Action::OpenConfig)),
            RegionListEvent::ToggleFavorite => {
                let Some(region) = self.current() else {
                    return Ok(None);
                };
                let mut config = self.config.lock().unwrap();
                config.toggle_favorite_region(region)?;
                Ok(None)
            },
            RegionListEvent::Down => {
                self.next();
                Ok(None)
            }
            RegionListEvent::Up => {
                self.previous();
                Ok(None)
            }
            RegionListEvent::Enter => match self.current() {
                Some(str) => Ok(Some(Action::ReturnRegion(str))),
                None => Ok(None),
            },
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let vertical_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Percentage(90), Constraint::Percentage(10)])
            .split(area);

        let list = self.get_list();
        frame.render_stateful_widget(list, vertical_layout[0], &mut self.state.clone());
        let help = self.get_help();
        frame.render_widget(help, vertical_layout[1]);
    }
}
