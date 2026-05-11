use std::io::Stdout;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    Terminal,
    layout::{Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Row, Table},
};

use crate::components::file_pane::FilePane;
use crate::sftp::{FileInfo, SftpClient};
use super::Screen;

#[derive(Debug, Clone)]
enum TransferDirection {
    Upload,   // local → remote
    Download, // remote → local
}

#[derive(Debug, Clone)]
struct TransferItem {
    source: PathBuf,
    destination: PathBuf,
    direction: TransferDirection,
    is_dir: bool,
}

enum ActivePane {
    Local,
    Remote,
}

pub struct FileManagerScreen {
    local_pane: FilePane,
    remote_pane: FilePane,
    local_path: PathBuf,
    remote_path: PathBuf,
    local_files: Vec<FileInfo>,
    remote_files: Vec<FileInfo>,
    active: ActivePane,
    show_transfer_dialog: bool,
    transfer_queue: Vec<TransferItem>,
    status: String,
}

pub enum FileManagerScreenOutcome {
    Quit,
}

impl FileManagerScreen {
    pub fn new(_sftp: &SftpClient, initial_remote_path: PathBuf) -> Result<Self> {
        let local_path = std::env::current_dir()?;
        let local_files = Self::read_local_dir(&local_path)?;

        let mut local_pane = FilePane::new("Local");
        local_pane.set_item_count(local_files.len());
        local_pane.state.select(Some(0));

        let remote_pane = FilePane::new("Remote");

        Ok(Self {
            local_pane,
            remote_pane,
            local_path,
            remote_path: initial_remote_path,
            local_files,
            remote_files: vec![],
            active: ActivePane::Local,
            show_transfer_dialog: false,
            transfer_queue: vec![],
            status: "Loading remote directory…".to_string(),
        })
    }

