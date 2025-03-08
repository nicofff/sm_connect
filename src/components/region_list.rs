use std::sync::Arc;
use std::sync::Mutex;

use crate::app::config::Config;

use super::get_help_styled;
use super::{Action, Component};
use anyhow::Result;
use crossterm::event::{Event, KeyCode};
use ratatui::text::Line;
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

    fn update_items(&mut self, items: Vec<String>) {
        self.items = items;
        if let Some(i) = self.state.selected_mut() {
            if *i >= self.items.len() {
                *i = self.items.len() - 1;
            }
        }
        self.sort_list();
    }

    fn toggle_favorite_region(&mut self) -> Result<()> {
        let Some(region) = self.current() else {
            return Ok(());
        };
        {
            let mut config = self.config.lock().unwrap();
            config.toggle_favorite_region(region)?;
            self.favorites = config.get_favorite_regions();
        }
        self.sort_list();
        Ok(())
    }

    fn hide_region(&mut self) -> Result<()> {
        let Some(region) = self.current() else {
            return Ok(());
        };
        let regions = {
            let mut config = self.config.lock().unwrap();
            config.hide_region(region)?;
            config.get_visible_regions()
        };
        self.update_items(regions);
        self.sort_list();
        Ok(())
    }

    fn reset_hidden_regions(&mut self) -> Result<()> {
        {
            let mut config = self.config.lock().unwrap();
            config.reset_hidden_regions()?;
            self.items = config.get_visible_regions();
        }
        self.sort_list();
        Ok(())
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
                ListItem::new(format!("{} {}", prefix, i)).style(Style::default())
            })
            .collect();

        // Create a List from all list items and highlight the currently selected one
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

    fn get_help(&self) -> Table {
        let rows = vec![
            Row::new(vec![
                get_help_styled('q', "Exit"),
                get_help_styled('h', "Hide"),
                get_help_styled('r', "Reset regions"),
            ]),
            Row::new(vec![
                get_help_styled('*', "Toggle Favorite"),
                get_help_styled('c', "Open Configuration"),
            ]),
        ];
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
                self.hide_region()?;
                Ok(None)
            }
            RegionListEvent::Reset => {
                self.reset_hidden_regions()?;
                Ok(None)
            }
            RegionListEvent::OpenConfig => Ok(Some(Action::OpenConfig)),
            RegionListEvent::ToggleFavorite => {
                self.toggle_favorite_region()?;
                Ok(None)
            }
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
            .constraints(vec![Constraint::Fill(1), Constraint::Max(2)])
            .split(area);

        let list = self.get_list();
        frame.render_stateful_widget(list, vertical_layout[0], &mut self.state.clone());
        let help = self.get_help();
        frame.render_widget(help, vertical_layout[1]);
    }
}
