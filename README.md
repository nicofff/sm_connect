Session Manager Connect
=======================

Session Manager Connect is a TUI to simplify using AWS Systems Manager's Session Manager to connect to EC2 instances

# Why?
If you have a more than a few of EC2 servers, you usually do two things:
1. Go to the AWS console, and get the ip address to connect to the server
2. SSH in, usually this means connecting to a VPN, which is annoying to have to maintain

What if you could save you the VPN, and the looking up the instance information?
This is what this tool does for you.
It leverages AWS Session Manager to connect to your EC2 instances, which doesn't require you having network connectivity to the instance.
Also, it removes the complexity of connecting to it, by providing an easy way to find which server you want to connect to, and piping out the the correct AWS CLI command.
Bonus points for not needing SSH anymore.

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
1. Select the __region__ that contains your instance.
2. Select the __instance__ you want to connect to.
4. __Connect__ and enjoy!

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
- `-h, --help`: Print help information
- `-V, --version`: Print version information

[aws-cli-install]: https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html
[aws-sm-install]: https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager-working-with-install-plugin.html
[aws-sm-config]: https://docs.aws.amazon.com/systems-manager/latest/userguide/session-manager-getting-started.html
