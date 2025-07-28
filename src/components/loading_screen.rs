use std::time::Instant;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph},
};

pub struct LoadingScreen {
    start_time: Instant,
    message: String,
}

impl LoadingScreen {
    pub fn new(message: String) -> Self {
        Self {
            start_time: Instant::now(),
            message,
        }
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect) {
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
        let spinner_chars = ['|', '/', '-', '\\'];
        let spinner_index = (elapsed.as_millis() / 200) as usize % spinner_chars.len();
        let spinner = spinner_chars[spinner_index];

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
            .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));

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
}