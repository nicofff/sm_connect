use std::io::Stdout;
use std::net::TcpListener;
use std::process::{Child, Command};
use std::time::Duration;

use anyhow::{Result, anyhow};
use crossterm::event;
use ratatui::{Terminal, prelude::CrosstermBackend};
use tokio::sync::oneshot;

use crate::aws::InstanceInfo;
use crate::components::{Component, loader::{Loader, LoaderOutputAction}};
use crate::sftp::SftpClient;
use super::Screen;

pub struct ConnectingScreen {
    loader: Loader<Result<(SftpClient, u16)>>,
    ssm_child: Option<Child>,
}

pub enum Outcome {
    Connected(SftpClient, Child),
    Failed(String),
    Cancelled,
}

fn pick_local_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

impl ConnectingScreen {
    pub fn new(instance: InstanceInfo, username: String) -> Result<Self> {
        let local_port = pick_local_port()?;
        let local_port_str = local_port.to_string();
        let parameters = format!(
            r#"{{"portNumber":["22"],"localPortNumber":["{local_port}"]}}"#
        );

        let ssm_child = Command::new("aws")
            .args([
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
            .spawn()?;

        let (tx, rx) = oneshot::channel();

        tokio::spawn(async move {
            // Poll until localhost:local_port accepts connections (max 10s)
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            loop {
                if std::time::Instant::now() > deadline {
                    let _ = tx.send(Err(anyhow!(
                        "Timed out waiting for SSH on the instance. Is sshd running?"
                    )));
                    return;
                }
                match std::net::TcpStream::connect(format!("127.0.0.1:{local_port_str}")) {
                    Ok(_) => break,
                    Err(_) => tokio::time::sleep(Duration::from_millis(500)).await,
                }
            }

            let result = SftpClient::connect("127.0.0.1", local_port, &username).await;
            let _ = tx.send(result.map(|client| (client, local_port)));
        });

        Ok(Self {
            loader: Loader::new("Establishing SSM tunnel and connecting via SFTP…", rx),
            ssm_child: Some(ssm_child),
        })
    }
}

impl Screen<Outcome> for ConnectingScreen {
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<Outcome> {
        loop {
            terminal.draw(|frame| self.loader.view(frame, frame.area()))?;

            let msg = if event::poll(Duration::from_millis(50))? {
                self.loader.handle_event(event::read()?)
            } else {
                None
            };

            match self.loader.update(msg)? {
                Some(LoaderOutputAction::Exit) => {
                    if let Some(mut child) = self.ssm_child.take() {
                        let _ = child.kill();
                    }
                    return Ok(Outcome::Cancelled);
                }
                Some(LoaderOutputAction::Return(Ok((client, _port)))) => {
                    let child = self.ssm_child.take().unwrap();
                    return Ok(Outcome::Connected(client, child));
                }
                Some(LoaderOutputAction::Return(Err(e))) => {
                    if let Some(mut child) = self.ssm_child.take() {
                        let _ = child.kill();
                    }
                    return Ok(Outcome::Failed(e.to_string()));
                }
                None => {}
            }
        }
    }
}
