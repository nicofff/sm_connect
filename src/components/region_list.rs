use super::{Action, HandleAction, View};
use crossterm::event::{Event, KeyCode};
use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};

#[derive(Default, Debug, Clone)]
pub struct RegionList {
    state: ListState,
    items: Vec<String>,
    favorites: Vec<String>,
}

impl RegionList {
    pub fn with_items(items: Vec<String>) -> RegionList {
        let mut state = ListState::default();
        state.select(Some(0));
        RegionList {
            state,
            items,
            favorites: Vec::new(),
        }
    }

    pub fn update_items(&mut self, items: Vec<String>) {
        self.items = items;
        match self.state.selected_mut() {
            Some(i) => {
                if *i >= self.items.len() {
                    *i = self.items.len() - 1;
                }
            }
            None => {}
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

    pub fn get_state_mut(&mut self) -> &mut ListState {
        &mut self.state
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
}

impl HandleAction for RegionList {
    fn handle_action(&mut self, action: Event) -> Action {
        match action {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => Action::Exit,
                KeyCode::Char('h') => Action::Hide(self.current().unwrap()),
                KeyCode::Char('r') => Action::Reset,
                KeyCode::Char('*') => Action::ToggleFavorite(self.current().unwrap()),
                KeyCode::Down => {
                    self.next();
                    Action::Noop
                }
                KeyCode::Up => {
                    self.previous();
                    Action::Noop
                }
                KeyCode::Right | KeyCode::Enter => match self.current() {
                    Some(str) => Action::Return(str.to_owned()),
                    None => Action::Noop,
                },
                _ => Action::Noop,
            },
            _ => Action::Noop,
        }
    }
}

#[allow(refining_impl_trait)]
impl View for RegionList {
    fn get_widget(&self) -> List {
        
        let items: Vec<ListItem> = self.items
            .iter()
            .map(|i| {
                let prefix = if self.favorites.contains(i) { "★" } else { "" };
                ListItem::new(format!("{} {}",prefix,i))
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
