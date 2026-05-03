# File Manager Feature Design

## Overview

Add an integrated TUI file manager to `sm_connect` that allows users to browse and transfer files to/from EC2 instances over an SSM-tunneled SFTP connection. No VPN, no persistent SSH config, no extra remote software required beyond a running sshd.

## User Flow

### Interactive mode (primary)

Instance select screen gains two new actions alongside Enter (Connect):
- `t` → Tunnel mode (existing)
- `f` → File Manager mode (new)

After selecting an instance with `f`, the user is taken through:
1. **Username prompt screen** — text input pre-populated with `$USER` as a hint. Enter confirms, q/Esc cancels back to instance select.
2. **Connecting screen** — spins while establishing the SSM tunnel and SFTP connection. Shows meaningful error if SSH isn't running or auth fails.
3. **File manager TUI** — dual-pane browser, local left / remote right.

### Direct CLI mode (secondary)

`--file-manager` flag works alongside `-r` and `-i`, same pattern as `--tunnel`. Behavior when `--file-manager` is passed without `-r`/`-i` is undefined for now (left for a future iteration, possibly via clap subcommands).

---

## Architecture

### `UserAction` enum (`src/app.rs`)

```rust
pub enum UserAction {
    Connect(InstanceInfo),
    Tunnel(InstanceInfo),
    FileManager(InstanceInfo),
}
```

`App::run()` returns `Result<Option<UserAction>>`. `main()` dispatches:
- `Connect` → existing `connect()`
- `Tunnel` → existing `tunnel()`
- `FileManager` → new `launch_file_manager()`

### Instance select changes (`src/components/instance_table.rs`)

New `InstanceTableMessage` variants: `Tunnel`, `FileManager` (mapped from `t` and `f` keypresses).

New `InstanceTableOutputAction` variants:
```rust
ReturnInstanceForTunnel(InstanceInfo),
ReturnInstanceForFileManager(InstanceInfo),
```

Help bar gains: `t: Tunnel  f: File Manager`.

`InstanceSelectScreen::Outcome` gains `Tunnel(InstanceInfo)` and `FileManager(InstanceInfo)` variants to match.

### `src/sftp.rs` (new)

Pure SFTP client, no UI coupling. Ported and cleaned up from sftui — ProxyJump machinery dropped entirely. Auth is always ssh-agent; the tunnel is already established before connection.

```rust
pub struct FileInfo {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

pub struct SftpClient { ... }

impl SftpClient {
    pub fn connect(host: &str, port: u16, user: &str) -> Result<Self>
    pub fn list_directory(&self, path: &Path) -> Result<Vec<FileInfo>>
    pub fn download_file(&self, remote: &Path, local: &Path) -> Result<()>
    pub fn upload_file(&self, local: &Path, remote: &Path) -> Result<()>
    pub fn download_directory(&self, remote: &Path, local: &Path) -> Result<()>
    pub fn upload_directory(&self, local: &Path, remote: &Path) -> Result<()>
}
```

Improvements over sftui:
- `upload_file`/`download_file` use `std::io::copy` instead of manual buffer loops.
- `download_directory` added (missing from sftui).

### `src/file_manager_app.rs` (new)

Owns its own terminal instance. Fully isolated from the main `App`.

```rust
pub struct FileManagerApp {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}
```

Constructor calls `setup_terminal()`. `Drop` impl calls `restore_terminal()`.

Internal state machine:
```rust
enum State {
    PromptUsername(InstanceInfo),
    Connecting(InstanceInfo, String),   // instance + username
    FileManager(InstanceInfo, String, SftpClient),
}
```

`Connecting` state:
1. Picks a free local port (port-0 trick, same as `tunnel()`).
2. Spawns SSM port-forwarding process in the background (non-blocking).
3. Polls `TcpStream::connect("localhost:N")` — 500ms interval, 10s total timeout.
4. On success: constructs `SftpClient::connect("localhost", N, user)`.
5. On timeout: prints error, kills SSM process, exits.
6. SSM process is killed when `FileManagerApp` is dropped.

`main.rs` gains:
```rust
fn launch_file_manager(instance: InstanceInfo) -> Result<()>
```

### New screens

**`src/screens/username_prompt_screen.rs`**
- Reuses existing `TextInput` component.
- Layout: centered label + input field.
- `Outcome::Username(String)` or `Outcome::Cancel`.

**`src/screens/connecting_screen.rs`**
- Mirrors `LoadingInstancesScreen`.
- Spinner with status messages: "Establishing SSM tunnel…" → "Connecting via SFTP…".
- `Outcome::Connected(SftpClient)` or `Outcome::Failed(String)`.
- The SSM process handle is owned by `FileManagerApp`, not the screen. `ConnectingScreen` receives the spawned `Child` as input and returns it (or drops it on failure) — `FileManagerApp` holds it for the duration and kills it on drop.

**`src/screens/file_manager_screen.rs`**
- Contains two `FilePaneComponent` instances (local left, remote right).
- Owns transfer queue state.
- `Outcome::Quit`.
- Transfer flow: Space selects files, `t` opens confirmation overlay (same overlay pattern as search), Enter executes transfers, Esc cancels.

### New component

**`src/components/file_pane.rs`**
- Stateless ratatui widget rendering a file list with cursor highlight and selection markers (`[*]` prefix or similar).
- Implements `Component` with messages: `Up`, `Down`, `Enter`, `Select`.
- Same structure as `InstanceTable`. Two instances in `FileManagerScreen`, one per pane.

---

## Key bindings (file manager screen)

| Key      | Action                        |
|----------|-------------------------------|
| Tab      | Switch active pane            |
| ↑/↓ or j/k | Navigate                  |
| Enter    | Enter directory               |
| Space    | Select/deselect file          |
| t        | Open transfer confirmation    |
| q / Esc  | Quit file manager             |

---

## Dependencies

Add to `Cargo.toml`:
```toml
ssh2 = "0.9"
```

Requires OpenSSL on the build machine (same constraint as sftui). On macOS: `brew install openssl@3`.

---

## Future work (out of scope for this iteration)

- Config value for default SSH username per instance/region.
- `--file-manager` without `-r`/`-i` (possibly via clap subcommands).
- Search within file panes.
- Progress bar during file transfers.
