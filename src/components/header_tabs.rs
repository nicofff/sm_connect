use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{Block, Tabs},
};

use super::Component;

pub struct HeaderTabs {
    tabs: Vec<Tab>,
    selected: Option<Tab>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Region,
    Instances,
    Connection,
    Tasks,
    Container,
}

impl Tab {
    pub fn get_tab(&self) -> &'static str {
        match self {
            Tab::Region => "Region",
            Tab::Instances => "Instances",
            Tab::Connection => "Connection",
            Tab::Tasks => "Tasks",
            Tab::Container => "Container",
        }
    }

    /// The EC2 / Session Manager flow steps.
    pub fn ec2_flow() -> Vec<Tab> {
        vec![Tab::Region, Tab::Instances, Tab::Connection]
    }

    /// The ECS Exec flow steps.
    pub fn ecs_flow() -> Vec<Tab> {
        vec![Tab::Region, Tab::Tasks, Tab::Container]
    }
}

impl From<Tab> for Line<'static> {
    fn from(val: Tab) -> Self {
        Line::raw(val.get_tab())
    }
}

impl HeaderTabs {
    pub fn new(tabs: Vec<Tab>) -> Self {
        Self {
            tabs,
            selected: None,
        }
    }

    pub fn set_selected(&mut self, tab: Tab) {
        self.selected = Some(tab);
    }
}

pub enum HeaderTabMessage {}
pub enum HeaderTabOutputAction {}

impl Component for HeaderTabs {
    type Message = HeaderTabMessage;
    type OutputAction = HeaderTabOutputAction;
    fn update(
        &mut self,
        _msg: Option<HeaderTabMessage>,
    ) -> anyhow::Result<Option<Self::OutputAction>> {
        Ok(None)
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let selected_index = self
            .selected
            .and_then(|sel| self.tabs.iter().position(|t| *t == sel));
        let tabs = Tabs::new(self.tabs.clone())
            .block(Block::bordered())
            .style(Style::default().white())
            .highlight_style(Style::default().yellow())
            .select(selected_index);
        frame.render_widget(tabs, area);
    }

    fn handle_event(&self, _event: crossterm::event::Event) -> Option<Self::Message> {
        None
    }
}
