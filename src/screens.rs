pub mod config_screen;
pub mod instance_select_screen;
pub mod region_select_screen;

use std::io::Stdout;

use anyhow::Result;
use ratatui::{Terminal, prelude::CrosstermBackend};

pub trait Screen<Outcome> {
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<Outcome>;
}
