# File Manager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an integrated TUI file manager to sm_connect that lets users browse and transfer files to/from EC2 instances over an SSM-tunneled SFTP connection, launched from the instance select screen with `f`.

**Architecture:** Instance select screen gains `t`/`f` keybindings that return action-typed outcomes; `App::run()` returns a `UserAction` enum dispatching to `connect()`, `tunnel()`, or a new `launch_file_manager()`. The file manager is a fully isolated second `App`-like struct (`FileManagerApp`) with its own terminal, running three screens in sequence: username prompt → connecting (SSM tunnel + SFTP handshake via russh) → dual-pane file browser.

**Tech Stack:** Rust, ratatui 0.30, crossterm 0.29, russh 0.60, russh-sftp 2.1, tokio (already present), signal-hook (already present)

---

## File Map

| File | Change |
|------|--------|
| `Cargo.toml` | Add `russh`, `russh-sftp` dependencies |
| `src/main.rs` | Add `UserAction` dispatch, `--file-manager` flag, `launch_file_manager()` fn |
| `src/app.rs` | `App::run()` returns `Result<Option<UserAction>>`; new `UserAction` enum |
| `src/components/instance_table.rs` | Add `Tunnel`/`FileManager` messages and output actions; update help bar |
| `src/screens/instance_select_screen.rs` | Handle new `InstanceTableOutputAction` variants |
| `src/screens.rs` | Register new screen modules |
| `src/components.rs` | Register new component module |
| `src/sftp.rs` | New — async SFTP client wrapping russh/russh-sftp |
| `src/file_manager_app.rs` | New — isolated app owning terminal + SSM process, drives screen state machine |
| `src/screens/username_prompt_screen.rs` | New — text input for SSH username |
| `src/screens/connecting_screen.rs` | New — SSM tunnel setup + SFTP connect, Loader-based |
| `src/screens/file_manager_screen.rs` | New — dual-pane file browser with transfer flow |
| `src/components/file_pane.rs` | New — file list widget with cursor and selection |
| `README.md` | Document file manager mode |

---

### Task 1: Add `UserAction` enum and update `App::run()` return type

