use crossterm::event::{Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Row, Table, TableState, Cell},
};

use crate::sftp::FileInfo;
use super::{Component, get_help_styled};
use anyhow::Result;
use std::collections::HashSet;

pub struct FilePane {
    pub state: TableState,
    pub selected: HashSet<usize>,
    title: String,
}

pub enum FilePaneMessage {
    Up,
    Down,
    Enter,
    Select,
    Exit,
}

pub enum FilePaneOutputAction {
    Select,
    Exit,
}

impl FilePane {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            state: TableState::default(),
            selected: HashSet::new(),
            title: title.into(),
        }
    }

    pub fn set_item_count(&mut self, count: usize) {
        if count == 0 {
            self.state.select(None);
        } else if self.state.selected().is_none_or(|i| i >= count) {
            self.state.select(Some(0));
        }
    }

    pub fn next(&mut self, count: usize) {
        if count == 0 { return; }
        let i = self.state.selected().map_or(0, |i| {
            if i >= count - 1 { 0 } else { i + 1 }
        });
        self.state.select(Some(i));
    }

    pub fn previous(&mut self, count: usize) {
        if count == 0 { return; }
        let i = self.state.selected().map_or(0, |i| {
            if i == 0 { count - 1 } else { i - 1 }
        });
        self.state.select(Some(i));
    }

    pub fn current_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn toggle_selected(&mut self, index: usize) {
        if self.selected.contains(&index) {
            self.selected.remove(&index);
        } else {
            self.selected.insert(index);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    pub fn view_with_files(&mut self, frame: &mut Frame, area: Rect, files: &[FileInfo], active: bool) {
        let border_style = if active {
            Style::default().fg(Color::LightGreen)
        } else {
            Style::default().fg(Color::Gray)
        };

        let rows: Vec<Row> = files
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let marker = if self.selected.contains(&i) { "[*] " } else { "    " };
                let name = if f.is_dir {
                    format!("{marker}{}/", f.name)
                } else {
                    format!("{marker}{}", f.name)
                };
                let size = if f.is_dir {
                    String::from("<DIR>")
                } else {
                    format!("{}", f.size)
                };
                Row::new(vec![Cell::from(name), Cell::from(size)])
                    .style(if self.selected.contains(&i) {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    })
            })
            .collect();

        let table = Table::new(
            rows,
            [Constraint::Fill(1), Constraint::Min(10)],
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", self.title))
                .border_style(border_style),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

        frame.render_stateful_widget(table, area, &mut self.state);
    }

    fn get_help(&self) -> Table<'_> {
        let rows = vec![Row::new(vec![
            get_help_styled('^', "Up"),
            get_help_styled('v', "Down"),
            get_help_styled(' ', "Select"),
            get_help_styled('t', "Transfer"),
            get_help_styled('q', "Quit"),
        ])];
        Table::new(
            rows,
            vec![
                Constraint::Min(8),
                Constraint::Min(10),
                Constraint::Min(10),
                Constraint::Min(12),
                Constraint::Min(8),
            ],
        )
    }
}

impl Component for FilePane {
    type Message = FilePaneMessage;
    type OutputAction = FilePaneOutputAction;

    fn update(&mut self, msg: Option<FilePaneMessage>) -> Result<Option<Self::OutputAction>> {
        let Some(msg) = msg else { return Ok(None); };
        match msg {
            FilePaneMessage::Up => Ok(None), // cursor movement handled by screen with file count
            FilePaneMessage::Down => Ok(None),
            FilePaneMessage::Enter => Ok(None), // screen resolves the file from index
            FilePaneMessage::Select => {
                if let Some(i) = self.current_index() {
                    self.toggle_selected(i);
                    Ok(Some(FilePaneOutputAction::Select))
                } else {
                    Ok(None)
                }
            }
            FilePaneMessage::Exit => Ok(Some(FilePaneOutputAction::Exit)),
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // FilePane.view() is not used directly — call view_with_files() from the screen
        // so the screen can pass the current file list without storing it in the component.
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Max(1)])
            .split(area);
        let help = self.get_help();
        frame.render_widget(help, layout[1]);
    }

    fn handle_event(&self, event: Event) -> Option<Self::Message> {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => Some(FilePaneMessage::Up),
                KeyCode::Down | KeyCode::Char('j') => Some(FilePaneMessage::Down),
                KeyCode::Enter => Some(FilePaneMessage::Enter),
                KeyCode::Char(' ') => Some(FilePaneMessage::Select),
                KeyCode::Char('q') => Some(FilePaneMessage::Exit),
                _ => None,
            },
            _ => None,
        }
    }
}
