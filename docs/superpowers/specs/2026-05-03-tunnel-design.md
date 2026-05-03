# SSH Tunnel Feature Design

## Overview

Add a `--tunnel` flag to `sm_connect` that, instead of dropping into an interactive SSM shell session, establishes an SSH port-forwarding tunnel over SSM. This allows the user to connect their own file manager (or any SFTP client) to the instance without VPN, without touching system SSH config, and without any additional tooling on the remote.

## CLI Interface

`--tunnel` / `-t` is an optional boolean flag added to the existing `Args` struct. It works in both interactive and direct modes:

```
sm_connect --tunnel                              # interactive instance selection, then tunnel
sm_connect -r us-east-1 -i i-abc123 --tunnel    # direct tunnel to specific instance
```

Instance selection (TUI or direct args) is identical to the current flow. The flag only affects what happens after an instance is chosen.

## Behavior: tunnel mode

After instance selection, `sm_connect` spawns:

```
aws --region <region> ssm start-session \
    --target <instance-id> \
    --document-name AWS-StartPortForwardingSession \
    --parameters '{"portNumber":["22"],"localPortNumber":["<N>"]}'
```

Where `<N>` is a randomly chosen available local port (search from e.g. 2222 upward, bind-test to confirm availability).

The process runs in the foreground. Once spawned, print to stdout:

```
SSH tunnel ready — connect your file manager to sftp://localhost:<N>
Press Ctrl+C to close the tunnel.
```

Then wait on the child process. SIGINT and SIGTSTP are caught and forwarded (same pattern as the existing `connect()` function) so Ctrl+C gracefully terminates the SSM session rather than killing `sm_connect` itself.

When the child exits, the tunnel is closed. No remote cleanup is needed — the existing SSH daemon on the remote is untouched.

## Prerequisites

- Instance must have SSH running (port 22) and the user must have credentials their file manager can use independently. The tool makes no attempt to manage SSH auth.
- The SSM port forwarding document (`AWS-StartPortForwardingSession`) must be available in the region (it is an AWS-managed document, available in all commercial regions).
- The IAM role/user must have `ssm:StartSession` permission with the port forwarding document.

## Implementation notes

- `connect()` and `tunnel()` in `main.rs` share the same signal-handling pattern; consider extracting a `run_aws_command(args) -> Result<()>` helper to avoid duplication.
- Port selection: try binding a `TcpListener` on candidate ports starting at 2222; use the first that succeeds.

## Future: ephemeral sshd path

Not implemented in this iteration. The `--tunnel` flag is designed to accommodate it later.

The intended design for users who do not have SSH set up on the remote:

1. Generate a temporary Ed25519 key pair locally (written to a tmpdir, deleted on exit).
2. Inject the public key via `aws ssm send-command` using the `AWS-RunShellScript` document, writing it to a temp `authorized_keys` file in `/tmp/sm_connect_<session>/`.
3. Write a minimal `sshd_config` to `/tmp/sm_connect_<session>/` — custom port (e.g. 2222), `AuthorizedKeysFile` pointing to the temp file, `HostKey` pointing to a generated temp host key, `PermitRootLogin yes` (ssm-user is a sudoer), no PAM, no system auth.
4. Launch `/usr/sbin/sshd -f /tmp/sm_connect_<session>/sshd_config -D -p <remote-port>` via `send-command` (runs as root via sudo).
5. Forward that remote port via SSM port forwarding to a local port.
6. Print connection details including the path to the ephemeral private key.
7. On Ctrl+C: send an SSM command to kill the remote sshd process and `rm -rf /tmp/sm_connect_<session>/`.

This path requires no changes to the system SSH installation. The custom sshd is fully isolated in `/tmp`.
