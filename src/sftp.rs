use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use russh::client::{self, Handle};
use russh::keys::agent::client::AgentClient;
use russh_sftp::client::SftpSession;
use tokio::fs;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

pub struct SftpClient {
    sftp: SftpSession,
    _handle: Handle<ClientHandler>,
}

struct ClientHandler;

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // Security comes from the SSM tunnel — accept any host key on localhost
        Ok(true)
    }
}

impl SftpClient {
    pub async fn connect(host: &str, port: u16, user: &str) -> Result<Self> {
        let config = Arc::new(client::Config::default());
        let mut handle = client::connect(config, (host, port), ClientHandler).await?;

        // Connect to the SSH agent and try each available key
        let mut agent = AgentClient::connect_env()
            .await
            .map_err(|e| anyhow!("Failed to connect to SSH agent: {e}"))?;

        let identities = agent
            .request_identities()
            .await
            .map_err(|e| anyhow!("Failed to list SSH agent identities: {e}"))?;

        if identities.is_empty() {
            return Err(anyhow!(
                "SSH agent has no identities loaded — run `ssh-add` to load a key"
            ));
        }

        let mut authenticated = false;
        for identity in identities {
            let public_key = identity.public_key().clone();
            match handle
                .authenticate_publickey_with(user, public_key.into_owned(), None, &mut agent)
                .await
            {
                Ok(auth_result) => {
                    if auth_result.success() {
                        authenticated = true;
                        break;
                    }
                }
                Err(_) => continue, // transport error on this key — try next
            }
        }

        if !authenticated {
            return Err(anyhow!(
                "Authentication rejected for user '{user}' — ensure ssh-agent is running with the correct key loaded"
            ));
        }

        let channel = handle.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let sftp = SftpSession::new(channel.into_stream()).await?;

        Ok(SftpClient { sftp, _handle: handle })
    }

    pub async fn list_directory(&self, path: &Path) -> Result<Vec<FileInfo>> {
        let path_str = path.to_string_lossy();
        let entries = self.sftp.read_dir(&*path_str).await?;

        let mut files: Vec<FileInfo> = entries
            .into_iter()
            .map(|entry| {
                let is_dir = entry.file_type().is_dir();
                let size = entry.metadata().len();
                let name = entry.file_name();
                FileInfo {
                    path: path.join(&name),
                    name,
                    is_dir,
                    size,
                }
            })
            .collect();

        files.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        // Prepend .. for navigation (russh-sftp ReadDir skips . and ..)
        if let Some(parent) = path.parent() {
            files.insert(0, FileInfo {
                name: "..".to_string(),
                path: parent.to_path_buf(),
                is_dir: true,
                size: 0,
            });
        }

        Ok(files)
    }

    pub async fn download_file(&self, remote: &Path, local: &Path) -> Result<()> {
        let remote_str = remote.to_string_lossy();
        let mut remote_file = self.sftp.open(&*remote_str).await?;
        let mut local_file = fs::File::create(local).await?;
        tokio::io::copy(&mut remote_file, &mut local_file).await?;
        Ok(())
    }

    pub async fn upload_file(&self, local: &Path, remote: &Path) -> Result<()> {
        let remote_str = remote.to_string_lossy();
        let mut local_file = fs::File::open(local).await?;
        let mut remote_file = self.sftp.create(&*remote_str).await?;
        tokio::io::copy(&mut local_file, &mut remote_file).await?;
        Ok(())
    }

    pub async fn download_directory(&self, remote: &Path, local: &Path) -> Result<()> {
        fs::create_dir_all(local).await?;
        let entries = self.list_directory(remote).await?;
        for entry in entries {
            if entry.name == ".." { continue; }
            let local_path = local.join(&entry.name);
            if entry.is_dir {
                Box::pin(self.download_directory(&entry.path, &local_path)).await?;
            } else {
                self.download_file(&entry.path, &local_path).await?;
            }
        }
        Ok(())
    }

    pub async fn upload_directory(&self, local: &Path, remote: &Path) -> Result<()> {
        let remote_str = remote.to_string_lossy();
        self.sftp.create_dir(&*remote_str).await?;
        let mut entries = fs::read_dir(local).await?;
        while let Some(entry) = entries.next_entry().await? {
            let remote_path = remote.join(entry.file_name());
            if entry.file_type().await?.is_dir() {
                Box::pin(self.upload_directory(&entry.path(), &remote_path)).await?;
            } else {
                self.upload_file(&entry.path(), &remote_path).await?;
            }
        }
        Ok(())
    }
}
