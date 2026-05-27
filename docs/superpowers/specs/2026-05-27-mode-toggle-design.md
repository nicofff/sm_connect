# Configurable EC2 / ECS Mode Toggle — Design

## Goal

Let users disable the EC2 (Session Manager) or ECS (Exec) mode they don't use, via
persistent config. When only one mode is enabled, the interactive flow skips the
Mode-Select screen and goes straight into that mode after region selection.

Builds on the ECS Exec feature (the `ecs` branch) — it depends on the `ModeSelect`
screen and the EC2/ECS branch existing.

## Behavior

- Both modes enabled (default): unchanged — Region → Mode-Select → chosen flow.
- One mode disabled: Region → straight into the enabled mode's flow (Mode-Select is
  never rendered). Disabling a mode actually saves a step.
- The "both disabled" state is forbidden (see invariant below).

## Config (`src/app/config.rs`)

Add two fields to `Config`:

```rust
ec2_enabled: bool,  // default true
ecs_enabled: bool,  // default true
```

- Serialized with serde. Both use `#[serde(default = "default_true")]` so existing
  `~/.sm_connect.json` files written before this feature load fine and default to
  enabled. `Default for Config` also sets both to `true`.

### Invariant enforced at write time

`Config::persist()` is the single gatekeeper. At the top of `persist()`:

```rust
if !self.ec2_enabled && !self.ecs_enabled {
    return Err(anyhow!("at least one mode (EC2 or ECS) must remain enabled"));
}
```

Nothing is written when this trips. Since no other setter touches the mode flags, this
only ever blocks a both-off write. The invariant lives in the data layer, not the UI.

### Methods

```rust
pub fn is_ec2_enabled(&self) -> bool
pub fn is_ecs_enabled(&self) -> bool
pub fn toggle_ec2(&mut self) -> Result<()>
pub fn toggle_ecs(&mut self) -> Result<()>
```

Each toggle flips its flag in memory, calls `persist()`, and **reverts the flip if
persist returns `Err`**, propagating the error. The toggle contains no invariant logic
— it just undoes a rejected write:

```rust
pub fn toggle_ecs(&mut self) -> Result<()> {
    self.ecs_enabled = !self.ecs_enabled;
    if let Err(e) = self.persist() {
        self.ecs_enabled = !self.ecs_enabled; // revert
        return Err(e);
    }
    Ok(())
}
```

### Load-time self-heal

A hand-edited config with both flags `false` is repaired on load. In `Config::new`,
after deserializing, if `!ec2_enabled && !ecs_enabled`, set both to `true` and persist
(which passes the gate, since we're writing both-on). This does not fight the write
gate — the gate only rejects writes that would *store* both-off.

## Config screen (`src/components/config_list.rs`, `src/screens/config_screen.rs`)

Two new `ConfigOption` entries with live state in the label, e.g.:

```
EC2 mode: enabled
ECS mode: disabled
```

- `ConfigList` takes the `Arc<Mutex<Config>>` handle (the same way `RegionList` already
  does) so it can render the current on/off state in those two labels.
- Pressing Enter on an entry calls `config.toggle_ec2()` / `toggle_ecs()`.
- On `Ok`, the list re-renders with the flipped state. On `Err` (refused — would
  disable the last mode), reuse the existing red "Operation failed" banner
  (`last_operation_success = Some(false)`). No invariant logic in the screen.

## Routing (`src/app.rs`)

Add a `config: Arc<Mutex<Config>>` field to `App` (cloned from the same handle already
shared with the region/config screens).

Replace the `RegionSelected(region)` transition (currently always `ModeSelect(region)`)
with an inline match on the enabled modes:

```rust
RegionSelectScreenOutcome::RegionSelected(region) => {
    let cfg = self.config.lock().unwrap();
    self.selected_screen = match (cfg.is_ec2_enabled(), cfg.is_ecs_enabled()) {
        (true, false) => SelectedScreen::LoadingInstances(region),
        (false, true) => SelectedScreen::LoadingTasks(region),
        _             => SelectedScreen::ModeSelect(region), // both (none is impossible)
    };
}
```

Loading-screen cancels are simplified to always return to `RegionSelect`:

- `LoadingInstancesScreenOutcome::Cancelled` → `RegionSelect` (was `ModeSelect`)
- `LoadingTasksScreenOutcome::Cancelled` → `RegionSelect` (was `ModeSelect`)

`InstanceSelect` / `TaskSelect` exit already go to `RegionSelect` — unchanged. With all
cancels/exits returning to Region, no skip-induced loop is possible and the config check
exists in exactly one place.

## Testing

Unit tests on `src/app/config.rs` (pure logic, no AWS/TUI):

- `persist` (or a toggle through it) rejects disabling the last enabled mode, and the
  in-memory flag is reverted to its prior value.
- Toggling a mode while the other stays enabled succeeds and flips state.
- Load-time self-heal: a config with both flags false ends up with both true.

## Out of scope

- CLI flags to set the toggles (config + config screen only).
- Per-region or per-profile mode preferences (global only).
- Changing the direct `-r -i` CLI connect path (it bypasses the TUI entirely and is
  unaffected).
