use crate::aws::InstanceInfo;

use crate::components::loading_screen::LoadingScreen;
use crate::screens::Screen;
use crate::screens::config_screen::ConfigScreen;
use crate::screens::instance_select_screen::InstanceSelectScreen;
use crate::screens::region_select_screen;
use crate::screens::region_select_screen::RegionSelectScreen;
use crate::ui::restore_terminal;
use crate::ui::setup_terminal;

use anyhow::Context;
use crossterm::event;
use ratatui::prelude::*;
use tokio::time::Duration;

const LOADING_POLL_DURATION_MS: u64 = 50;

use std::io::Stdout;

use anyhow::Result;
use thiserror::Error;

pub mod config;

#[derive(Debug, Clone)]
pub enum SelectedScreen {
    RegionSelect,
    LoadingInstances,
    InstanceSelect,
    Config,
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("User exited the application")]
    UserExit,
    #[error("Error fetching instances. Check your AWS credentials and try again.")]
    FetchInstanceError,
}

pub struct App {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    selected_screen: SelectedScreen,
    region_select_screen: RegionSelectScreen,
    instance_selection_screen: InstanceSelectScreen,
    config_screen: ConfigScreen,
    loading_screen: Option<LoadingScreen>,
    loading_task: Option<tokio::task::JoinHandle<Result<Vec<crate::aws::InstanceInfo>>>>,
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
            config_screen,
            loading_screen: None,
            loading_task: None,
        })
    }

    pub async fn run(&mut self) -> Result<InstanceInfo> {
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
                        region_select_screen::Outcome::RegionSelected(region) => {
                            self.selected_screen = SelectedScreen::LoadingInstances;
                            self.loading_screen = Some(LoadingScreen::new(format!(
                                "Loading instances for region: {}",
                                region
                            )));

                            // Start the async loading task
                            let region_clone = region.clone();
                            self.loading_task = Some(tokio::spawn(async move {
                                crate::aws::fetch_instances(aws_config::Region::new(region_clone))
                                    .await
                            }));
                        }
                        region_select_screen::Outcome::OpenConfig => {
                            self.selected_screen = SelectedScreen::Config;
                        }
                    }
                }
                SelectedScreen::LoadingInstances => {
                    // Draw the loading screen (this will show the animated spinner)
                    if let Some(ref loading_screen) = self.loading_screen {
                        self.terminal.draw(|frame| {
                            loading_screen.draw(frame, frame.area());
                        })?;
                    }

                    // Check for any input events (like ESC to cancel)
                    if let Ok(true) = event::poll(Duration::from_millis(LOADING_POLL_DURATION_MS)) {
                        if let Ok(event) = event::read()
                            && let event::Event::Key(key) = event
                            && key.code == event::KeyCode::Esc
                        {
                            // Cancel the loading task if it's running
                            if let Some(task) = self.loading_task.take() {
                                task.abort();
                            }
                            self.selected_screen = SelectedScreen::RegionSelect;
                            self.loading_screen = None;
                            continue;
                        }
                    } else {
                        // No input event, just continue to check if loading is done
                        // This sleep ensures we redraw the spinner regularly
                        tokio::time::sleep(Duration::from_millis(LOADING_POLL_DURATION_MS)).await;
                    }

                    // Check if the loading task is complete
                    if let Some(task) = &mut self.loading_task
                        && task.is_finished()
                        && let Some(task) = self.loading_task.take()
                    {
                        match task.await {
                            Ok(Ok(instances)) => {
                                self.instance_selection_screen.set_instances(instances);
                                self.selected_screen = SelectedScreen::InstanceSelect;
                                self.loading_screen = None;
                            }
                            Ok(Err(_)) | Err(_) => {
                                return Err(RuntimeError::FetchInstanceError.into());
                            }
                        }
                    }
                }
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
            Some(instance) => Ok(instance),
            None => Err(RuntimeError::UserExit.into()),
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
