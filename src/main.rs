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
use clap::Parser;
use aws_config::Region;

/// AWS Systems Manager Session Manager connection tool
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// AWS region (e.g., us-east-1, us-west-2)
    #[arg(short, long)]
    region: Option<String>,

    /// AWS EC2 instance ID (e.g., i-ad53d5e3831ea)
    #[arg(short, long)]
    instance: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // If both region and instance are provided, connect directly
    if let (Some(region_str), Some(instance_id)) = (args.region, args.instance) {
        let region = Region::new(region_str);
        let instance = InstanceInfo::new(region, instance_id);
        connect(instance)?;
        return Ok(());
    }

    // Otherwise, run the interactive UI
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
    let mut child = Command::new("aws")
        .args([
            "--region",
            instance.get_region().as_ref(),
            "ssm",
            "start-session",
            "--target",
            instance.get_instance_id(),
        ])
        .spawn()?;

    // Catch SIGINT, SIGSTP signal and do nothing
    // So that actually ctrl+c / ctrl+z works on the aws ssm session instead of killing / stopping us
    let mut _signals = Signals::new([SIGINT, SIGTSTP])?;

    child.wait()?;
    Ok(())
}
