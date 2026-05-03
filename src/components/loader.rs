use crossterm::event::{Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};
use tokio::sync::oneshot::{Receiver, error::TryRecvError};
use std::time::Instant;

use crate::components::Component;

const SPINNER_CHARS: [char; 4] = ['|', '/', '-', '\\'];
const SPINNER_FRAME_DURATION_MS: u128 = 200;

pub struct Loader<T> {
    start_time: Instant,
    message: String,
    rx: Receiver<T>
}

impl<T> Loader<T> {
    pub fn new<U: Into<String>>(message: U, rx: Receiver<T>) -> Self {
        Self {
            start_time: Instant::now(),
            message: message.into(),
            rx
        }
    }

    pub fn set_message<U: Into<String>>(&mut self, message: U) {
        self.message = message.into()
    }
}

pub enum LoaderMessage {
    Cancel
}
pub enum LoaderOutputAction<T> {
    Return(T),
    Exit,
    Error
}

impl<T> Component for Loader<T> {
    type Message = LoaderMessage;
    type OutputAction = LoaderOutputAction<T>;
    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // Create a centered layout
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(7),
                Constraint::Fill(1),
            ])
            .split(area);

        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Min(40),
                Constraint::Fill(1),
            ])
            .split(vertical[1]);

        // Get elapsed time and create spinning indicator
        let elapsed = self.start_time.elapsed();
        let spinner_index =
            (elapsed.as_millis() / SPINNER_FRAME_DURATION_MS) as usize % SPINNER_CHARS.len();
        let spinner = SPINNER_CHARS[spinner_index];

        // Create the loading message with spinner
        let loading_text = format!("{} {}", spinner, self.message);

        // Create loading paragraph
        let loading_paragraph = Paragraph::new(loading_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Loading ")
                    .style(Style::default().fg(Color::Blue)),
            )
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(loading_paragraph, horizontal[1]);

        // Add elapsed time indicator and help text
        let elapsed_secs = elapsed.as_secs();
        let time_text = if elapsed_secs > 0 {
            format!("Elapsed: {}s", elapsed_secs)
        } else {
            "Starting...".to_string()
        };

        let info_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(horizontal[1]);

        let time_paragraph = Paragraph::new(time_text)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::Gray));

        let help_paragraph = Paragraph::new("Press ESC to cancel")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::DarkGray));

        frame.render_widget(time_paragraph, info_layout[1]);
        frame.render_widget(help_paragraph, info_layout[2]);
    }


    fn update(&mut self, msg: Option<Self::Message>) -> anyhow::Result<Option<Self::OutputAction>> {
        match msg {
            Some(LoaderMessage::Cancel) => return Ok(Some(LoaderOutputAction::Exit)),
            None => {}
        }
        match self.rx.try_recv() {
            Ok(waited_data) => Ok(Some(LoaderOutputAction::Return(waited_data))),
            Err(TryRecvError::Empty) => {
                Ok(None)
            },
            Err(TryRecvError::Closed) => {
                self.message = "There was an error loading the information".into(); // TODO: should we recieve the error message as a param?
                Ok(Some(LoaderOutputAction::Error))
            },
        }
    }

    fn handle_event(&self, event: crossterm::event::Event) -> Option<Self::Message> {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Esc => Some(LoaderMessage::Cancel),
                _ => None
                
            },
            _ => None,
        }
    }
}
