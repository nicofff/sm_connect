mod aws;
use aws::InstanceInfo;
mod sftp;
mod app;
mod ui;
use app::{App, UserAction};
use std::net::TcpListener;
use std::process::Command;
mod components;
mod history;
use history::{History, HistoryEntry};
mod screens;
mod file_manager_app;
use file_manager_app::FileManagerApp;
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

    /// Forward SSH port over SSM instead of opening an interactive session
    #[arg(short, long)]
    tunnel: bool,

    /// Open integrated file manager over SSM tunnel
    #[arg(short, long)]
    file_manager: bool,
}

fn pick_local_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    Ok(port)
    // listener dropped here, releasing the port
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // If both region and instance are provided, connect directly
    if let (Some(region_str), Some(instance_id)) = (args.region, args.instance) {
        let region = Region::new(region_str);
        let instance = InstanceInfo::new(region, instance_id);
        if args.tunnel {
            tunnel(instance)?;
        } else if args.file_manager {
            launch_file_manager(instance)?;
        } else {
            connect(instance)?;
        }
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
        Ok(Some(UserAction::Connect(instance))) => connect(instance)?,
        Ok(Some(UserAction::Tunnel(instance))) => tunnel(instance)?,
        Ok(Some(UserAction::FileManager(instance))) => launch_file_manager(instance)?,
        Ok(Some(UserAction::EcsExec { task, container })) => ecs_exec(task, container)?,
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

fn tunnel(instance: InstanceInfo) -> Result<()> {
    let local_port = pick_local_port()?;
    let parameters = format!(
        r#"{{"portNumber":["22"],"localPortNumber":["{}"]}}"#,
        local_port
    );
    println!(
        "SSH tunnel ready — connect your file manager to sftp://localhost:{}\nPress Ctrl+C to close the tunnel.",
        local_port
    );
    run_aws_command(&[
        "--region",
        instance.get_region().as_ref(),
        "ssm",
        "start-session",
        "--target",
        instance.get_instance_id(),
        "--document-name",
        "AWS-StartPortForwardingSession",
        "--parameters",
        &parameters,
    ])
}

fn launch_file_manager(instance: InstanceInfo) -> Result<()> {
    let mut app = FileManagerApp::new()?;
    app.run(instance)
}

fn ecs_exec(task: aws::EcsTaskInfo, container: String) -> Result<()> {
    run_aws_command(&[
        "--region",
        task.get_region().as_ref(),
        "ecs",
        "execute-command",
        "--cluster",
        task.get_cluster(),
        "--task",
        task.get_task_arn(),
        "--container",
        &container,
        "--interactive",
        "--command",
        // v1: shell is hardcoded; making this configurable is a deliberate future step.
        "/bin/sh",
    ])
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
