# ECS Exec Support — Design

## Goal

Let users connect into a running ECS task's container via `aws ecs execute-command`,
alongside the existing EC2 / Session Manager flow. The interactive TUI gains a path:
pick a region, choose the ECS mode, search the region's running tasks, pick a task,
and (when needed) pick a container — then `sm_connect` shells out to the AWS CLI to
open an interactive shell in that container.

This mirrors the existing EC2 flow (`Region → LoadingInstances → InstanceSelect →
connect`) rather than abstracting the working EC2 code into shared generics.

## User flow

```
RegionSelect (existing)
  └─> ModeSelect (NEW)
        ├─ "EC2 (Session Manager)" ─> LoadingInstances → InstanceSelect → Connect/Tunnel/FileManager  (existing)
        └─ "ECS (Exec)" ───────────> LoadingTasks (NEW) → TaskSelect (NEW) → ContainerSelect (NEW) → EcsExec
```

The final ECS action shells out to:

```
aws --region <REGION> ecs execute-command \
  --cluster <CLUSTER> --task <TASK> --container <CONTAINER> \
  --interactive --command "/bin/sh"
```

If the selected task has exactly **one** container, the ContainerSelect screen is
skipped and exec starts immediately on that container.

## Data model (`src/aws.rs`)

New struct:

```rust
pub struct EcsTaskInfo {
    region: Region,
    cluster: String,          // cluster name or ARN (whatever execute-command accepts);
                              //   the Cluster column displays the short name parsed from the ARN
    task_id: String,          // short id for display
    task_arn: String,         // passed to --task
    task_definition: String,  // family:revision, for display
    service: String,          // ECS service name (from task `group`), for display; may be empty
    last_status: String,      // e.g. RUNNING
    containers: Vec<String>,  // container names; drives ContainerSelect
}
```

Container names ride along on the task, so ContainerSelect needs no extra AWS call.

New async fetch:

```rust
pub async fn fetch_ecs_tasks(region: Region) -> Result<Vec<EcsTaskInfo>>
```

Steps:
1. `list_clusters` — enumerate all clusters in the region.
2. For each cluster, `list_tasks` (paginated) with `desired_status = RUNNING`.
3. `describe_tasks` in batches of ≤100 to pull `group` (service name), task definition,
   `last_status`, and `containers[].name`.
4. Flatten into `Vec<EcsTaskInfo>`.

Uses `aws-sdk-ecs` (new dependency, sibling to the existing `aws-sdk-ec2`).

## New screens (`src/screens/`)

- **`mode_select_screen.rs`** — choose EC2 vs ECS. Returns which branch to take.
- **`loading_tasks_screen.rs`** — mirrors `loading_instances_screen.rs`; spawns a
  tokio task running `fetch_ecs_tasks`, drives the generic `Loader<Result<Vec<EcsTaskInfo>>>`.
  Outcome: `Cancelled` | `TasksFetched(Vec<EcsTaskInfo>)`.
- **`task_select_screen.rs`** — mirrors `instance_select_screen.rs`: a searchable table
  plus the existing `TextInput` search overlay. Outcome: `Exit` | `Exec(EcsTaskInfo)`.
- **`container_select_screen.rs`** — lists the selected task's containers.
  Outcome: `Exit` | `Return(container_name)`.

## New components (`src/components/`)

- **`task_table.rs`** — modeled on `instance_table.rs`. Columns: Cluster, Task ID,
  Task Definition, Service, Status, Containers (count). Searchable/filterable across
  those fields like the instance table. Output action: `ReturnTask(EcsTaskInfo)`,
  plus `Search` / `Exit` to match the instance table's interaction model.
- **`simple_list.rs`** — minimal single-select string list (up / down / enter / exit).
  Reused by both `mode_select_screen` and `container_select_screen` to avoid
  duplicating two near-identical list components.

## `app.rs` wiring

`SelectedScreen` gains: `ModeSelect`, `LoadingTasks(String /* region */)`,
`TaskSelect`, `ContainerSelect`. After a region is chosen, `RegionSelect` now routes
to `ModeSelect` (instead of straight to `LoadingInstances`). New match arms drive the
ECS screens; the ECS task selected on `TaskSelect` is carried into `ContainerSelect`,
and the resulting (task, container) becomes `UserAction::EcsExec`.

New `UserAction` variant:

```rust
EcsExec { task: EcsTaskInfo, container: String }
```

`main.rs` gains the matching arm that builds and runs the `ecs execute-command` via
the existing `run_aws_command` helper (so SIGINT/SIGTSTP pass-through still works).

## Header (`src/components/header_tabs.rs`)

The header "tabs" are really a flow/progress indicator, not interactive tabs. Make
`HeaderTabs` accept its tab set so the ECS path shows its own steps
(`Region / Mode / Tasks / Container`) while EC2 keeps `Region / Instances / Connection`.
Two existing call sites (`region_select_screen`, `instance_select_screen`) are updated
to pass their tab set.

## Error handling

- `fetch_ecs_tasks` returns `Result`; the loader screen surfaces failures the same way
  `loading_instances_screen` does ("Failed to fetch tasks. Check your AWS credentials.").
- ECS exec prerequisites (`enableExecuteCommand` on the service/task, task-role IAM
  permissions, the SSM Session Manager plugin) are **not** pre-validated; if they are
  missing the AWS CLI prints its own error after `sm_connect` exits the TUI. This
  matches how the EC2 path already defers to the CLI.

## Testing

- Follow the repo's existing testing conventions (the EC2 screens/components are not
  unit-tested; logic-bearing helpers such as `fetch_ecs_tasks` flattening and the
  task-table filter are the candidates for targeted tests if the codebase supports it).
- Manual verification: run against a real account with at least one ECS cluster that
  has exec-enabled tasks; confirm region → mode → tasks → container → shell works,
  and that the single-container auto-skip behaves.

## Documentation

Update `README.md`: add ECS exec to the feature description and add an ECS exec
prerequisites note (`enableExecuteCommand`, task-role IAM, SSM Session Manager plugin).

## Out of scope (v1)

- **Configurable exec command** — hardcoded `/bin/sh`. CLI/config override is a later
  addition.
- **Direct ECS CLI args** (`--cluster` / `--task` / `--container`) — interactive flow
  only for now.
- **History / last-access for ECS** — task IDs are ephemeral, so last-access sorting
  is not meaningful; omitted.
- **ECS port-forwarding / tunnel / file-manager** for tasks — EC2-only for now.