**Files:**
- Modify: `src/app.rs`
- Modify: `src/screens/instance_select_screen.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add `UserAction` to `src/app.rs`**

Add this enum before the `SelectedScreen` enum:

```rust
#[derive(Debug, Clone)]
pub enum UserAction {
    Connect(InstanceInfo),
    Tunnel(InstanceInfo),
    FileManager(InstanceInfo),
}
```

- [ ] **Step 2: Change `App::run()` return type**

Change the signature and update the return sites in `src/app.rs`:

```rust
pub async fn run(&mut self) -> Result<Option<UserAction>> {
```

In the match arm for `InstanceSelect`, replace:

```rust
crate::screens::instance_select_screen::Outcome::InstanceSelected(instance_info) => {
    should_exit = true;
    return_value = Some(instance_info);
}
```

with:

```rust
crate::screens::instance_select_screen::Outcome::Connect(instance_info) => {
    should_exit = true;
    return_value = Some(UserAction::Connect(instance_info));
}
crate::screens::instance_select_screen::Outcome::Tunnel(instance_info) => {
    should_exit = true;
    return_value = Some(UserAction::Tunnel(instance_info));
}
crate::screens::instance_select_screen::Outcome::FileManager(instance_info) => {
    should_exit = true;
    return_value = Some(UserAction::FileManager(instance_info));
}
```

Also change `return_value` type from `Option<InstanceInfo>` to `Option<UserAction>`.

- [ ] **Step 3: Update `InstanceSelectScreen::Outcome`**

In `src/screens/instance_select_screen.rs`, replace:

```rust
pub enum Outcome {
    Exit,
    InstanceSelected(InstanceInfo),
}
```

with:

```rust
pub enum Outcome {
    Exit,
    Connect(InstanceInfo),
    Tunnel(InstanceInfo),
    FileManager(InstanceInfo),
}
```

- [ ] **Step 4: Update `main.rs` to dispatch on `UserAction`**

In `src/main.rs`, replace:

```rust
Ok(Some(instance)) => connect(instance)?,
```

with:

```rust
Ok(Some(UserAction::Connect(instance))) => connect(instance)?,
Ok(Some(UserAction::Tunnel(instance))) => tunnel(instance)?,
Ok(Some(UserAction::FileManager(instance))) => {
    eprintln!("File manager not yet implemented");
}
```

Also add the import at the top:

```rust
use app::UserAction;
```

- [ ] **Step 5: Build to verify**

```bash
cargo build 2>&1
```

Expected: no errors. There will be a dead_code warning on `UserAction::FileManager` — that's fine.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/screens/instance_select_screen.rs src/main.rs
git commit -m "feat: add UserAction enum and update App::run() return type"
```

---

### Task 2: Add `t`/`f` keybindings to instance table

**Files:**
- Modify: `src/components/instance_table.rs`
- Modify: `src/screens/instance_select_screen.rs`

- [ ] **Step 1: Add new message and output action variants to `src/components/instance_table.rs`**

In `InstanceTableMessage`, add:

```rust
pub enum InstanceTableMessage {
    Exit,
    Up,
    Down,
    Enter,
    Search,
    RecentFirst,
    Tunnel,
    FileManager,
}
```

In `InstanceTableOutputAction`, add:

```rust
pub enum InstanceTableOutputAction {
    Exit,
    ReturnInstance(InstanceInfo),
    ReturnInstanceForTunnel(InstanceInfo),
    ReturnInstanceForFileManager(InstanceInfo),
    Search,
}
```

- [ ] **Step 2: Handle new messages in `update()`**

In the `update()` match arm, add after the `InstanceTableMessage::Enter` arm:

```rust
InstanceTableMessage::Tunnel => match self.current() {
    Some(item) => Ok(Some(InstanceTableOutputAction::ReturnInstanceForTunnel(item))),
    None => Ok(None),
},
InstanceTableMessage::FileManager => match self.current() {
    Some(item) => Ok(Some(InstanceTableOutputAction::ReturnInstanceForFileManager(item))),
    None => Ok(None),
},
```

- [ ] **Step 3: Map `t`/`f` keypresses in `handle_event()`**

In the `handle_event()` match arm, add:

```rust
KeyCode::Char('t') => Some(InstanceTableMessage::Tunnel),
KeyCode::Char('f') => Some(InstanceTableMessage::FileManager),
```

- [ ] **Step 4: Update the help bar in `get_help()`**

Replace the existing `get_help()` rows with:

```rust
fn get_help(&self) -> Table<'_> {
    let rows = vec![Row::new(vec![
        get_help_styled('q', "Exit"),
        get_help_styled('/', "Search"),
        get_help_styled(
            'r',
            if self.recent_first {
                "Ignore recent"
            } else {
                "Recent First"
            },
        ),
        get_help_styled('t', "Tunnel"),
        get_help_styled('f', "File Manager"),
    ])];

    Table::new(
        rows,
        vec![
            Constraint::Min(10),
            Constraint::Min(10),
            Constraint::Min(10),
            Constraint::Min(10),
            Constraint::Min(15),
        ],
    )
}
```

- [ ] **Step 5: Handle new output actions in `InstanceSelectScreen`**

In `src/screens/instance_select_screen.rs`, update the match on `InstanceTableOutputAction`:

```rust
Some(InstanceTableOutputAction::Exit) => return Ok(Outcome::Exit),
Some(InstanceTableOutputAction::ReturnInstance(instance)) => {
    return Ok(Outcome::Connect(instance));
}
Some(InstanceTableOutputAction::ReturnInstanceForTunnel(instance)) => {
    return Ok(Outcome::Tunnel(instance));
}
Some(InstanceTableOutputAction::ReturnInstanceForFileManager(instance)) => {
    return Ok(Outcome::FileManager(instance));
}
Some(InstanceTableOutputAction::Search) => {
    self.search_active = true;
}
None => {}
```

- [ ] **Step 6: Build and manually verify**

```bash
cargo build 2>&1
```

Expected: no errors.

Run the binary and open the instance select screen. Verify the help bar shows `T Tunnel` and `F File Manager`. Press `t` and `f` on a selected instance — the app should exit (printing "File manager not yet implemented" for `f`, connecting for `t`).

- [ ] **Step 7: Commit**

```bash
git add src/components/instance_table.rs src/screens/instance_select_screen.rs
git commit -m "feat: add t/f keybindings to instance table for tunnel and file manager"
```

---

### Task 3: Add `--file-manager` CLI flag and `launch_file_manager()` stub

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add `file_manager` field to `Args`**

In `src/main.rs`, update `Args`:

```rust
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
```

- [ ] **Step 2: Add `launch_file_manager` stub and update direct-mode dispatch**

Add the stub function after `tunnel()`:

```rust
fn launch_file_manager(instance: InstanceInfo) -> Result<()> {
    // TODO: implemented in a later task
    println!("File manager launching for instance: {}", instance.get_instance_id());
    Ok(())
}
```

Update the direct-mode dispatch in `main()`:

```rust
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
```

Also update the interactive dispatch to remove the `eprintln!` stub from Task 1:

```rust
Ok(Some(UserAction::Connect(instance))) => connect(instance)?,
Ok(Some(UserAction::Tunnel(instance))) => tunnel(instance)?,
Ok(Some(UserAction::FileManager(instance))) => launch_file_manager(instance)?,
```

- [ ] **Step 3: Build and verify help output**

```bash
cargo build 2>&1
./target/debug/sm_connect --help
```

Expected: help output includes:
```
  -f, --file-manager  Open integrated file manager over SSM tunnel
```

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: add --file-manager flag and launch_file_manager stub"
```

---

### Task 4: Add `russh` dependencies and implement `src/sftp.rs`

**Files:**
- Modify: `Cargo.toml`
- Create: `src/sftp.rs`
- Modify: `src/main.rs` (add `mod sftp;`)

The `russh` SSH client requires implementing a `Handler` trait for the client. For our use case (connecting to a known local port that is already secured by SSM), we accept the host key without verification — the security comes from the SSM tunnel itself, not SSH host key verification.

- [ ] **Step 1: Add dependencies to `Cargo.toml`**

Add to `[dependencies]`:

```toml
russh = "0.60"
russh-sftp = "2.1"
```

- [ ] **Step 2: Create `src/sftp.rs`**

```rust
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Result, anyhow};
use russh::client::{self, Handle};
use russh::keys::ssh_key::PublicKey;
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
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        // Security comes from the SSM tunnel — accept any host key on localhost
        Ok(true)
    }
}

