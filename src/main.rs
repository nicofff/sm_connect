mod aws;
use aws::InstanceInfo;
mod app;
mod ui;
use app::App;
use std::process::Command;
mod components;
mod history;
use history::{History, HistoryEntry};
mod screens;
use anyhow::Result;
use signal_hook::{consts::signal::*, iterator::Signals};
use home::home_dir;

#[tokio::main]
async fn main() -> Result<()> {
    let mut app = App::new()?;
    let selected = app.run().await;
    drop(app);
    match selected {
        Err(e) => match e.downcast_ref() {
            Some(app::RuntimeError::UserExit) => {}
            _ => {
                println!("{:?}", e);
            }
        },
        Ok(instance) => connect(instance)?,
    }
    Ok(())
}

fn connect(instance: InstanceInfo) -> Result<()> {
    // Run the AWS command
    let entry = HistoryEntry::new(instance.get_instance_id());
    History::save(entry)?;

    // Try AWS-StartInteractiveCommand first to start in home directory
    let mut command = Command::new("aws");
    command.args([
        "--region",
        instance.get_region().as_ref(),
        "ssm",
        "start-session",
        "--target",
        instance.get_instance_id(),
        "--document-name",
        "AWS-StartInteractiveCommand",
        "--parameters",
        r#"command="cd ~ && exec bash -l""#,
    ]);

    // Set local working directory to home directory if possible
    if let Some(home_dir) = home_dir() {
        command.current_dir(home_dir);
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => {
            println!("AWS-StartInteractiveCommand failed, trying standard start-session");
            let mut fallback_command = Command::new("aws");
            fallback_command.args([
                "--region",
                instance.get_region().as_ref(),
                "ssm",
                "start-session",
                "--target",
                instance.get_instance_id(),
            ]);

            if let Some(home_dir) = home_dir() {
                fallback_command.current_dir(home_dir);
            }

            fallback_command.spawn()?
        }
    };

    // Catch SIGINT, SIGSTP signal and do nothing
    // So that actually ctrl+c / ctrl+z works on the aws ssm session instead of killing / stopping us
    let mut _signals = Signals::new([SIGINT, SIGTSTP])?;

    child.wait()?;
    Ok(())
}
