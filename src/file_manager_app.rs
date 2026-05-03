use std::io::Stdout;
use std::path::PathBuf;
use std::process::Child;

use anyhow::Result;
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;

use crate::aws::InstanceInfo;
use crate::screens::Screen;
use crate::screens::connecting_screen::{ConnectingScreen, Outcome as ConnectingOutcome};
use crate::screens::file_manager_screen::FileManagerScreen;
use crate::screens::username_prompt_screen::{UsernamePromptScreen, Outcome as UsernameOutcome};
use crate::ui::{restore_terminal, setup_terminal};

pub struct FileManagerApp {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    _ssm_child: Option<Child>,
}

impl FileManagerApp {
    pub fn new() -> Result<Self> {
        Ok(Self {
            terminal: setup_terminal()?,
            _ssm_child: None,
        })
    }

    pub fn run(&mut self, instance: InstanceInfo) -> Result<()> {
        // Step 1: Prompt for username
        let username = {
            let mut screen = UsernamePromptScreen::new();
            match screen.run(&mut self.terminal)? {
                UsernameOutcome::Username(u) => u,
                UsernameOutcome::Cancel => return Ok(()),
            }
        };

        // Step 2: Establish SSM tunnel + SFTP connection
        let (sftp, child) = {
            let mut screen = ConnectingScreen::new(instance, username)?;
            match screen.run(&mut self.terminal)? {
                ConnectingOutcome::Connected(client, child) => (client, child),
                ConnectingOutcome::Failed(msg) => {
                    // Restore terminal before printing error so it shows correctly
                    restore_terminal(&mut self.terminal)?;
                    eprintln!("Failed to connect: {msg}");
                    return Ok(());
                }
                ConnectingOutcome::Cancelled => return Ok(()),
            }
        };

        self._ssm_child = Some(child);

        // Step 3: Run the file manager
        let mut screen = FileManagerScreen::new(&sftp, PathBuf::from("/"))?;
        screen.run_with_sftp(&mut self.terminal, &sftp)?;

        Ok(())
    }
}

impl Drop for FileManagerApp {
    fn drop(&mut self) {
        if let Err(e) = restore_terminal(&mut self.terminal) {
            eprintln!("Failed to restore terminal: {e:?}");
        }
        if let Some(mut child) = self._ssm_child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