impl SftpClient {
    pub async fn connect(host: &str, port: u16, user: &str) -> Result<Self> {
        let config = Arc::new(client::Config::default());
        let mut handle = client::connect(config, (host, port), ClientHandler).await?;

        let authenticated = handle
            .authenticate_agent_buffered(user)
            .await
            .map_err(|e| anyhow!("SSH agent authentication failed: {e}"))?;

        if !authenticated {
            return Err(anyhow!("Authentication rejected for user '{user}' — ensure ssh-agent is running with the correct key loaded"));
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
                let size = entry.metadata().size.unwrap_or(0);
                FileInfo {
                    name: entry.file_name(),
                    path: path.join(entry.file_name()),
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
```

- [ ] **Step 3: Register the module in `src/main.rs`**

Add at the top of `src/main.rs` with the other `mod` declarations:

```rust
mod sftp;
```

- [ ] **Step 4: Build to verify**

```bash
cargo build 2>&1
```

Expected: no errors. The `sftp` module compiles cleanly.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/sftp.rs src/main.rs
git commit -m "feat: add russh/russh-sftp dependencies and SftpClient implementation"
```

---

### Task 5: Implement `src/screens/username_prompt_screen.rs`

**Files:**
- Create: `src/screens/username_prompt_screen.rs`
- Modify: `src/screens.rs`

- [ ] **Step 1: Create `src/screens/username_prompt_screen.rs`**

```rust
use std::io::Stdout;
use std::env;

use anyhow::Result;
use crossterm::event;
use ratatui::{
    Terminal,
    layout::{Alignment, Constraint, Direction, Layout},
    prelude::CrosstermBackend,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

use crate::components::{Component, text_input::{TextInput, TextInputOutputAction}};
use super::Screen;

pub struct UsernamePromptScreen {
    input: TextInput,
}

pub enum Outcome {
    Username(String),
    Cancel,
}

impl UsernamePromptScreen {
    pub fn new() -> Self {
        let mut input = TextInput::new("Username: ".to_string());
        if let Ok(user) = env::var("USER").or_else(|_| env::var("USERNAME")) {
            input.set_value(user);
        }
        Self { input }
    }

    fn draw(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        terminal.draw(|frame| {
            let area = frame.area();

            let vertical = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Length(5),
                    Constraint::Fill(1),
                ])
                .split(area);

            let horizontal = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Fill(1),
                    Constraint::Min(40),
                    Constraint::Fill(1),
                ])
                .split(vertical[1]);

            let block = Block::default()
                .borders(Borders::ALL)
                .title(" SSH Username ")
                .style(Style::default().fg(Color::Blue));

            let inner = block.inner(horizontal[1]);
            frame.render_widget(block, horizontal[1]);

            let help = Paragraph::new("Enter to confirm · Esc to cancel")
                .alignment(Alignment::Center)
                .style(Style::default().fg(Color::DarkGray));

            let inner_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Fill(1), Constraint::Length(1)])
                .split(inner);

            self.input.view(frame, inner_layout[0]);
            frame.render_widget(help, inner_layout[1]);
        })?;
        Ok(())
    }
}

impl Screen<Outcome> for UsernamePromptScreen {
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<Outcome> {
        loop {
            self.draw(terminal)?;
            let event = event::read()?;
            let msg = self.input.handle_event(event);
            match self.input.update(msg)? {
                Some(TextInputOutputAction::Return(value)) => {
                    let trimmed = value.trim().to_string();
                    if !trimmed.is_empty() {
                        return Ok(Outcome::Username(trimmed));
                    }
                }
                Some(TextInputOutputAction::Exit) => return Ok(Outcome::Cancel),
                _ => {}
            }
        }
    }
}
```

- [ ] **Step 2: Register in `src/screens.rs`**

Add to the module list:

```rust
pub mod username_prompt_screen;
```

- [ ] **Step 3: Build to verify**

```bash
cargo build 2>&1
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/screens/username_prompt_screen.rs src/screens.rs
git commit -m "feat: add UsernamePromptScreen"
```

---

### Task 6: Implement `src/screens/connecting_screen.rs`

This screen manages the SSM port-forward process and SFTP connection. It spawns both as a tokio task, communicates via a `oneshot` channel, and uses the existing `Loader` component for the spinner UI. The `SftpClient` and the SSM `Child` process are passed out together — `FileManagerApp` owns the child and kills it on drop.

**Files:**
- Create: `src/screens/connecting_screen.rs`
- Modify: `src/screens.rs`

- [ ] **Step 1: Create `src/screens/connecting_screen.rs`**

```rust
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
                Some(LoaderOutputAction::Exit) => return Ok(Outcome::Cancelled),
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
```

- [ ] **Step 2: Register in `src/screens.rs`**

Add:

```rust
pub mod connecting_screen;
```

- [ ] **Step 3: Build to verify**

```bash
cargo build 2>&1
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/screens/connecting_screen.rs src/screens.rs
git commit -m "feat: add ConnectingScreen for SSM tunnel + SFTP handshake"
```

---

### Task 7: Implement `src/components/file_pane.rs`

This component renders a single pane (local or remote) — a scrollable file list with cursor highlight and selection markers. It is stateless with respect to data (files are passed in on each `view()` call via the screen) but owns cursor and selection state.

**Files:**
- Create: `src/components/file_pane.rs`
- Modify: `src/components.rs`

- [ ] **Step 1: Create `src/components/file_pane.rs`**

```rust
use crossterm::event::{Event, KeyCode};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Row, Table, TableState, Cell},
};

use crate::sftp::FileInfo;
use super::{Component, get_help_styled};
use anyhow::Result;
use std::collections::HashSet;

pub struct FilePane {
    pub state: TableState,
    pub selected: HashSet<usize>,
    title: String,
}

pub enum FilePaneMessage {
    Up,
    Down,
    Enter,
    Select,
    Exit,
}

pub enum FilePaneOutputAction {
    Enter(FileInfo),
    Select(usize),
    Exit,
}

impl FilePane {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            state: TableState::default(),
            selected: HashSet::new(),
            title: title.into(),
        }
    }

    pub fn set_item_count(&mut self, count: usize) {
        if count == 0 {
            self.state.select(None);
        } else if self.state.selected().map_or(true, |i| i >= count) {
            self.state.select(Some(0));
        }
    }

    pub fn next(&mut self, count: usize) {
        if count == 0 { return; }
        let i = self.state.selected().map_or(0, |i| {
            if i >= count - 1 { 0 } else { i + 1 }
        });
        self.state.select(Some(i));
    }

    pub fn previous(&mut self, count: usize) {
        if count == 0 { return; }
        let i = self.state.selected().map_or(0, |i| {
            if i == 0 { count - 1 } else { i - 1 }
        });
        self.state.select(Some(i));
    }

    pub fn current_index(&self) -> Option<usize> {
        self.state.selected()
    }

    pub fn toggle_selected(&mut self, index: usize) {
        if self.selected.contains(&index) {
            self.selected.remove(&index);
        } else {
            self.selected.insert(index);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected.clear();
    }

    pub fn view_with_files(&mut self, frame: &mut Frame, area: Rect, files: &[FileInfo], active: bool) {
        let border_style = if active {
            Style::default().fg(Color::LightGreen)
        } else {
            Style::default().fg(Color::Gray)
        };

        let rows: Vec<Row> = files
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let marker = if self.selected.contains(&i) { "[*] " } else { "    " };
                let name = if f.is_dir {
                    format!("{marker}{}/", f.name)
                } else {
                    format!("{marker}{}", f.name)
                };
                let size = if f.is_dir {
                    String::from("<DIR>")
                } else {
                    format!("{}", f.size)
                };
                Row::new(vec![Cell::from(name), Cell::from(size)])
                    .style(if self.selected.contains(&i) {
                        Style::default().fg(Color::Cyan)
                    } else {
                        Style::default()
                    })
            })
            .collect();

        let table = Table::new(
            rows,
            [Constraint::Fill(1), Constraint::Min(10)],
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} ", self.title))
                .border_style(border_style),
        )
        .row_highlight_style(
            Style::default()
                .bg(Color::LightGreen)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

        frame.render_stateful_widget(table, area, &mut self.state.clone());
    }

    fn get_help(&self) -> Table<'_> {
        let rows = vec![Row::new(vec![
            get_help_styled('↑', "Up"),
            get_help_styled('↓', "Down"),
            get_help_styled(' ', "Select"),
            get_help_styled('t', "Transfer"),
            get_help_styled('q', "Quit"),
        ])];
        Table::new(
            rows,
            vec![
                Constraint::Min(8),
                Constraint::Min(10),
                Constraint::Min(10),
                Constraint::Min(12),
                Constraint::Min(8),
            ],
        )
    }
}

impl Component for FilePane {
    type Message = FilePaneMessage;
    type OutputAction = FilePaneOutputAction;

    fn update(&mut self, msg: Option<FilePaneMessage>) -> Result<Option<Self::OutputAction>> {
        let Some(msg) = msg else { return Ok(None); };
        match msg {
            FilePaneMessage::Up => Ok(None), // cursor movement handled by screen with file count
            FilePaneMessage::Down => Ok(None),
            FilePaneMessage::Enter => Ok(None), // screen resolves the file from index
            FilePaneMessage::Select => {
                if let Some(i) = self.current_index() {
                    self.toggle_selected(i);
                    Ok(Some(FilePaneOutputAction::Select(i)))
                } else {
                    Ok(None)
                }
            }
            FilePaneMessage::Exit => Ok(Some(FilePaneOutputAction::Exit)),
        }
    }

    fn view(&mut self, frame: &mut Frame, area: Rect) {
        // FilePane.view() is not used directly — call view_with_files() from the screen
        // so the screen can pass the current file list without storing it in the component.
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Max(1)])
            .split(area);
        let help = self.get_help();
        frame.render_widget(help, layout[1]);
    }

    fn handle_event(&self, event: Event) -> Option<Self::Message> {
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Up | KeyCode::Char('k') => Some(FilePaneMessage::Up),
                KeyCode::Down | KeyCode::Char('j') => Some(FilePaneMessage::Down),
                KeyCode::Enter => Some(FilePaneMessage::Enter),
                KeyCode::Char(' ') => Some(FilePaneMessage::Select),
                KeyCode::Char('q') => Some(FilePaneMessage::Exit),
                _ => None,
            },
            _ => None,
        }
    }
}
```

- [ ] **Step 2: Register in `src/components.rs`**

Add:

```rust
pub mod file_pane;
```

- [ ] **Step 3: Build to verify**

```bash
cargo build 2>&1
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/components/file_pane.rs src/components.rs
git commit -m "feat: add FilePaneComponent for dual-pane file browser"
```

---

### Task 8: Implement `src/screens/file_manager_screen.rs`

This screen owns two `FilePane` components (local and remote), the file lists, and transfer queue state. Async SFTP operations (list, transfer) are dispatched via `tokio::spawn` + `oneshot`, matching the `LoadingInstancesScreen` pattern.

**Files:**
- Create: `src/screens/file_manager_screen.rs`
- Modify: `src/screens.rs`

- [ ] **Step 1: Create `src/screens/file_manager_screen.rs`**

```rust
use std::collections::HashSet;
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
use tokio::sync::oneshot::{self, Receiver};

use crate::components::file_pane::{FilePane, FilePaneMessage};
use crate::components::Component;
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
}

enum ActivePane {
    Local,
    Remote,
}

enum PendingOp {
    ListRemote(Receiver<Result<Vec<FileInfo>>>),
    Transfer(Receiver<Result<()>>),
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
    pending: Option<PendingOp>,
    status: String,
}

pub enum Outcome {
    Quit,
}

impl FileManagerScreen {
    pub fn new(sftp: &SftpClient, initial_remote_path: PathBuf) -> Result<Self> {
        let local_path = std::env::current_dir()?;
        let local_files = Self::read_local_dir(&local_path)?;

        let mut local_pane = FilePane::new("Local");
        local_pane.set_item_count(local_files.len());
        local_pane.state.select(Some(0));

        let remote_pane = FilePane::new("Remote");

        let mut screen = Self {
            local_pane,
            remote_pane,
            local_path,
            remote_path: initial_remote_path.clone(),
            local_files,
            remote_files: vec![],
            active: ActivePane::Local,
            show_transfer_dialog: false,
            transfer_queue: vec![],
            pending: None,
            status: "Loading remote directory…".to_string(),
        };

        Ok(screen)
    }

    fn read_local_dir(path: &PathBuf) -> Result<Vec<FileInfo>> {
        let mut files = vec![];
        if let Some(parent) = path.parent() {
            files.push(FileInfo { name: "..".into(), path: parent.to_path_buf(), is_dir: true, size: 0 });
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
        files[1..].sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });
        Ok(files)
    }

    fn prepare_transfer(&mut self) {
        self.transfer_queue.clear();
        for &i in &self.local_pane.selected {
            if let Some(f) = self.local_files.get(i) {
                self.transfer_queue.push(TransferItem {
                    source: f.path.clone(),
                    destination: self.remote_path.join(&f.name),
                    direction: TransferDirection::Upload,
                });
            }
        }
        for &i in &self.remote_pane.selected {
            if let Some(f) = self.remote_files.get(i) {
                self.transfer_queue.push(TransferItem {
                    source: f.path.clone(),
                    destination: self.local_path.join(&f.name),
                    direction: TransferDirection::Download,
                });
            }
        }
        if !self.transfer_queue.is_empty() {
            self.show_transfer_dialog = true;
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
                vec![Row::new(vec!["Tab: Switch pane", "Space: Select", "t: Transfer", "q: Quit"])],
                vec![Constraint::Fill(1); 4],
            );
            frame.render_widget(help, layout[2]);

            if self.show_transfer_dialog {
                let items: Vec<String> = self.transfer_queue.iter().map(|item| {
                    let arrow = match item.direction {
                        TransferDirection::Upload => "↑",
                        TransferDirection::Download => "↓",
                    };
                    format!("{arrow} {}", item.source.file_name().unwrap_or_default().to_string_lossy())
                }).collect();

                let text = format!(
                    "Transfer {} item(s):\n{}\n\nEnter to confirm · Esc to cancel",
                    items.len(),
                    items.join("\n")
                );

                let dialog_area = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Fill(1), Constraint::Min(10), Constraint::Fill(1)])
                    .split(area)[1];
                let dialog_area = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Fill(1), Constraint::Min(50), Constraint::Fill(1)])
                    .split(dialog_area)[1];

                frame.render_widget(Clear, dialog_area);
                frame.render_widget(
                    Paragraph::new(text).block(
                        Block::default().borders(Borders::ALL).title(" Confirm Transfer ")
                    ),
                    dialog_area,
                );
            }
        })?;
        Ok(())
    }
}

impl Screen<Outcome> for FileManagerScreen {
    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<Outcome> {
        // NOTE: FileManagerScreen::run takes a &SftpClient so it can spawn async operations.
        // The Screen trait signature takes only the terminal. We work around this by having
        // FileManagerApp call a run_with_sftp() method instead of the trait method directly.
        // The trait impl here is a stub that immediately returns Quit.
        Ok(Outcome::Quit)
    }
}

impl FileManagerScreen {
    pub fn run_with_sftp(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<Stdout>>,
        sftp: &SftpClient,
    ) -> Result<Outcome> {
        // Initial remote listing
        let remote_path = self.remote_path.clone();
        let (tx, mut rx) = oneshot::channel();
        {
            // We need Send + 'static for spawn. SftpClient holds an Arc internally via russh.
            // Because we can't clone SftpClient, we do the initial listing synchronously
            // using block_in_place (we are inside a tokio runtime already).
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(sftp.list_directory(&remote_path))
            });
            let _ = tx.send(result);
        }

        loop {
            // Poll pending op
            if let Some(pending) = &mut self.pending {
                match pending {
                    PendingOp::ListRemote(rx) => {
                        match rx.try_recv() {
                            Ok(Ok(files)) => {
                                self.remote_files = files;
                                self.remote_pane.set_item_count(self.remote_files.len());
                                self.status = format!("Remote: {}", self.remote_path.display());
                                self.pending = None;
                            }
                            Ok(Err(e)) => {
                                self.status = format!("Error: {e}");
                                self.pending = None;
                            }
                            Err(_) => {} // still pending
                        }
                    }
                    PendingOp::Transfer(rx) => {
                        match rx.try_recv() {
                            Ok(Ok(())) => {
                                self.local_pane.clear_selection();
                                self.remote_pane.clear_selection();
                                self.local_files = Self::read_local_dir(&self.local_path)?;
                                self.local_pane.set_item_count(self.local_files.len());
                                // Re-list remote
                                let (tx2, rx2) = oneshot::channel();
                                let remote_path = self.remote_path.clone();
                                let result = tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current()
                                        .block_on(sftp.list_directory(&remote_path))
                                });
                                let _ = tx2.send(result);
                                self.pending = Some(PendingOp::ListRemote(rx2));
                                self.status = "Transfer complete.".to_string();
                            }
                            Ok(Err(e)) => {
                                self.status = format!("Transfer error: {e}");
                                self.pending = None;
                            }
                            Err(_) => {} // still pending
                        }
                    }
                }
            }

            // Poll initial listing rx
            if self.remote_files.is_empty() {
                match rx.try_recv() {
                    Ok(Ok(files)) => {
                        self.remote_files = files;
                        self.remote_pane.set_item_count(self.remote_files.len());
                        self.status = format!("Remote: {}", self.remote_path.display());
                    }
                    Ok(Err(e)) => {
                        self.status = format!("Error listing remote: {e}");
                    }
                    Err(_) => {}
                }
            }

            self.draw(terminal)?;

            if !event::poll(Duration::from_millis(50))? {
                continue;
            }

            let event = event::read()?;

            if self.show_transfer_dialog {
                if let Event::Key(key) = &event {
                    match key.code {
                        KeyCode::Enter => {
                            self.show_transfer_dialog = false;
                            let queue = self.transfer_queue.clone();
                            let (tx, rx2) = oneshot::channel();
                            let result = tokio::task::block_in_place(|| {
                                tokio::runtime::Handle::current().block_on(async {
                                    for item in &queue {
                                        let r = match item.direction {
                                            TransferDirection::Upload => {
                                                if item.source.is_dir() {
                                                    sftp.upload_directory(&item.source, &item.destination).await
                                                } else {
                                                    sftp.upload_file(&item.source, &item.destination).await
                                                }
                                            }
                                            TransferDirection::Download => {
                                                if sftp.list_directory(&item.source).await.is_ok() {
                                                    sftp.download_directory(&item.source, &item.destination).await
                                                } else {
                                                    sftp.download_file(&item.source, &item.destination).await
                                                }
                                            }
                                        };
                                        if r.is_err() { return r; }
                                    }
                                    Ok(())
                                })
                            });
                            let _ = tx.send(result);
                            self.pending = Some(PendingOp::Transfer(rx2));
                            self.status = "Transferring…".to_string();
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

            if let Event::Key(key) = &event {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(Outcome::Quit),
                    KeyCode::Tab => {
                        self.active = match self.active {
                            ActivePane::Local => ActivePane::Remote,
                            ActivePane::Remote => ActivePane::Local,
                        };
                    }
                    KeyCode::Char('t') => self.prepare_transfer(),
                    KeyCode::Up | KeyCode::Char('k') => {
                        match self.active {
                            ActivePane::Local => self.local_pane.previous(self.local_files.len()),
                            ActivePane::Remote => self.remote_pane.previous(self.remote_files.len()),
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        match self.active {
                            ActivePane::Local => self.local_pane.next(self.local_files.len()),
                            ActivePane::Remote => self.remote_pane.next(self.remote_files.len()),
                        }
                    }
                    KeyCode::Enter => {
                        match self.active {
                            ActivePane::Local => {
                                if let Some(i) = self.local_pane.current_index() {
                                    if let Some(f) = self.local_files.get(i) {
                                        if f.is_dir {
                                            self.local_path = f.path.clone();
                                            self.local_files = Self::read_local_dir(&self.local_path)?;
                                            self.local_pane.set_item_count(self.local_files.len());
                                            self.local_pane.clear_selection();
                                        }
                                    }
                                }
                            }
                            ActivePane::Remote => {
                                if let Some(i) = self.remote_pane.current_index() {
                                    if let Some(f) = self.remote_files.get(i).cloned() {
                                        if f.is_dir {
                                            self.remote_path = f.path.clone();
                                            let (tx2, rx2) = oneshot::channel();
                                            let remote_path = self.remote_path.clone();
                                            let result = tokio::task::block_in_place(|| {
                                                tokio::runtime::Handle::current()
                                                    .block_on(sftp.list_directory(&remote_path))
                                            });
                                            let _ = tx2.send(result);
                                            self.pending = Some(PendingOp::ListRemote(rx2));
                                            self.remote_pane.clear_selection();
                                            self.status = format!("Loading {}…", self.remote_path.display());
                                        }
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char(' ') => {
                        match self.active {
                            ActivePane::Local => {
                                if let Some(i) = self.local_pane.current_index() {
                                    self.local_pane.toggle_selected(i);
                                }
                            }
                            ActivePane::Remote => {
                                if let Some(i) = self.remote_pane.current_index() {
                                    self.remote_pane.toggle_selected(i);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
```

- [ ] **Step 2: Register in `src/screens.rs`**

Add:

```rust
pub mod file_manager_screen;
```

- [ ] **Step 3: Build to verify**

```bash
cargo build 2>&1
```

Expected: no errors. There may be dead_code warnings on unused `HashSet` import or `FilePaneMessage` variants — fix any that are errors, leave warnings for now.

- [ ] **Step 4: Commit**

```bash
git add src/screens/file_manager_screen.rs src/screens.rs
git commit -m "feat: add FileManagerScreen with dual-pane browser and transfer flow"
```

---

### Task 9: Implement `src/file_manager_app.rs` and wire `launch_file_manager()`

**Files:**
- Create: `src/file_manager_app.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create `src/file_manager_app.rs`**

```rust
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
use crate::sftp::SftpClient;
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
```

- [ ] **Step 2: Register module and update `launch_file_manager()` in `src/main.rs`**

Add the module declaration near the other `mod` statements:

```rust
mod file_manager_app;
use file_manager_app::FileManagerApp;
```

Replace the `launch_file_manager` stub:

```rust
fn launch_file_manager(instance: InstanceInfo) -> Result<()> {
    let mut app = FileManagerApp::new()?;
    app.run(instance)
}
```

- [ ] **Step 3: Build**

```bash
cargo build 2>&1
```

Expected: no errors.

- [ ] **Step 4: End-to-end smoke test**

With an accessible EC2 instance that has sshd running:

```bash
./target/debug/sm_connect -r <region> -i <instance-id> --file-manager
```

Expected:
1. Username prompt screen appears, pre-filled with `$USER`.
2. After entering the correct SSH username and pressing Enter, the connecting spinner appears.
3. After a few seconds, the dual-pane file manager opens — local files on left, remote `/` on right.
4. Tab switches active pane. Arrow keys navigate. Space selects. `t` opens transfer confirmation. `q` exits and the SSM tunnel is cleaned up.

- [ ] **Step 5: Commit**

```bash
git add src/file_manager_app.rs src/main.rs
git commit -m "feat: implement FileManagerApp and wire launch_file_manager()"
```

---

### Task 10: Update README

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add `--file-manager` to Available Options**

In the `### Available Options` section, add:

```markdown
- `-f, --file-manager`: Open an integrated dual-pane file manager connected to the instance over an SSM tunnel. Requires SSH running on the instance (port 22) and ssh-agent running locally with the correct key loaded.
```

- [ ] **Step 2: Add File Manager Mode usage section**

After the `## Tunnel Mode` section, add:

```markdown
## File Manager Mode

To browse and transfer files interactively:

```sh
sm_connect --file-manager
```

Or directly, skipping instance selection:

```sh
sm_connect -r us-east-1 -i i-ad53d5e3831ea --file-manager
```

You will be prompted for the SSH username. After connecting, a dual-pane file manager opens with your local filesystem on the left and the remote instance on the right.

### Key bindings

| Key        | Action                     |
|------------|----------------------------|
| Tab        | Switch active pane         |
| ↑/↓ or j/k | Navigate                   |
| Enter      | Enter directory            |
| Space      | Select/deselect file       |
| t          | Open transfer confirmation |
| q / Esc    | Quit                       |

> **Prerequisites:** SSH must be running on the instance (port 22). `ssh-agent` must be running locally with the correct key: `eval "$(ssh-agent -s)" && ssh-add ~/.ssh/id_rsa`
```

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: document --file-manager flag and file manager mode"
```
