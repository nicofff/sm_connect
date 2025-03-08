use crossterm::event::{Event, KeyCode};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::{Frame, text::Text};

use crate::components::Action;

use super::Component;
use anyhow::Result;
#[derive(Debug, Clone)]
pub struct TextInput {
    prompt: String,
    search_input: String,
    search_cursor_position: usize,
}

impl Default for TextInput {
    fn default() -> Self {
        TextInput {
            prompt: "Searching: ".into(),
            search_input: "".into(),
            search_cursor_position: 0,
        }
    }
}

impl TextInput {
    pub fn new(prompt: String) -> Self {
        TextInput {
            prompt,
            search_input: "".into(),
            search_cursor_position: 0,
        }
    }

    #[allow(dead_code)]
    pub fn get_cursor_position(&self) -> usize {
        self.search_cursor_position + self.prompt.len()
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.search_cursor_position.saturating_sub(1);
        self.search_cursor_position = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.search_cursor_position.saturating_add(1);
        self.search_cursor_position = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        self.search_input
            .insert(self.search_cursor_position, new_char);

        self.move_cursor_right();
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.search_cursor_position != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.search_cursor_position;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.search_input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.search_input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.search_input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.search_input = "".into();
        self.search_cursor_position = 0;
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.search_input.len())
    }

    #[allow(dead_code)]
    fn reset_cursor(&mut self) {
        self.search_cursor_position = 0;
    }

    pub fn get_value(&self) -> String {
        self.search_input.clone()
    }

    pub fn set_value(&mut self, value: String) {
        self.search_cursor_position = value.len();
        self.search_input = value;
    }

    fn get_widget(&self) -> Paragraph {
        let text: String = format!("{}{}", self.prompt, self.search_input);
        Paragraph::new(Text::from(text))
    }
}

pub enum TextInputMessage {
    Char(char),
    Backspace,
    Right,
    Left,
    Esc,
    Up,
    Down,
    Enter,
}

impl Component<TextInputMessage> for TextInput {
    fn update(&mut self, msg: Option<TextInputMessage>) -> Result<Option<Action>> {
        let Some(msg) = msg else {
            return Ok(None);
        };
        match msg {
            TextInputMessage::Char(c) => {
                self.enter_char(c);
                Ok(Some(Action::PartialReturn(self.get_value())))
            }
            TextInputMessage::Backspace => {
                self.delete_char();
                Ok(Some(Action::PartialReturn(self.get_value())))
            }
            TextInputMessage::Right => {
                self.move_cursor_right();
                Ok(None)
            }
            TextInputMessage::Left => {
                self.move_cursor_left();
                Ok(None)
            }
            TextInputMessage::Esc => {
                Ok(Some(Action::Exit))
            }
            TextInputMessage::Up => {
                Ok(Some(Action::ReturnWithKeyUp))
            }
            TextInputMessage::Down => {
                Ok(Some(Action::ReturnWithKeyDown))
            }
            TextInputMessage::Enter => {
                Ok(Some(Action::Return(self.get_value())))
            }
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(Color::Blue));
        let widget = self.get_widget().block(block);
        frame.render_widget(widget, area);
    }

    fn handle_event(&self, event: Event) -> Option<TextInputMessage> {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char(c) => Some(TextInputMessage::Char(c)),
                KeyCode::Backspace => Some(TextInputMessage::Backspace),
                KeyCode::Right => Some(TextInputMessage::Right),
                KeyCode::Left => Some(TextInputMessage::Left),
                KeyCode::Esc => Some(TextInputMessage::Esc),
                KeyCode::Up => Some(TextInputMessage::Up),
                KeyCode::Down => Some(TextInputMessage::Down),
                KeyCode::Enter => Some(TextInputMessage::Enter),
                _ => None,
            },
            _ => None,
        }
    }
}
