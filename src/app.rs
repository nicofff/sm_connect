use crate::aws::fetch_instances;
use crate::aws::InstanceInfo;
use crate::components::config_panel::ConfigPanel;
use crate::components::instance_details::InstanceDetails;
use crate::components::region_list::RegionList;
use crate::components::Component;
use crate::components::{Action, HandleAction, Render};

use crate::screens::config_screen::ConfigScreen;
use crate::screens::instance_select_screen::InstanceSelectScreen;
use crate::screens::region_select_screen;
use crate::screens::region_select_screen::RegionSelectScreen;
use crate::screens::Screen;
use crate::ui::restore_terminal;
use crate::ui::setup_terminal;

use anyhow::Context;
use aws_config::Region;
use crossterm::event::Event;
use crossterm::event::{self};

use ratatui::style::Style;
use ratatui::{prelude::*, widgets::*};

use std::io::Stdout;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use thiserror::Error;

pub mod config;

#[derive(Debug, Clone)]
pub enum SelectedScreen {
    RegionSelect,
    InstanceSelect,
    Config,
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("User exited the application")]
    UserExit,
    #[error("Debugging, region {0}")]
    Region(String),
    #[error("Error fetching instances. Check your AWS credentials and try again.")]
    FetchInstanceError,
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
            config_screen,
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
                            self.selected_screen = SelectedScreen::InstanceSelect;
                            match self.instance_selection_screen.with_region(region).await {
                                Ok(_) => {},
                                Err(_e) => {
                                    return Err(RuntimeError::FetchInstanceError.into());
                                }
                            }

                        }
                        region_select_screen::Outcome::OpenConfig => {
                            self.selected_screen = SelectedScreen::Config;
                        }
                    }
                }
                SelectedScreen::InstanceSelect => {
                    match self.instance_selection_screen.run(&mut self.terminal)? {
                        crate::screens::instance_select_screen::Outcome::Exit => {
                            self.selected_screen = SelectedScreen::RegionSelect;
                        },
                        crate::screens::instance_select_screen::Outcome::InstanceSelected(instance_info) => {
                            should_exit = true;
                            return_value = Some(instance_info);
                        },
                    }
                }
                SelectedScreen::Config => {
                    match self.config_screen.run(&mut self.terminal)? {
                        crate::screens::config_screen::Outcome::Exit => {
                            self.selected_screen = SelectedScreen::RegionSelect;
                        }
                    }
                }
            }
            // handle events
            // let event = event::read()?;
            // match self.status {
            //     SelectedScreen::RegionSelect => {
            //         let action = self.region_select_component.handle_action(event)?;
            //         match action {
            //             Action::Exit => {
            //                 should_exit = true;
            //             }
            //             Action::Return(region) => {
            //                 self.status = SelectedScreen::InstanceSelect;
            //                 let instances = fetch_instances(Region::new(region)).await?;
            //                 self.instance_selection_component
            //                     .update_instances(instances);
            //             }
            //             Action::Hide(region) => {
            //                 let mut config = self.config.lock().unwrap();
            //                 config.hide_region(region)?;
            //                 self.region_select_component
            //                     .update_items(config.get_visible_regions());
            //             }
            //             Action::Reset => {
            //                 let mut config = self.config.lock().unwrap();
            //                 config.reset_hidden_regions()?;
            //                 self.region_select_component
            //                     .update_items(config.get_visible_regions());
            //             }
            //             Action::ToggleFavorite(region) => {
            //                 let mut config = self.config.lock().unwrap();
            //                 config.toggle_favorite_region(region)?;
            //                 self.region_select_component
            //                     .set_favorites(config.get_favorite_regions());
            //             }
            //             Action::OpenConfig => {
            //                 self.status = SelectedScreen::Config;
            //             }
            //             _ => {}
            //         }
            //     }
            //     SelectedScreen::InstanceSelect => {
            //         let action = self.instance_selection_component.handle_action(event)?;
            //         match action {
            //             Action::Exit => {
            //                 self.status = SelectedScreen::RegionSelect;
            //             }
            //             Action::ReturnInstance(instance) => {
            //                 should_exit = true;
            //                 return_value = Some(instance);
            //             }
            //             Action::Select(instance) => {
            //                 self.info_panel_component.set_instance(instance);
            //             }
            //             _ => {}
            //         }
            //     }
            //     SelectedScreen::Config => {
            //         let action = self.config_panel.handle_action(event)?;
            //         if let Action::Exit = action {
            //             self.status = SelectedScreen::RegionSelect;
            //         }
            //     }
            // }

            if should_exit {
                break;
            }
        }
        match return_value {
            Some(instance) => Ok(instance),
            None => Err(RuntimeError::UserExit.into()),
        }
    }

    /**
     * Creates the app layout and returns the area for components to render themselves
     */
    fn get_component_render_area(&self, frame: &mut Frame) -> Rect {
        let outer = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Max(3), Constraint::Fill(1)].as_ref())
            .split(frame.area());

        let tabs = Tabs::new(vec!["Region", "Instances", "Connection"])
            .block(Block::bordered())
            .style(Style::default().white())
            .highlight_style(Style::default().yellow())
            .select(match self.selected_screen {
                SelectedScreen::RegionSelect => Some(0),
                SelectedScreen::InstanceSelect => Some(1),
                _ => None,
            });
        frame.render_widget(tabs, outer[0]);
        outer[1]
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Err(e) = restore_terminal(&mut self.terminal) {
            eprintln!("Failed to restore terminal: {:?}", e);
        }
    }
}
