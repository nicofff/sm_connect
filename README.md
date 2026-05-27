Session Manager Connect
=======================

Session Manager Connect is a TUI to simplify using AWS Systems Manager's Session Manager to connect to EC2 instances. It also supports Exec'ing into ECS tasks

![Demo](docs/demo.gif)

# Why?
If you have a more than a few of EC2 servers, you usually do two things:
1. Go to the AWS console, and get the ip address to connect to the server
2. SSH in, usually this means connecting to a VPN, which is annoying to have to maintain

What if you could save you the VPN, and the looking up the instance information?
This is what this tool does for you.
It leverages AWS Session Manager to connect to your EC2 instances, which doesn't require you having network connectivity to the instance.
Also, it removes the complexity of connecting to it, by providing an easy way to find which server you want to connect to, and piping out the the correct AWS CLI command.
Bonus points for not needing SSH anymore.

# AI Usage Disclaimer
Most of the tool was written before AI became mainstream. I've been using Claude lately.
There core of the tool is hand-written, the latest aditions (like the file manager) have been built with AI assistance

# Install

```sh
cargo install sm_connect
```

If you don't have Rust installed, get it from [rustup.rs](https://rustup.rs).

# Prerequisites

- You must have the `aws` CLI [installed][aws-cli-install].
- You must [install][aws-sm-install] AWS Session Manager plugin.
- You must [configure][aws-sm-config] your instances to allow connections from Session Manager.

# Usage

## Interactive Mode

```sh
export AWS_PROFILE=my-profile
aws sso login
sm_connect
```

1. The `sm_connect` TUI will launch.
1. Select the __region__ that contains your instance or task.
1. Choose a __mode__: EC2 (Session Manager) or ECS (Exec).
1. For EC2: select the __instance__ and __connect__.
1. For ECS: select a __task__, then a __container__ (skipped if the task has only one), and a shell opens via `aws ecs execute-command`.

## Command-line Arguments

You can also connect directly to an instance by providing the region and instance ID as command-line arguments:

```sh
export AWS_PROFILE=my-profile
aws sso login
sm_connect --region us-east-1 --instance i-ad53d5e3831ea
```

Or using the short form:

```sh
sm_connect -r us-east-1 -i i-ad53d5e3831ea
```

### Available Options

- `-r, --region <REGION>`: AWS region (e.g., us-east-1, us-west-2)
- `-i, --instance <INSTANCE>`: AWS EC2 instance ID (e.g., i-ad53d5e3831ea)
- `-t, --tunnel`: Forward the instance's SSH port (22) over SSM to a random local port, then print the SFTP URL for use with a file manager. Requires SSH running on the instance; authentication is handled by your file manager.
- `-f, --file-manager`: Open an integrated dual-pane file manager connected to the instance over an SSM tunnel. Requires SSH running on the instance (port 22) and ssh-agent running locally with the correct key loaded.
- `-h, --help`: Print help information
- `-V, --version`: Print version information

## Tunnel Mode

To connect a file manager to an instance without a VPN:

```sh
sm_connect --tunnel
```

Or directly, skipping instance selection:

```sh
sm_connect -r us-east-1 -i i-ad53d5e3831ea --tunnel
```

`sm_connect` prints an SFTP URL (e.g. `sftp://localhost:52341`) that you can open in any file manager (Cyberduck, Transmit, Finder, Midnight Commander, etc.). Press Ctrl+C when done to close the tunnel.

> **Prerequisites:** SSH must be running on the instance (port 22). Your file manager handles authentication using your existing SSH credentials.

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

## ECS Exec Mode

After selecting a region, choose **ECS (Exec)** to browse running ECS tasks across all
clusters in the region. Pick a task, then a container, and `sm_connect` opens an
interactive `/bin/sh` shell in that container via `aws ecs execute-command`. If a task
has a single container, the container step is skipped.

> **Prerequisites:**
> - The [Session Manager plugin][aws-sm-install] must be installed (same as EC2 mode).
> - The task's service/task must be launched with **ECS Exec enabled**
>   (`enableExecuteCommand`).
> - The task role must grant the SSM permissions ECS Exec requires
>   (`ssmmessages:*`).
>
> See the [ECS Exec documentation][ecs-exec] for setup details.

[aws-cli-install]: https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html
[aws-sm-install]: https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager-working-with-install-plugin.html
[aws-sm-config]: https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager-getting-started.html
[ecs-exec]: https://docs.aws.amazon.com/AmazonECS/latest/developerguide/ecs-exec-run.html
