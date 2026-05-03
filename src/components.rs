pub mod config_list;
pub mod header_tabs;
pub mod instance_table;
pub mod loading_screen;
pub mod region_list;
pub mod text_input;
use anyhow::Result;
use crossterm::event::Event;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Cell,
};

pub trait Component {
    type Message;
    type OutputAction;
    fn update(&mut self, msg: Option<Self::Message>) -> Result<Option<Self::OutputAction>>;
    fn view(&mut self, frame: &mut Frame, area: Rect);
    fn handle_event(&self, event: Event) -> Option<Self::Message>;
}

fn get_help_styled(c: char, message: &str) -> Cell {
    let line = Line::from(vec![
        Span::styled(
            format!(" {} ", c.to_string().to_ascii_uppercase()),
            Style::default().bg(Color::Gray).fg(Color::Black),
        ),
        Span::raw(" "),
        Span::styled(message, Style::default()),
    ]);
    Cell::from(line)
}
