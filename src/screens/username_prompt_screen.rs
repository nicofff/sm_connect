use std::io::Stdout;
use std::env;

use anyhow::Result;
use crossterm::event;
use ratatui::{
    Terminal,
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::components::{Component, text_input::{TextInput, TextInputOutputAction}};
use super::Screen;

pub struct UsernamePromptScreen {
    input: TextInput,
}

pub enum UsernamePromptScreenOutcome {
    Username(String),
    Cancel,
}

impl UsernamePromptScreen {
    pub fn new() -> Self {
        let mut input = TextInput::new("Username: ".to_string());
        if let Ok(user) = env::var("USER").or_else(|_| env::var("USERNAME")) {
            input.set_value(user);
        }
        Self { input }
    }

    fn draw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        terminal.draw(|frame| {
            let area = frame.area();

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

            let block = Block::default()
                .borders(Borders::ALL)
                .title(" SSH Username ")
                .style(Style::default().fg(Color::Blue));

            let inner = block.inner(horizontal[1]);
            frame.render_widget(block, horizontal[1]);

            let help = Paragraph::new("Enter to confirm · Esc to cancel")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray));

            let inner_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Fill(1), Constraint::Length(1)])
                .split(inner);

            self.input.view(frame, inner_layout[0]);
            frame.render_widget(help, inner_layout[1]);
        })?;
        Ok(())
    }
}

impl Screen for UsernamePromptScreen {
    type Outcome = UsernamePromptScreenOutcome;
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<UsernamePromptScreenOutcome> {
        loop {
            self.draw(terminal)?;
            let event = event::read()?;
            let msg = self.input.handle_event(event);
            match self.input.update(msg)? {
                Some(TextInputOutputAction::Return(value)) => {
                    let trimmed = value.trim().to_string();
                    if !trimmed.is_empty() {
                        return Ok(UsernamePromptScreenOutcome::Username(trimmed));
                    }
                }
                Some(TextInputOutputAction::Exit) => return Ok(UsernamePromptScreenOutcome::Cancel),
                _ => {}
            }
        }
    }
}
