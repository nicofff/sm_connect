# Configurable EC2 / ECS Mode Toggle Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users disable the EC2 or ECS mode they don't use via persistent config; when only one mode is enabled, the interactive flow skips Mode-Select and goes straight into that mode after region selection.

**Architecture:** Add two boolean flags to the existing `Config` (persisted JSON at `~/.sm_connect.json`). The "at least one mode enabled" invariant is enforced in `Config::persist()` (the single write gate); toggles flip-then-revert-on-error so no invariant logic lives in the UI. A hand-edited both-off config is self-healed on load. The `ConfigScreen`/`ConfigList` gain two live-state toggle entries. `app.rs` routing reads the flags in one place (the `RegionSelected` transition) to skip Mode-Select for single-mode setups; all loading-screen cancels simplify to returning to Region select.

**Tech Stack:** Rust (edition 2024), serde/serde_json (config), ratatui 0.30 (TUI). Binary crate — run tests with `cargo test <filter>` (NOT `--lib`).

**Reference spec:** `docs/superpowers/specs/2026-05-27-mode-toggle-design.md`

**Project rule (CLAUDE.md):** NEVER build with `--release` — the AWS crates are slow to compile. Use plain `cargo build` / `cargo test`.

---

## File structure

**Modified files:**
- `src/app/config.rs` — two `bool` flags, `persist()` write-gate, accessors, toggles, load self-heal, unit tests.
- `src/components/config_list.rs` — two new `ConfigOption` variants with live-state labels; `ConfigList` gains the config handle.
- `src/screens/config_screen.rs` — pass config to `ConfigList`; handle the two toggle actions.
- `src/app.rs` — `config` field on `App`; inline routing match in `RegionSelected`; loading-screen cancels → `RegionSelect`.

No new files.

---

## Task 1: Config flags, write-gate, toggles, and self-heal

**Files:**
- Modify: `src/app/config.rs`

This is the data layer and is fully unit-testable. The tests are written to do **no filesystem IO**: they construct `Config::default()` (no IO) and set private fields directly (the test module is in the same file, so private fields are accessible), and the refusal test relies on `persist()` validating *before* it touches the filesystem.

- [ ] **Step 1: Write the failing tests**

Add this to the END of `src/app/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_valid_rejects_both_disabled() {
        let mut c = Config::default();
        assert!(c.ensure_valid().is_ok()); // both enabled by default
        c.ec2_enabled = false;
        c.ecs_enabled = false;
        assert!(c.ensure_valid().is_err());
    }

    #[test]
    fn toggle_refuses_to_disable_last_mode_and_reverts() {
        let mut c = Config::default();
        c.ecs_enabled = false; // only EC2 left enabled (direct set, no IO)
        let result = c.toggle_ec2(); // would disable the last mode
        assert!(result.is_err());
        assert!(c.is_ec2_enabled()); // reverted in memory
    }

    #[test]
    fn heal_reenables_both_when_both_disabled() {
        let mut c = Config::default();
        c.ec2_enabled = false;
        c.ecs_enabled = false;
        assert!(c.heal_disabled_modes());
        assert!(c.is_ec2_enabled() && c.is_ecs_enabled());
    }

    #[test]
    fn heal_is_noop_when_a_mode_is_enabled() {
        let mut c = Config::default();
        c.ecs_enabled = false; // ec2 still enabled
        assert!(!c.heal_disabled_modes());
        assert!(c.is_ec2_enabled());
        assert!(!c.is_ecs_enabled());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test config::tests 2>&1 | tail -20`
Expected: FAIL — compile errors (`ec2_enabled`/`ecs_enabled` fields, `ensure_valid`, `toggle_ec2`, `is_ec2_enabled`, `heal_disabled_modes` don't exist yet).

- [ ] **Step 3: Add the fields and a serde default helper**

In `src/app/config.rs`, replace the `Config` struct definition:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    recent_timeout: u64,
    regions: HashMap<String, RegionConfig>,
}
```

with:

```rust
fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    recent_timeout: u64,
    regions: HashMap<String, RegionConfig>,
    #[serde(default = "default_true")]
    ec2_enabled: bool,
    #[serde(default = "default_true")]
    ecs_enabled: bool,
}
```

(The `#[serde(default = ...)]` makes existing `~/.sm_connect.json` files that predate these fields load fine, defaulting both to enabled.)

- [ ] **Step 4: Default both flags to enabled**

