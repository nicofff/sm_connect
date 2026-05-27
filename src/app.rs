use crate::aws::InstanceInfo;
use crate::aws::EcsTaskInfo;

use crate::screens::Screen;
use crate::screens::config_screen::ConfigScreen;
use crate::screens::container_select_screen::{ContainerSelectScreen, ContainerSelectScreenOutcome};
use crate::screens::instance_select_screen::InstanceSelectScreen;
use crate::screens::loading_instances_screen::LoadingInstancesScreen;
use crate::screens::loading_tasks_screen::{LoadingTasksScreen, LoadingTasksScreenOutcome};
use crate::screens::mode_select_screen::{ModeSelectScreen, ModeSelectScreenOutcome};
use crate::screens::region_select_screen::RegionSelectScreen;
use crate::screens::task_select_screen::{TaskSelectScreen, TaskSelectScreenOutcome};
use crate::screens::{loading_instances_screen, region_select_screen};
use crate::ui::restore_terminal;
use crate::ui::setup_terminal;

use anyhow::Context;
use ratatui::prelude::*;

use std::io::Stdout;

use anyhow::Result;

pub mod config;

#[derive(Debug, Clone)]
pub enum UserAction {
    Connect(InstanceInfo),
    Tunnel(InstanceInfo),
    FileManager(InstanceInfo),
    EcsExec { task: EcsTaskInfo, container: String },
}

#[derive(Debug, Clone)]
pub enum SelectedScreen {
    RegionSelect,
    ModeSelect(String),
    LoadingInstances(String),
    InstanceSelect,
    LoadingTasks(String),
    TaskSelect,
    ContainerSelect,
    Config,
}

pub struct App {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    selected_screen: SelectedScreen,
    region_select_screen: RegionSelectScreen,
    instance_selection_screen: InstanceSelectScreen,
    task_select_screen: TaskSelectScreen,
    selected_task: Option<EcsTaskInfo>,
    config_screen: ConfigScreen,
}

impl App {
    pub fn new() -> Result<Self> {
        let terminal = setup_terminal().context("setup failed")?;
        let config = config::Config::new()?;
        let region_select_screen = RegionSelectScreen::new(config.clone());
        let instance_selection_screen = InstanceSelectScreen::new();
        let task_select_screen = TaskSelectScreen::new();
        let config_screen = ConfigScreen::new(config.clone());
        Ok(App {
            terminal,
            selected_screen: SelectedScreen::RegionSelect,
            region_select_screen,
            instance_selection_screen,
            task_select_screen,
            selected_task: None,
            config_screen,
        })
    }