    fn read_local_dir(path: &PathBuf) -> Result<Vec<FileInfo>> {
        let mut files = vec![];
        if let Some(parent) = path.parent() {
            files.push(FileInfo {
                name: "..".into(),
                path: parent.to_path_buf(),
                is_dir: true,
                size: 0,
            });
        }
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            files.push(FileInfo {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: entry.path(),
                is_dir: meta.is_dir(),
                size: meta.len(),
            });
        }
        if files.len() > 1 {
            files[1..].sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            });
        }
        Ok(files)
    }

    fn prepare_transfer(&mut self) {
        self.transfer_queue.clear();
        for &i in &self.local_pane.selected {
            if let Some(f) = self.local_files.get(i) {
                if f.name == ".." {
                    continue;
                }
                self.transfer_queue.push(TransferItem {
                    source: f.path.clone(),
                    destination: self.remote_path.join(&f.name),
                    direction: TransferDirection::Upload,
                    is_dir: f.is_dir,
                });
            }
        }
        for &i in &self.remote_pane.selected {
            if let Some(f) = self.remote_files.get(i) {
                if f.name == ".." {
                    continue;
                }
                self.transfer_queue.push(TransferItem {
                    source: f.path.clone(),
                    destination: self.local_path.join(&f.name),
                    direction: TransferDirection::Download,
                    is_dir: f.is_dir,
                });
            }
        }
        if !self.transfer_queue.is_empty() {
            self.show_transfer_dialog = true;
        }
    }

    fn execute_transfers(&mut self, sftp: &SftpClient) -> Result<()> {
        let queue = self.transfer_queue.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                for item in &queue {
                    let result = match item.direction {
                        TransferDirection::Upload => {
                            if item.is_dir {
                                sftp.upload_directory(&item.source, &item.destination).await
                            } else {
                                sftp.upload_file(&item.source, &item.destination).await
                            }
                        }
                        TransferDirection::Download => {
                            if item.is_dir {
                                sftp.download_directory(&item.source, &item.destination).await
                            } else {
                                sftp.download_file(&item.source, &item.destination).await
                            }
                        }
                    };
                    result?;
                }
                Ok::<(), anyhow::Error>(())
            })
        })
    }

    fn refresh_remote(&mut self, sftp: &SftpClient) -> Result<()> {
        let remote_path = self.remote_path.clone();
        let result = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(sftp.list_directory(&remote_path))
        });
        match result {
            Ok(files) => {
                self.remote_files = files;
                self.remote_pane.set_item_count(self.remote_files.len());
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn draw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        terminal.draw(|frame| {
            let area = frame.area();

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Fill(1), Constraint::Max(1), Constraint::Max(1)])
                .split(area);

            let panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(layout[0]);

            let local_active = matches!(self.active, ActivePane::Local);
            self.local_pane.view_with_files(frame, panes[0], &self.local_files, local_active);
            self.remote_pane.view_with_files(frame, panes[1], &self.remote_files, !local_active);

            let status = Paragraph::new(self.status.as_str())
                .style(Style::default().fg(Color::Gray));
            frame.render_widget(status, layout[1]);

            let help = Table::new(
                vec![Row::new(vec![
                    "Tab: Switch pane",
                    "Space: Select",
                    "t: Transfer",
                    "q: Quit",
                ])],
                vec![Constraint::Fill(1); 4],
            );
            frame.render_widget(help, layout[2]);

            if self.show_transfer_dialog {
                let items: Vec<String> = self.transfer_queue.iter().map(|item| {
                    let arrow = match item.direction {
                        TransferDirection::Upload => "^",
                        TransferDirection::Download => "v",
                    };
                    format!(
                        "{arrow} {}",
                        item.source
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                    )
                }).collect();

                let text = format!(
                    "Transfer {} item(s):\n{}\n\nEnter to confirm * Esc to cancel",
                    items.len(),
                    items.join("\n")
                );

                let dialog_area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Fill(1),
                        Constraint::Min(10),
                        Constraint::Fill(1),
                    ])
                    .split(area)[1];
                let dialog_area = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Fill(1),
                        Constraint::Min(50),
                        Constraint::Fill(1),
                    ])
                    .split(dialog_area)[1];

                frame.render_widget(Clear, dialog_area);
                frame.render_widget(
                    Paragraph::new(text).block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(" Confirm Transfer "),
                    ),
                    dialog_area,
                );
            }
        })?;
        Ok(())
    }

    pub fn run_with_sftp(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        sftp: &SftpClient,
    ) -> Result<FileManagerScreenOutcome> {
        // Initial remote listing
        match self.refresh_remote(sftp) {
            Ok(()) => self.status = format!("Remote: {}", self.remote_path.display()),
            Err(e) => self.status = format!("Error listing remote: {e}"),
        }

        loop {
            self.draw(terminal)?;

            if !event::poll(Duration::from_millis(50))? {
                continue;
            }

            let ev = event::read()?;

            if self.show_transfer_dialog {
                if let Event::Key(key) = &ev {
                    match key.code {
                        KeyCode::Enter => {
                            self.show_transfer_dialog = false;
                            self.status = "Transferring…".to_string();
                            match self.execute_transfers(sftp) {
                                Ok(()) => {
                                    self.local_pane.clear_selection();
                                    self.remote_pane.clear_selection();
                                    self.local_files = Self::read_local_dir(&self.local_path)?;
                                    self.local_pane.set_item_count(self.local_files.len());
                                    match self.refresh_remote(sftp) {
                                        Ok(()) => self.status = "Transfer complete.".to_string(),
                                        Err(e) => self.status = format!("Error listing remote: {e}"),
                                    }
                                    self.transfer_queue.clear();
                                }
                                Err(e) => {
                                    self.status = format!("Transfer error: {e}");
                                    self.transfer_queue.clear();
                                }
                            }
                        }
                        KeyCode::Esc => {
                            self.show_transfer_dialog = false;
                            self.transfer_queue.clear();
                        }
                        _ => {}
                    }
                }
                continue;
            }

            if let Event::Key(key) = &ev {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(FileManagerScreenOutcome::Quit),
                    KeyCode::Tab => {
                        self.active = match self.active {
                            ActivePane::Local => ActivePane::Remote,
                            ActivePane::Remote => ActivePane::Local,
                        };
                    }
                    KeyCode::Char('t') => self.prepare_transfer(),
                    KeyCode::Up | KeyCode::Char('k') => match self.active {
                        ActivePane::Local => self.local_pane.previous(self.local_files.len()),
                        ActivePane::Remote => self.remote_pane.previous(self.remote_files.len()),
                    },
                    KeyCode::Down | KeyCode::Char('j') => match self.active {
                        ActivePane::Local => self.local_pane.next(self.local_files.len()),
                        ActivePane::Remote => self.remote_pane.next(self.remote_files.len()),
                    },
                    KeyCode::Enter => match self.active {
                        ActivePane::Local => {
                            if let Some(i) = self.local_pane.current_index()
                                && let Some(f) = self.local_files.get(i)
                                    && f.is_dir {
                                        self.local_path = f.path.clone();
                                        self.local_files =
                                            Self::read_local_dir(&self.local_path)?;
                                        self.local_pane.set_item_count(self.local_files.len());
                                        self.local_pane.clear_selection();
                                    }
                        }
                        ActivePane::Remote => {
                            if let Some(i) = self.remote_pane.current_index()
                                && let Some(f) = self.remote_files.get(i).cloned()
                                    && f.is_dir {
                                        self.remote_path = f.path.clone();
                                        match self.refresh_remote(sftp) {
                                            Ok(()) => self.status = format!("Remote: {}", self.remote_path.display()),
                                            Err(e) => self.status = format!("Error listing remote: {e}"),
                                        }
                                        self.remote_pane.clear_selection();
                                    }
                        }
                    },
                    KeyCode::Char(' ') => match self.active {
                        ActivePane::Local => {
                            if let Some(i) = self.local_pane.current_index()
                                && self.local_files.get(i).map(|f| f.name.as_str()) != Some("..") {
                                    self.local_pane.toggle_selected(i);
                                }
                        }
                        ActivePane::Remote => {
                            if let Some(i) = self.remote_pane.current_index()
                                && self.remote_files.get(i).map(|f| f.name.as_str()) != Some("..") {
                                    self.remote_pane.toggle_selected(i);
                                }
                        }
                    },
                    _ => {}
                }
            }
        }
    }
}

impl Screen for FileManagerScreen {
    type Outcome = FileManagerScreenOutcome;
    fn run(&mut self, _terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<FileManagerScreenOutcome> {
        Ok(FileManagerScreenOutcome::Quit)
    }
}