In the `impl Default for Config` block, change the returned struct literal from:

```rust
        Config {
            regions,
            recent_timeout: DEFAULT_RECENT_TIMEOUT,
        }
```

to:

```rust
        Config {
            regions,
            recent_timeout: DEFAULT_RECENT_TIMEOUT,
            ec2_enabled: true,
            ecs_enabled: true,
        }
```

- [ ] **Step 5: Add the write-gate to `persist()`**

In `src/app/config.rs`, change the start of `persist()` from:

```rust
    pub fn persist(&self) -> Result<()> {
        let config_path = Config::get_config_path()?;
```

to:

```rust
    pub fn persist(&self) -> Result<()> {
        self.ensure_valid()?;
        let config_path = Config::get_config_path()?;
```

Then add the `ensure_valid` helper (place it just above `persist`, inside `impl Config`):

```rust
    /// The single invariant gate, enforced on every write: at least one of the
    /// EC2 / ECS modes must remain enabled. Checked before any filesystem IO.
    fn ensure_valid(&self) -> Result<()> {
        if !self.ec2_enabled && !self.ecs_enabled {
            return Err(anyhow::anyhow!(
                "at least one mode (EC2 or ECS) must remain enabled"
            ));
        }
        Ok(())
    }
```

- [ ] **Step 6: Add accessors, toggles, and the self-heal helper**

Add these methods inside `impl Config` (e.g. after `ensure_valid`):

```rust
    pub fn is_ec2_enabled(&self) -> bool {
        self.ec2_enabled
    }

    pub fn is_ecs_enabled(&self) -> bool {
        self.ecs_enabled
    }

    /// Flips the EC2 flag and persists. If the write is rejected (would leave no
    /// enabled mode), the flip is reverted and the error propagated.
    pub fn toggle_ec2(&mut self) -> Result<()> {
        self.ec2_enabled = !self.ec2_enabled;
        if let Err(e) = self.persist() {
            self.ec2_enabled = !self.ec2_enabled;
            return Err(e);
        }
        Ok(())
    }

    /// Flips the ECS flag and persists. If the write is rejected (would leave no
    /// enabled mode), the flip is reverted and the error propagated.
    pub fn toggle_ecs(&mut self) -> Result<()> {
        self.ecs_enabled = !self.ecs_enabled;
        if let Err(e) = self.persist() {
            self.ecs_enabled = !self.ecs_enabled;
            return Err(e);
        }
        Ok(())
    }

    /// Repairs a hand-edited config that disabled both modes by re-enabling both.
    /// Returns true if a change was made (so the caller can persist the fix).
    fn heal_disabled_modes(&mut self) -> bool {
        if !self.ec2_enabled && !self.ecs_enabled {
            self.ec2_enabled = true;
            self.ecs_enabled = true;
            true
        } else {
            false
        }
    }
```

- [ ] **Step 7: Self-heal on load**

In `Config::new`, change the deserialize-and-return block from:

```rust
        let config = match from_str(&contents) {
            Ok(config) => config,
            Err(_) => {
                let config = Config::default();
                config.persist()?;
                config
            }
        };
        Ok(Arc::new(Mutex::new(config)))
```

to:

```rust
        let mut config: Config = match from_str(&contents) {
            Ok(config) => config,
            Err(_) => {
                let config = Config::default();
                config.persist()?;
                config
            }
        };
        // Repair a hand-edited file that disabled both modes.
        if config.heal_disabled_modes() {
            config.persist()?;
        }
        Ok(Arc::new(Mutex::new(config)))
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test config::tests 2>&1 | tail -20`
Expected: PASS — 4 tests pass.

- [ ] **Step 9: Commit**