    pub async fn run(&mut self) -> Result<Option<UserAction>> {
        let mut should_exit = false;
        let mut return_value: Option<UserAction> = None;
        loop {
            match self.selected_screen.clone() {
                SelectedScreen::RegionSelect => {
                    match self.region_select_screen.run(&mut self.terminal)? {
                        region_select_screen::RegionSelectScreenOutcome::Exit => should_exit = true,
                        region_select_screen::RegionSelectScreenOutcome::RegionSelected(region) => {
                            self.selected_screen = SelectedScreen::ModeSelect(region);
                        }
                        region_select_screen::RegionSelectScreenOutcome::OpenConfig => {
                            self.selected_screen = SelectedScreen::Config;
                        }
                    }
                }
                SelectedScreen::LoadingInstances(region) => {
                    match LoadingInstancesScreen::new(region.clone()).run(&mut self.terminal)? {
                        loading_instances_screen::LoadingInstancesScreenOutcome::Cancelled => {
                            self.selected_screen = SelectedScreen::ModeSelect(region);
                        }
                        loading_instances_screen::LoadingInstancesScreenOutcome::InstancesFetched(instances) => {
                            self.instance_selection_screen.set_instances(instances);
                            self.selected_screen = SelectedScreen::InstanceSelect;
                        }
                    }
                }
                SelectedScreen::InstanceSelect => {
                    match self.instance_selection_screen.run(&mut self.terminal)? {
                        crate::screens::instance_select_screen::InstanceSelectScreenOutcome::Exit => {
                            self.selected_screen = SelectedScreen::RegionSelect;
                        }
                        crate::screens::instance_select_screen::InstanceSelectScreenOutcome::Connect(
                            instance_info,
                        ) => {
                            should_exit = true;
                            return_value = Some(UserAction::Connect(instance_info));
                        }
                        crate::screens::instance_select_screen::InstanceSelectScreenOutcome::Tunnel(
                            instance_info,
                        ) => {
                            should_exit = true;
                            return_value = Some(UserAction::Tunnel(instance_info));
                        }
                        crate::screens::instance_select_screen::InstanceSelectScreenOutcome::FileManager(
                            instance_info,
                        ) => {
                            should_exit = true;
                            return_value = Some(UserAction::FileManager(instance_info));
                        }
                    }
                }
                SelectedScreen::ModeSelect(region) => {
                    match ModeSelectScreen::new().run(&mut self.terminal)? {
                        ModeSelectScreenOutcome::Exit => {
                            self.selected_screen = SelectedScreen::RegionSelect;
                        }
                        ModeSelectScreenOutcome::Ec2 => {
                            self.selected_screen = SelectedScreen::LoadingInstances(region);
                        }
                        ModeSelectScreenOutcome::Ecs => {
                            self.selected_screen = SelectedScreen::LoadingTasks(region);
                        }
                    }
                }
                SelectedScreen::LoadingTasks(region) => {
                    match LoadingTasksScreen::new(region.clone()).run(&mut self.terminal)? {
                        LoadingTasksScreenOutcome::Cancelled => {
                            self.selected_screen = SelectedScreen::ModeSelect(region);
                        }
                        LoadingTasksScreenOutcome::TasksFetched(tasks) => {
                            self.task_select_screen.set_tasks(tasks);
                            self.selected_screen = SelectedScreen::TaskSelect;
                        }
                    }
                }
                SelectedScreen::TaskSelect => {
                    match self.task_select_screen.run(&mut self.terminal)? {
                        TaskSelectScreenOutcome::Exit => {
                            // Exiting the list starts over at region select, matching the
                            // EC2 path (InstanceSelect Exit also returns to RegionSelect).
                            self.selected_screen = SelectedScreen::RegionSelect;
                        }
                        TaskSelectScreenOutcome::Exec(task) => {
                            if task.get_containers().len() == 1 {
                                let container = task.get_containers()[0].clone();
                                should_exit = true;
                                return_value = Some(UserAction::EcsExec { task, container });
                            } else {
                                self.selected_task = Some(task);
                                self.selected_screen = SelectedScreen::ContainerSelect;
                            }
                        }
                    }
                }
                SelectedScreen::ContainerSelect => {
                    let task = self
                        .selected_task
                        .clone()
                        .expect("ContainerSelect requires a selected task");
                    let containers = task.get_containers().to_vec();
                    match ContainerSelectScreen::new(containers).run(&mut self.terminal)? {
                        ContainerSelectScreenOutcome::Exit => {
                            self.selected_screen = SelectedScreen::TaskSelect;
                        }
                        ContainerSelectScreenOutcome::Return(container) => {
                            should_exit = true;
                            return_value = Some(UserAction::EcsExec { task, container });
                        }
                    }
                }
                SelectedScreen::Config => match self.config_screen.run(&mut self.terminal)? {
                    crate::screens::config_screen::ConfigScreenOutcome::Exit => {
                        self.selected_screen = SelectedScreen::RegionSelect;
                    }
                },
            }

            if should_exit {
                break;
            }
        }
        Ok(return_value)
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Err(e) = restore_terminal(&mut self.terminal) {
            eprintln!("Failed to restore terminal: {:?}", e);
        }
    }
}
