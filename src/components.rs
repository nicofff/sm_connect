pub mod instance_details;
pub mod instance_table;
pub mod region_list;
pub mod text_input;
use crossterm::event::{Event, KeyCode};

use ratatui::widgets::Widget;

use crate::aws::InstanceInfo;

pub enum Action {
    Noop,
    Exit,
    Return(String),
    ReturnWithKey(KeyCode),
    ReturnInstance(InstanceInfo),
    PartialReturn(String),
    Search,
    ToggleInfoPanel,
    Select(InstanceInfo),
    Hide(String),
    Reset,
    ToggleFavorite(String),
}

pub trait HandleAction {
    fn handle_action(&mut self, action: Event) -> Action;
}

pub trait View {
    fn get_widget(&self) -> impl Widget;
}
