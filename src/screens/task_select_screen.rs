use std::io::Stdout;

use crossterm::event;
use ratatui::{
    Terminal,
    layout::{Constraint, Layout},
    prelude::CrosstermBackend,
    widgets::Clear,
};

use crate::{
    aws::EcsTaskInfo,
    components::{
        Component,
        header_tabs::{HeaderTabs, Tab},
        task_table::{TaskTable, TaskTableOutputAction},
        text_input::{TextInput, TextInputOutputAction},
    },
};

use super::Screen;
use anyhow::Result;

pub struct TaskSelectScreen {
    header_tabs_component: HeaderTabs,
    task_table_component: TaskTable,
    search_component: TextInput,
    search_active: bool,
}

// Short-lived outcome enum built and matched immediately, so the size spread
// between the data and unit variants does not matter (mirrors the EC2 outcome enums).
#[allow(clippy::large_enum_variant)]
pub enum TaskSelectScreenOutcome {
    Exit,
    Exec(EcsTaskInfo),
}

impl TaskSelectScreen {
    pub fn new() -> TaskSelectScreen {
        let task_table_component = TaskTable::new();
        let search_component = TextInput::default();
        let mut header_tabs_component = HeaderTabs::new(Tab::ecs_flow());
        header_tabs_component.set_selected(Tab::Tasks);
        Self {
            header_tabs_component,
            task_table_component,
            search_component,
            search_active: false,
        }
    }

    pub fn set_tasks(&mut self, tasks: Vec<EcsTaskInfo>) {
        self.task_table_component.set_tasks(tasks);
    }

    fn draw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        terminal.draw(|frame| {
            let layout = Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .margin(0)
                .constraints([Constraint::Max(3), Constraint::Fill(1)].as_ref())
                .split(frame.area());
            self.header_tabs_component.view(frame, layout[0]);
            self.task_table_component.view(frame, layout[1]);
            if self.search_active {
                let search_layout = Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints(vec![Constraint::Fill(1), Constraint::Max(3)])
                    .split(layout[1]);
                frame.render_widget(Clear, search_layout[1]); //this clears out the background
                //TODO: Since we are drawing on top, maybe give it some distinct style?
                self.search_component.view(frame, search_layout[1]);
            }
        })?;

        Ok(())
    }
}

impl Screen for TaskSelectScreen {
    type Outcome = TaskSelectScreenOutcome;
    fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<TaskSelectScreenOutcome> {
        loop {
            self.draw(terminal)?;
            let event = event::read()?;
            if !self.search_active {
                let message = self.task_table_component.handle_event(event);
                let action = self.task_table_component.update(message)?;
                match action {
                    Some(TaskTableOutputAction::Exit) => return Ok(TaskSelectScreenOutcome::Exit),
                    Some(TaskTableOutputAction::ReturnTask(task)) => {
                        return Ok(TaskSelectScreenOutcome::Exec(task));
                    }
                    Some(TaskTableOutputAction::Search) => {
                        self.search_active = true;
                    }
                    None => {}
                }
            } else {
                let message = self.search_component.handle_event(event);
                let action = self.search_component.update(message)?;
                match action {
                    Some(TextInputOutputAction::Exit) => {
                        // Search text is intentionally kept so reopening (`/`) resumes
                        // the previous query; matches instance_select_screen behavior.
                        self.search_active = false;
                    }
                    Some(TextInputOutputAction::Return(search)) => {
                        self.task_table_component.apply_filter(search);
                        self.search_active = false;
                    }
                    Some(TextInputOutputAction::PartialReturn(search)) => {
                        self.task_table_component.apply_filter(search);
                    }
                    Some(TextInputOutputAction::ReturnWithKeyUp) => {
                        self.task_table_component.previous();
                        self.search_active = false;
                    }
                    Some(TextInputOutputAction::ReturnWithKeyDown) => {
                        self.task_table_component.next();
                        self.search_active = false;
                    }
                    None => {}
                }
            }
        }
    }
}
