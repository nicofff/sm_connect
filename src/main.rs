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
use aws_config::Region;
use clap::Parser;
use signal_hook::{consts::signal::*, iterator::Signals};

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
        Err(e) => {
            eprintln!("Error: {}", e);
            for cause in e.chain().skip(1) {
                eprintln!("  caused by: {}", cause);
            }
        },
        Ok(Some(instance)) => connect(instance)?,
        Ok(None) => {}
    }
    Ok(())
}

fn run_aws_command(args: &[&str]) -> Result<()> {
    let mut child = Command::new("aws")
        .args(args)
        .spawn()?;

    // Catch SIGINT, SIGTSTP so they pass through to the child (aws ssm session) instead of killing us
    let mut _signals = Signals::new([SIGINT, SIGTSTP])?;

    child.wait()?;
    Ok(())
}

fn connect(instance: InstanceInfo) -> Result<()> {
    let entry = HistoryEntry::new(instance.get_instance_id());
    History::save(entry)?;
    run_aws_command(&[
        "--region",
        instance.get_region().as_ref(),
        "ssm",
        "start-session",
        "--target",
        instance.get_instance_id(),
    ])
}