```bash
git add src/app/config.rs
git commit -m "Add EC2/ECS mode flags with write-gate, toggles, and load self-heal

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 2: ConfigList — live-state toggle entries

**Files:**
- Modify: `src/components/config_list.rs`

No unit test (this is rendering glue; the live-state label is verified in manual testing, Task 5). The real logic lives in `config.rs` (Task 1).

- [ ] **Step 1: Add imports and the two new `ConfigOption` variants**

In `src/components/config_list.rs`, add these imports near the top (below the existing `use` lines):

```rust
use crate::app::config::Config;
use std::sync::{Arc, Mutex};
```

Change the `ConfigOption` enum from:

```rust
#[derive(Debug, Clone, Copy)]
pub enum ConfigOption {
    ResetRecent,
    SetRecentTimeout,
}
```

to:

```rust
#[derive(Debug, Clone, Copy)]
pub enum ConfigOption {
    ResetRecent,
    SetRecentTimeout,
    ToggleEc2,
    ToggleEcs,
}
```

- [ ] **Step 2: Remove the static `From<ConfigOption> for String` and expand the options array**

Delete this block entirely (labels now come from a config-aware method):

```rust
impl From<ConfigOption> for String {
    fn from(option: ConfigOption) -> String {
        match option {
            ConfigOption::ResetRecent => "Reset Recent Instances".to_string(),
            ConfigOption::SetRecentTimeout => "Set Recent Timeout".to_string(),
        }
    }
}
```

Change the options constant from:

```rust
const CONFIG_OPTIONS: [ConfigOption; 2] =
    [ConfigOption::ResetRecent, ConfigOption::SetRecentTimeout];
```

to:

```rust
const CONFIG_OPTIONS: [ConfigOption; 4] = [
    ConfigOption::ResetRecent,
    ConfigOption::SetRecentTimeout,
    ConfigOption::ToggleEc2,
    ConfigOption::ToggleEcs,
];
```

- [ ] **Step 3: Give `ConfigList` the config handle**

Change the struct and `new` from:

```rust
#[derive(Debug)]
pub struct ConfigList {
    state: ListState,
}

impl ConfigList {
    pub fn new() -> ConfigList {
        let mut state = ListState::default();
        state.select(Some(0));
        ConfigList { state }
    }
```

to:

```rust
#[derive(Debug)]
pub struct ConfigList {
    state: ListState,
    config: Arc<Mutex<Config>>,
}

impl ConfigList {
    pub fn new(config: Arc<Mutex<Config>>) -> ConfigList {
        let mut state = ListState::default();
        state.select(Some(0));
        ConfigList { state, config }
    }

