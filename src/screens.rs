pub mod region_select_screen;
pub mod config_screen;
pub mod instance_select_screen;

use std::io::Stdout;

use ratatui::{prelude::CrosstermBackend, Terminal};
use anyhow::Result;

pub trait Screen<Outcome> {
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<Outcome>;
}