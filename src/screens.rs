pub mod config_screen;
pub mod mode_select_screen;
pub mod file_manager_screen;
pub mod connecting_screen;
pub mod instance_select_screen;
pub mod loading_instances_screen;
pub mod loading_tasks_screen;
pub mod region_select_screen;
pub mod task_select_screen;
pub mod username_prompt_screen;
pub mod container_select_screen;

use std::io::Stdout;

use anyhow::Result;
use ratatui::{Terminal, prelude::CrosstermBackend};

pub trait Screen {
    type Outcome;
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<Self::Outcome>;
}
