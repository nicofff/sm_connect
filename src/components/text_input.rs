use crossterm::event::{Event, KeyCode};
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;
use ratatui::{text::Text, Frame};

use crate::components::{Action, HandleAction, View};

use super::Render;

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
}

impl HandleAction for TextInput {
    fn handle_action(&mut self, action: Event) -> Action {
        match action {
            Event::Key(key) => {
                match key.code {
                    KeyCode::Char(c) => {
                        self.enter_char(c);
                        Action::PartialReturn(self.search_input.clone())
                    }
                    KeyCode::Backspace => {
                        self.delete_char();
                        Action::PartialReturn(self.search_input.clone())
                    }
                    KeyCode::Right => {
                        self.move_cursor_right();
                        Action::Noop
                    }
                    KeyCode::Left => {
                        self.move_cursor_left();
                        Action::Noop
                    }
                    KeyCode::Esc => {
                        //self.reset_cursor();
                        Action::Exit
                    }
                    KeyCode::Up | KeyCode::Down => {
                        //self.reset_cursor();
                        Action::ReturnWithKey(key.code)
                    }
                    KeyCode::Enter => {
                        //self.reset_cursor();
                        Action::Return(self.search_input.clone())
                    }
                    _ => Action::Noop,
                }
            }
            _ => Action::Noop,
        }
    }
}

#[allow(refining_impl_trait)]
impl View for TextInput {
    fn get_widget(&self) -> Paragraph {
        let text: String = format!("Searching: {}", self.search_input);
        Paragraph::new(Text::from(text))
    }
}

impl Render for TextInput {
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let widget = self.get_widget();
        frame.render_widget(widget, area);
    }
}
