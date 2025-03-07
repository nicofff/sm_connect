use std::sync::{Arc, Mutex};

use crate::{app::config::Config, components::{config_panel::config_list::{ConfigList, ConfigOption}, text_input::TextInput}};



struct ConfigScreen {
    config: Arc<Mutex<Config>>,
    // config_list: ConfigList,
    // input_component: TextInput,
    // input_active: bool,
    // modifying_action: Option<ConfigOption>,
    // last_operation_success: Option<bool>,
}

impl ConfigScreen {
    pub fn new(config:  Arc<Mutex<Config>>) -> Self {
        Self { 
            config: config.clone() 
        }
    }
}