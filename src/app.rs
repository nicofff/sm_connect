use crate::aws::fetch_instances;
use crate::aws::InstanceInfo;
use crate::components::instance_details::InstanceDetails;
use crate::components::instance_table::InstanceTable;
use crate::components::region_list::RegionList;
use crate::components::text_input::TextInput;
use crate::components::RenderHelp;
use crate::components::{Action, HandleAction, Render};

use aws_config::Region;
use crossterm::event::{self};

use ratatui::style::{Style};
use ratatui::{prelude::*, widgets::*};

use std::io::Stdout;
use std::rc::Rc;

use anyhow::Result;
use thiserror::Error;

mod config;

#[derive(Debug, Clone)]
pub enum AppStatus {
    RegionSelectState,
    MainScreen,
}

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("User exited the application")]
    UserExit,
}

#[derive(Debug, Clone)]
pub struct App {
    config: config::Config,
    info_panel_enabled: bool,
    region_select_component: RegionList,
    search_component: TextInput,
    status: AppStatus,
    instances_table_component: InstanceTable,
    search_enabled: bool,
    info_panel_component: InstanceDetails,
}

impl App {
    pub fn new() -> Self {
        let config = config::Config::new();
        let mut region_select = RegionList::with_items(config.get_visible_regions());
        region_select.set_favorites(config.get_favorite_regions());
        App {
            config: config.clone(),
            search_component: TextInput::default(),
            region_select_component: region_select,
            status: AppStatus::RegionSelectState,
            instances_table_component: InstanceTable::with_items(vec![]),
            info_panel_enabled: false,
            info_panel_component: InstanceDetails::default(),
            search_enabled: false,
        }
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    ) -> Result<InstanceInfo> {
        let mut should_exit = false;
        let mut return_value: Option<InstanceInfo> = None;
        loop {
            // render
            terminal
                .draw(|frame| {
                    // Set global layout
                    let outer_layout = self.get_outer_layout(frame);
                    let inner_layout = self.get_inner_layout(frame, &outer_layout);
                    
                    match self.status {
                        AppStatus::RegionSelectState => {
                            self.region_select_component.render(frame, inner_layout[0]);
                            self.region_select_component.render_help(frame, outer_layout[2]);
                        }
                        AppStatus::MainScreen => {
                            self.instances_table_component.render(frame, inner_layout[0]);
                            self.info_panel_component.render(frame, inner_layout[1]);
                            if self.search_enabled {
                                self.search_component.render(frame, outer_layout[2]);
                                frame.set_cursor(
                                    outer_layout[2].x
                                        + self.search_component.get_cursor_position() as u16,
                                    outer_layout[2].y,
                                );
                            } else {
                                self.instances_table_component.render_help(frame, outer_layout[2])
                            }
                        }
                    }
                })
                .unwrap();

            // handle events
            let event = event::read().unwrap();
            match self.status {
                AppStatus::RegionSelectState => {
                    let action = self.region_select_component.handle_action(event);
                    match action {
                        Action::Exit => {
                            should_exit = true;
                        }
                        Action::Return(region) => {
                            self.status = AppStatus::MainScreen;
                            let mut instances = fetch_instances(Region::new(region)).await?;
                            instances
                                .sort_by_key(|instance_info| instance_info.get_name().to_owned());
                            let search_input = self.search_component.get_value();

                            self.instances_table_component = InstanceTable::with_items_and_filter(instances, search_input);
                            
                        }
                        Action::Hide(region) => {
                            self.config.hide_region(region);
                            self.region_select_component
                                .update_items(self.config.get_visible_regions());
                        }
                        Action::Reset => {
                            self.config.reset_hidden_regions();
                            self.region_select_component
                                .update_items(self.config.get_visible_regions());
                        }
                        Action::ToggleFavorite(region) => {
                            self.config.toggle_favorite_region(region);
                            self.region_select_component
                                .set_favorites(self.config.get_favorite_regions());
                        }
                        _ => {}
                    }
                }
                AppStatus::MainScreen => {
                    if self.search_enabled {
                        let action = self.search_component.handle_action(event);
                        match action {
                            Action::Exit => {
                                self.search_enabled = false;
                            }
                            Action::Return(search) => {
                                self.instances_table_component.apply_filter(search);
                                self.search_enabled = false;
                            }
                            Action::PartialReturn(search) => {
                                self.instances_table_component.apply_filter(search);
                            }
                            Action::ReturnWithKey(key) => {
                                match key {
                                    event::KeyCode::Up => {
                                        self.instances_table_component.previous();
                                    }
                                    event::KeyCode::Down => {
                                        self.instances_table_component.next();
                                    }
                                    _ => {}
                                }
                                self.search_enabled = false;
                            }
                            _ => {}
                        }
                    } else {
                        let action = self
                            .instances_table_component.handle_action(event);
                        match action {
                            Action::Exit => {
                                self.status = AppStatus::RegionSelectState;
                            }
                            Action::ReturnInstance(instance) => {
                                should_exit = true;
                                return_value = Some(instance);
                            }
                            Action::Search => {
                                self.search_enabled = true;
                            }
                            Action::ToggleInfoPanel => {
                                self.info_panel_enabled = !self.info_panel_enabled;
                            }
                            Action::Select(instance) => {
                                self.info_panel_component.set_instance(instance);
                            }
                            _ => {}
                        }
                    }
                }
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

    fn get_outer_layout(&self, frame: &mut Frame) -> Rc<[Rect]> {
        let outer = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints(
            [
                Constraint::Percentage(10),
                Constraint::Percentage(95),
                Constraint::Percentage(5),
            ]
            .as_ref(),
        )
        .split(frame.size());

        let tabs = Tabs::new(vec!["Region", "Instances", "Connection"])
            .block(Block::bordered())
            .style(Style::default().white())
            .highlight_style(Style::default().yellow())
            .select(match self.status {
                AppStatus::RegionSelectState => 0,
                AppStatus::MainScreen => 1,
            });
        //.divider(symbols::DOT);
        frame.render_widget(tabs, outer[0]);
        outer
    }

    fn get_inner_layout(&self, _frame: &mut Frame, outer_layout: &Rc<[Rect]>) -> Rc<[Rect]> {
        let inner_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(if self.info_panel_enabled {
                vec![Constraint::Percentage(75), Constraint::Percentage(25)]
            } else {
                vec![Constraint::Percentage(100), Constraint::Percentage(0)]
            })
            .split(outer_layout[1]);
        inner_layout
    }
}
