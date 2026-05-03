use crate::aws::InstanceInfo;

use crate::screens::Screen;
use crate::screens::config_screen::ConfigScreen;
use crate::screens::instance_select_screen::InstanceSelectScreen;
use crate::screens::region_select_screen;
use crate::screens::region_select_screen::RegionSelectScreen;
use crate::ui::restore_terminal;
use crate::ui::setup_terminal;

use anyhow::Context;
use ratatui::prelude::*;

use std::io::Stdout;

use anyhow::Result;

pub mod config;

#[derive(Debug, Clone)]
pub enum SelectedScreen {
    RegionSelect,
    InstanceSelect,
    Config,
}

pub struct App {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    selected_screen: SelectedScreen,
    region_select_screen: RegionSelectScreen,
    instance_selection_screen: InstanceSelectScreen,
    config_screen: ConfigScreen,
}

impl App {
    pub fn new() -> Result<Self> {
        let terminal = setup_terminal().context("setup failed")?;
        let config = config::Config::new()?;
        let region_select_screen = RegionSelectScreen::new(config.clone());
        let instance_selection_screen = InstanceSelectScreen::new();
        let config_screen = ConfigScreen::new(config.clone());
        Ok(App {
            terminal,
            selected_screen: SelectedScreen::RegionSelect,
            region_select_screen,
            instance_selection_screen,
            config_screen
        })
    }

    pub async fn run(&mut self) -> Result<Option<InstanceInfo>> {
        let mut should_exit = false;
        let mut return_value: Option<InstanceInfo> = None;
        loop {
            // render
            match self.selected_screen {
                SelectedScreen::RegionSelect => {
                    match self.region_select_screen.run(&mut self.terminal)? {
                        region_select_screen::Outcome::Exit => {
                            should_exit = true;
                        }
                        region_select_screen::Outcome::InstancesFetched(instances) => {
                            self.instance_selection_screen.set_instances(instances);
                            self.selected_screen = SelectedScreen::InstanceSelect;
                        }
                        region_select_screen::Outcome::OpenConfig => {
                            self.selected_screen = SelectedScreen::Config;
                        }
                    }
                },
                SelectedScreen::InstanceSelect => {
                    match self.instance_selection_screen.run(&mut self.terminal)? {
                        crate::screens::instance_select_screen::Outcome::Exit => {
                            self.selected_screen = SelectedScreen::RegionSelect;
                        }
                        crate::screens::instance_select_screen::Outcome::InstanceSelected(
                            instance_info,
                        ) => {
                            should_exit = true;
                            return_value = Some(instance_info);
                        }
                    }
                }
                SelectedScreen::Config => match self.config_screen.run(&mut self.terminal)? {
                    crate::screens::config_screen::Outcome::Exit => {
                        self.selected_screen = SelectedScreen::RegionSelect;
                    }
                },
            }

            if should_exit {
                break;
            }
        }
        match return_value {
            Some(instance) => Ok(Some(instance)),
            None => Ok(None),
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Err(e) = restore_terminal(&mut self.terminal) {
            eprintln!("Failed to restore terminal: {:?}", e);
        }
    }
}
