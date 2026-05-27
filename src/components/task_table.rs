use crate::aws::EcsTaskInfo;
use crossterm::event::{Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

use super::{Component, get_help_styled};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct TaskTable {
    pub state: TableState,
    items: Vec<EcsTaskInfo>,
    visible_items: Vec<EcsTaskInfo>,
    filter: String,
}

impl TaskTable {
    pub fn new() -> TaskTable {
        TaskTable {
            state: TableState::default(),
            items: vec![],
            visible_items: vec![],
            filter: String::default(),
        }
    }

    pub fn set_tasks(&mut self, tasks: Vec<EcsTaskInfo>) {
        self.items = tasks;
        self.apply_filter(self.filter.clone());
    }

    /// Case-insensitive substring match across cluster, task id, definition,
    /// service, and status.
    pub fn apply_filter(&mut self, filter: String) {
        self.filter = filter;
        let needle = self.filter.to_lowercase();
        self.visible_items = self
            .items
            .iter()
            .filter(|t| {
                let haystack = format!(
                    "{} {} {} {} {}",
                    t.get_cluster_name(),
                    t.get_task_id(),
                    t.get_task_definition(),
                    t.get_service(),
                    t.get_last_status(),
                )
                .to_lowercase();
                haystack.contains(&needle)
            })
            .cloned()
            .collect();
        self.state.select(if self.visible_items.is_empty() {
            None
        } else {
            Some(0)
        });
    }

    #[cfg(test)]
    pub fn visible_len(&self) -> usize {
        self.visible_items.len()
    }

    pub fn next(&mut self) {
        if self.visible_items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(i) if i >= self.visible_items.len() - 1 => 0,
            Some(i) => i + 1,
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        if self.visible_items.is_empty() {
            return;
        }
        let i = match self.state.selected() {
            Some(0) | None => self.visible_items.len() - 1,
            Some(i) => i - 1,
        };
        self.state.select(Some(i));
    }

    pub fn current(&self) -> Option<EcsTaskInfo> {
        self.state.selected().map(|i| self.visible_items[i].clone())
    }

    fn get_table(&self) -> Table<'_> {
        let items: Vec<Row> = self
            .visible_items
            .iter()
            .map(|t| {
                Row::new(vec![
                    Cell::from(t.get_cluster_name().to_string()),
                    Cell::from(t.get_task_id().to_string()),
                    Cell::from(t.get_task_definition().to_string()),
                    Cell::from(t.get_service().to_string()),
                    Cell::from(t.get_last_status().to_string()),
                    Cell::from(t.get_containers().len().to_string()),
                ])
                .height(1)
            })
            .collect();
        let widths = [
            Constraint::Percentage(18),
            Constraint::Percentage(22),
            Constraint::Percentage(22),
            Constraint::Percentage(20),
            Constraint::Percentage(10),
            Constraint::Percentage(8),
        ];
        Table::new(items, widths)
            .block(Block::default().borders(Borders::ALL))
            .row_highlight_style(
                Style::default()
                    .bg(Color::LightGreen)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ")
            .header(
                Row::new(vec![
                    "Cluster",
                    "Task ID",
                    "Task Definition",
                    "Service",
                    "Status",
                    "Containers",
                ])
                .style(Style::default().add_modifier(Modifier::BOLD).underlined()),
            )
    }

    fn get_help(&self) -> Table<'_> {
        let rows = vec![Row::new(vec![
            get_help_styled('q', "Exit"),
            get_help_styled('/', "Search"),
        ])];
        Table::new(rows, vec![Constraint::Min(10), Constraint::Min(10)])
    }
}

pub enum TaskTableMessage {
    Exit,
    Up,
    Down,
    Enter,
    Search,
}

pub enum TaskTableOutputAction {
    Exit,
    ReturnTask(EcsTaskInfo),
    Search,
}

impl Component for TaskTable {
    type Message = TaskTableMessage;
    type OutputAction = TaskTableOutputAction;

    fn update(&mut self, msg: Option<TaskTableMessage>) -> Result<Option<Self::OutputAction>> {
        let Some(msg) = msg else {
            return Ok(None);
        };
        match msg {
            TaskTableMessage::Exit => Ok(Some(TaskTableOutputAction::Exit)),
            TaskTableMessage::Up => {
                self.previous();
                Ok(None)
            }
            TaskTableMessage::Down => {
                self.next();
                Ok(None)
            }
            TaskTableMessage::Enter => match self.current() {
                Some(task) => Ok(Some(TaskTableOutputAction::ReturnTask(task))),
                None => Ok(None),
            },
            TaskTableMessage::Search => Ok(Some(TaskTableOutputAction::Search)),
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        let vertical_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Fill(1), Constraint::Max(1)])
            .split(area);
        let widget = self.get_table();
        frame.render_stateful_widget(widget, vertical_layout[0], &mut self.state.clone());
        let help = self.get_help();
        frame.render_widget(help, vertical_layout[1]);
    }

    fn handle_event(&self, event: Event) -> Option<Self::Message> {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => Some(TaskTableMessage::Exit),
                KeyCode::Down => Some(TaskTableMessage::Down),
                KeyCode::Up => Some(TaskTableMessage::Up),
                KeyCode::Right | KeyCode::Enter => Some(TaskTableMessage::Enter),
                KeyCode::Char('/') => Some(TaskTableMessage::Search),
                _ => None,
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws::EcsTaskInfo;
    use aws_config::Region;

    fn sample(service: &str) -> EcsTaskInfo {
        EcsTaskInfo::new(
            Region::new("us-east-1"),
            "arn:aws:ecs:us-east-1:123:cluster/prod".to_string(),
            format!("arn:aws:ecs:us-east-1:123:task/prod/{}", service),
            "arn:aws:ecs:us-east-1:123:task-definition/def:1".to_string(),
            format!("service:{}", service),
            "RUNNING".to_string(),
            vec!["app".to_string()],
        )
    }

    #[test]
    fn apply_filter_matches_service_substring() {
        let mut table = TaskTable::new();
        table.set_tasks(vec![sample("web-api"), sample("worker")]);
        table.apply_filter("web".to_string());
        assert_eq!(table.visible_len(), 1);
        assert_eq!(table.current().unwrap().get_service(), "web-api");
    }

    #[test]
    fn empty_filter_shows_all() {
        let mut table = TaskTable::new();
        table.set_tasks(vec![sample("web-api"), sample("worker")]);
        table.apply_filter(String::new());
        assert_eq!(table.visible_len(), 2);
    }
}
