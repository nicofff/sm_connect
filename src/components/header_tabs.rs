use ratatui::{style::{Style, Stylize}, text::Line, widgets::{Block, Borders, Tabs}};

use super::Component;

pub struct HeaderTabs {
    selected: Option<Tab>
}
#[derive(Debug, Clone, Copy)]
pub enum Tab {
    Region,
    Instances,
    Connection,
}

impl Tab {
    pub fn get_tab(&self) -> &'static str {
        match self {
            Tab::Region => "Region",
            Tab::Instances => "Instances",
            Tab::Connection => "Connection",
        }
    }

    pub fn get_index(&self) -> usize {
        match self {
            Tab::Region => 0,
            Tab::Instances => 1,
            Tab::Connection => 2,
        }
    }
}

impl Into<Line<'static>> for Tab {
    fn into(self) -> Line<'static> {
        Line::raw(self.get_tab())
    }
}

impl HeaderTabs {
    pub fn new() -> Self {
        Self { selected: None }
    }

    pub fn set_selected(&mut self, tab: Tab) {
        self.selected = Some(tab);
    }

    pub fn clear_selected(&mut self) {
        self.selected = None;
    }

    pub fn get_all_tabs(&self) -> Vec<Tab> {
        vec![Tab::Region, Tab::Instances, Tab::Connection]
    }
}

pub enum HeaderTabMessage {}

impl Component<HeaderTabMessage> for HeaderTabs {
    fn update(&mut self, _msg: Option<HeaderTabMessage>) -> anyhow::Result<Option<super::Action>> {
        Ok(None)
    }

    fn view(&mut self, frame: &mut ratatui::Frame, area: ratatui::prelude::Rect) {
        let tabs = Tabs::new(self.get_all_tabs())
        .block(Block::bordered())
        .style(Style::default().white())
        .highlight_style(Style::default().yellow())
        .select(self.selected.map(|tab| tab.get_index()));
        frame.render_widget(tabs, area);
    }

    fn handle_event(&self,_event: crossterm::event::Event)-> Option<HeaderTabMessage> {
        None
    }
}