    /// Display label for an option. The mode toggles show their current state.
    fn label(&self, option: ConfigOption) -> String {
        match option {
            ConfigOption::ResetRecent => "Reset Recent Instances".to_string(),
            ConfigOption::SetRecentTimeout => "Set Recent Timeout".to_string(),
            ConfigOption::ToggleEc2 => {
                let enabled = self.config.lock().unwrap().is_ec2_enabled();
                format!("EC2 mode: {}", if enabled { "enabled" } else { "disabled" })
            }
            ConfigOption::ToggleEcs => {
                let enabled = self.config.lock().unwrap().is_ecs_enabled();
                format!("ECS mode: {}", if enabled { "enabled" } else { "disabled" })
            }
        }
    }
```

- [ ] **Step 4: Render labels via the new method**

In `get_list`, change the item mapping from:

```rust
            .map(|i| {
                let name: String = (*i).into();
                ListItem::new(name)
            })
```

to:

```rust
            .map(|i| {
                let name = self.label(*i);
                ListItem::new(name)
            })
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build 2>&1 | tail -20`
Expected: builds. There will be an error at the `ConfigScreen::new` call site (`ConfigList::new()` now needs an argument) — that is fixed in Task 3. If `cargo build` errors ONLY about `ConfigList::new` arity in `config_screen.rs`, this task is correct. Confirm there are no other errors in `config_list.rs` itself.

- [ ] **Step 6: Commit**

```bash
git add src/components/config_list.rs
git commit -m "Add EC2/ECS mode toggle entries with live state to ConfigList

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 3: ConfigScreen — wire the toggles

**Files:**
- Modify: `src/screens/config_screen.rs`

- [ ] **Step 1: Pass the config handle to `ConfigList`**

In `ConfigScreen::new`, change:

```rust
        let config_list = ConfigList::new();
```

to:

```rust
        let config_list = ConfigList::new(config.clone());
```

(`config: Arc<Mutex<Config>>` is already a parameter of `ConfigScreen::new`.)

- [ ] **Step 2: Handle the two toggle actions**

In the `run` loop, the `ReturnConfig(option)` match currently has arms for `ResetRecent` and `SetRecentTimeout`. Add two arms so the match stays exhaustive over the four `ConfigOption` variants. Change:

```rust
                    Some(ConfigListOutputAction::ReturnConfig(option)) => match option {
                        ConfigOption::ResetRecent => {
                            History::reset().context("Failed to reset history")?;
                            self.last_operation_success = Some(true);
                        }
                        ConfigOption::SetRecentTimeout => {
                            self.modifying_action = Some(ConfigOption::SetRecentTimeout);
                            self.input_active = true;
                            let current_value = self.config.lock().unwrap().get_recent_timeout();
                            self.input_component.set_value(current_value.to_string());
                        }
                    },
```

to:

```rust
                    Some(ConfigListOutputAction::ReturnConfig(option)) => match option {
                        ConfigOption::ResetRecent => {
                            History::reset().context("Failed to reset history")?;
                            self.last_operation_success = Some(true);
                        }
                        ConfigOption::SetRecentTimeout => {
                            self.modifying_action = Some(ConfigOption::SetRecentTimeout);
                            self.input_active = true;
                            let current_value = self.config.lock().unwrap().get_recent_timeout();
                            self.input_component.set_value(current_value.to_string());
                        }
                        ConfigOption::ToggleEc2 => {
                            // Success is silent (the list label flips); a refused
                            // toggle (would disable the last mode) shows the failure banner.
                            match self.config.lock().unwrap().toggle_ec2() {
                                Ok(()) => self.last_operation_success = None,
                                Err(_) => self.last_operation_success = Some(false),
                            }
                        }
                        ConfigOption::ToggleEcs => {
                            match self.config.lock().unwrap().toggle_ecs() {
                                Ok(()) => self.last_operation_success = None,
                                Err(_) => self.last_operation_success = Some(false),
                            }
                        }
                    },
```

- [ ] **Step 2.5: Note on `get_recent_timeout`**

`get_recent_timeout` is currently marked `#[allow(dead_code)]` in `config.rs` but is already called here — leave that attribute as-is; this task does not touch `config.rs`.

- [ ] **Step 3: Verify it compiles and existing tests still pass**

Run: `cargo build 2>&1 | tail -20 && cargo test 2>&1 | tail -6`
Expected: builds with no errors; all tests pass (the 4 new config tests plus the pre-existing 8).

- [ ] **Step 4: Commit**

```bash
git add src/screens/config_screen.rs
git commit -m "Wire EC2/ECS mode toggles into the config screen

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 4: Routing — skip Mode-Select for single-mode setups

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 1: Add imports and a `config` field on `App`**

In `src/app.rs`, add this import (next to the other `use std::...` lines, e.g. after `use std::io::Stdout;`):

```rust
use std::sync::{Arc, Mutex};
```

Change the `App` struct from:

```rust
pub struct App {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    selected_screen: SelectedScreen,
    region_select_screen: RegionSelectScreen,
    instance_selection_screen: InstanceSelectScreen,
    task_select_screen: TaskSelectScreen,
    selected_task: Option<EcsTaskInfo>,
    config_screen: ConfigScreen,
}
```

to (add the `config` field):

```rust
pub struct App {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    selected_screen: SelectedScreen,
    region_select_screen: RegionSelectScreen,
    instance_selection_screen: InstanceSelectScreen,
    task_select_screen: TaskSelectScreen,
    selected_task: Option<EcsTaskInfo>,
    config_screen: ConfigScreen,
    config: Arc<Mutex<config::Config>>,
}
```

In `App::new`, the local `config` already exists (`let config = config::Config::new()?;`, type `Arc<Mutex<Config>>`). Add it to the returned struct literal — change:

```rust
        Ok(App {
            terminal,
            selected_screen: SelectedScreen::RegionSelect,
            region_select_screen,
            instance_selection_screen,
            task_select_screen,
            selected_task: None,
            config_screen,
        })
```

to:

```rust
        Ok(App {
            terminal,
            selected_screen: SelectedScreen::RegionSelect,
            region_select_screen,
            instance_selection_screen,
            task_select_screen,
            selected_task: None,
            config_screen,
            config,
        })
```

(`config` is moved into the struct last; the earlier `RegionSelectScreen::new(config.clone())` and `ConfigScreen::new(config.clone())` calls already clone it, so this is fine.)

- [ ] **Step 2: Inline the mode-aware routing in the `RegionSelected` transition**

In the `SelectedScreen::RegionSelect` arm, change:

```rust
                        region_select_screen::RegionSelectScreenOutcome::RegionSelected(region) => {
                            self.selected_screen = SelectedScreen::ModeSelect(region);
                        }
```

to:

```rust
                        region_select_screen::RegionSelectScreenOutcome::RegionSelected(region) => {
                            let (ec2, ecs) = {
                                let cfg = self.config.lock().unwrap();
                                (cfg.is_ec2_enabled(), cfg.is_ecs_enabled())
                            };
                            self.selected_screen = match (ec2, ecs) {
                                (true, false) => SelectedScreen::LoadingInstances(region),
                                (false, true) => SelectedScreen::LoadingTasks(region),
                                // both enabled (and the impossible both-disabled) -> show the picker
                                _ => SelectedScreen::ModeSelect(region),
                            };
                        }
```

- [ ] **Step 3: Simplify the LoadingInstances cancel target**

In the `SelectedScreen::LoadingInstances(region)` arm, change:

```rust
                        loading_instances_screen::LoadingInstancesScreenOutcome::Cancelled => {
                            self.selected_screen = SelectedScreen::ModeSelect(region);
                        }
```

to:

```rust
                        loading_instances_screen::LoadingInstancesScreenOutcome::Cancelled => {
                            self.selected_screen = SelectedScreen::RegionSelect;
                        }
```

- [ ] **Step 4: Simplify the LoadingTasks cancel target**

In the `SelectedScreen::LoadingTasks(region)` arm, change:

```rust
                        LoadingTasksScreenOutcome::Cancelled => {
                            self.selected_screen = SelectedScreen::ModeSelect(region);
                        }
```

to:

```rust
                        LoadingTasksScreenOutcome::Cancelled => {
                            self.selected_screen = SelectedScreen::RegionSelect;
                        }
```

- [ ] **Step 5: Verify it compiles and tests pass**

Run: `cargo build 2>&1 | tail -20 && cargo test 2>&1 | tail -6`
Expected: builds; all 12 tests pass (8 pre-existing + 4 new config tests). Note: after Step 3/4, the `region` binding in those two arms is now only used for `LoadingInstancesScreen::new(region.clone())` / `LoadingTasksScreen::new(region.clone())` — no unused-variable warning is expected because the constructor still consumes it.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs
git commit -m "Skip Mode-Select when only one mode is enabled; cancels return to Region

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Task 5: Final build, lint, and manual verification

**Files:** none (verification only)

- [ ] **Step 1: Full build + tests + clippy**

Run: `cargo build 2>&1 | tail -3 && cargo test 2>&1 | tail -8 && cargo clippy 2>&1 | tail -5`
Expected: clean build; 12 tests pass (8 existing + 4 new); no clippy warnings.

- [ ] **Step 2: Manual verification (requires AWS credentials)**

```bash
export AWS_PROFILE=<a-profile>
aws sso login
cargo run
```

Verify, in order:
1. With a fresh/default config (or both modes enabled), Region → Mode-Select appears as before.
2. Open the config screen (`c` from the region list). Two new entries show live state: `EC2 mode: enabled`, `ECS mode: enabled`.
3. Toggle `ECS mode` off — the label flips to `ECS mode: disabled` (no error banner).
4. Try to toggle `EC2 mode` off too — it's refused: the red "Operation failed" banner shows and the label stays `EC2 mode: enabled`.
5. Exit config, pick a region — Mode-Select is **skipped**, going straight to the EC2 instance loading/list.
6. Cancel (Esc) on the loading spinner → returns to Region select.
7. Re-enable ECS (and optionally disable EC2): with only ECS enabled, picking a region goes straight to the ECS task flow.
8. Inspect `~/.sm_connect.json` — it contains `"ec2_enabled"` / `"ecs_enabled"`.
9. Hand-edit the file setting both to `false`, save, relaunch — on load both are re-enabled (self-heal) and the file is rewritten with both `true`.

- [ ] **Step 3: Confirm branch state**

Run: `git status && git log --oneline -5`
Expected: clean working tree; the four feature commits present on the `ecs` branch.

---

## Notes for the implementer

- **Never** build with `--release` (CLAUDE.md).
- The Task 1 tests are deliberately IO-free: `Config::default()` does no IO, private fields are set directly within the same-file test module, and the refusal test relies on `persist()` calling `ensure_valid()` *before* any filesystem access. Do not add tests that call a *successful* `toggle`/`persist` — that would write to the real `~/.sm_connect.json`.
- This feature builds on the `ecs` branch (it depends on `ModeSelect`, `LoadingTasks`, etc.). Stay on `ecs`.
- Out of scope (per spec): CLI flags for the toggles, per-region/per-profile preferences, and the direct `-r -i` CLI connect path (unaffected — it bypasses the TUI).
