use std::time::Duration;
use tokio::time;

/// This example executes two simple shell commands concurrently on an
/// existing SSH connection.
///
/// To create the SSH control socket, add the following to your ~/.ssh/config:
/// ```
/// Host dummy
///   HostName some_test_machine
///   ControlMaster auto
///   ControlPath /tmp/test.sock
/// ```
/// Then execute `ssh dummy`.
#[tokio::main(flavor = "current_thread")]
pub async fn main() -> Result<(), String> {
    let socket_path = "/tmp/test.sock";

    let (result1, result2) =
        match time::timeout(Duration::from_secs(6), async {
            tokio::join!(
                ssh_muxcontrol::run(socket_path, "sleep 5 && echo -n Hello\n"),
                ssh_muxcontrol::run(
                    socket_path,
                    "sleep 5 && echo -n World >&2\n"
                ),
            )
        })
        .await
        {
            Err(e) => return Err(format!("timeout: {:?}", e)),
            Ok((x, y)) => (x, y),
        };

    let result1 = result1.map_err(|e| format!("Session 1 failed: {}", e))?;
    let result2 = result2.map_err(|e| format!("Session 2 failed: {}", e))?;

    let stdout1 = String::from_utf8(result1.stdout)
        .map_err(|e| format!("Converting stdout to UTF8 failed: {}", e))?;

    let stderr2 = String::from_utf8(result2.stderr)
        .map_err(|e| format!("Converting stderr to UTF8 failed: {}", e))?;

    println!(
        "Command returned '{} {}', first exit code was: {}",
        stdout1, stderr2, result1.exit_code
    );

    Ok(())
}
