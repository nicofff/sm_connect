pub mod instance_table;
pub mod region_list;
pub mod text_input;
pub mod header_tabs;
pub mod config_list;
use config_list::ConfigOption;
use crossterm::event::Event;
use anyhow::Result;
use ratatui::{layout::Rect, Frame};

use crate::aws::InstanceInfo;

pub enum Action {
    Exit,
    Return(String),
    ReturnRegion(String),
    ReturnWithKeyUp,
    ReturnWithKeyDown,
    ReturnInstance(InstanceInfo),
    ReturnConfig(ConfigOption),
    OpenConfig,
    PartialReturn(String),
    Search
}

pub trait Component<Message> {
    fn update(&mut self, msg: Option<Message>) -> Result<Option<Action>>;
    fn view(&mut self, frame: &mut Frame, area: Rect);
    fn handle_event(&self,event: Event)-> Option<Message>;
}