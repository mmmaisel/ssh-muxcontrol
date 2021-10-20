use crate::session::{run, ShellResult};
use crate::SshctlError;
use tokio::time::{self, Duration};

/*
 * Write the following to ~/.ssh/config for testing:
 *
 * Host dummy
 *     HostName some_test_machine
 *     ControlMaster auto
 *     ControlPath /tmp/test.sock
 *
 * Then execute:
 *   ssh dummy
 */

const TEST_SOCKET: &'static str = "/tmp/test.sock";

#[tokio::test]
async fn test_connect_echo() -> Result<(), SshctlError> {
    let expectation = ShellResult {
        stdout: "asdf\n".into(),
        stderr: "".into(),
        exit_code: 0,
    };

    assert_eq!(expectation, run(TEST_SOCKET, "echo asdf\n").await?);
    Ok(())
}

#[tokio::test]
async fn test_abort_cmd_with_timeout() -> Result<(), SshctlError> {
    match time::timeout(
        Duration::from_secs(1),
        run(TEST_SOCKET, "sleep 5 && echo asdf\n"),
    )
    .await
    {
        Err(e) => println!("timeout: {:?}", e),
        Ok(x) => match x {
            Ok(x) => panic!("not timed out: {:?}", x),
            Err(e) => panic!("not timed out: {:?}", e),
        },
    }

    let expectation = ShellResult {
        stdout: "after timeout\n".into(),
        stderr: "".into(),
        exit_code: 0,
    };

    assert_eq!(expectation, run(TEST_SOCKET, "echo after timeout\n").await?);
    Ok(())
}

#[tokio::test]
async fn test_read_large_data() -> Result<(), SshctlError> {
    let result = run(
        TEST_SOCKET,
        "cat /dev/urandom | tr -dc 'a-zA-Z0-9' | fold -w 8192 | head -n 1\n",
    )
    .await?;
    assert_eq!(8193, result.stdout.len());
    Ok(())
}

#[tokio::test]
async fn test_parallel_commands() -> Result<(), SshctlError> {
    let expectation1 = ShellResult {
        stdout: "1234\n".into(),
        stderr: "".into(),
        exit_code: 0,
    };
    let expectation2 = ShellResult {
        stdout: "".into(),
        stderr: "2345\n".into(),
        exit_code: 0,
    };
    let expectation3 = ShellResult {
        stdout: "".into(),
        stderr: "".into(),
        exit_code: 1,
    };

    let (result1, result2, result3) =
        match time::timeout(Duration::from_secs(6), async {
            tokio::join!(
                run(TEST_SOCKET, "sleep 5 && echo 1234\n"),
                run(TEST_SOCKET, "sleep 5 && echo 2345 >&2\n"),
                run(TEST_SOCKET, "sleep 5 && exit 1\n"),
            )
        })
        .await
        {
            Err(e) => panic!("timeout1: {:?}", e),
            Ok((x, y, z)) => (x, y, z),
        };

    assert_eq!(expectation1, result1?);
    assert_eq!(expectation2, result2?);
    assert_eq!(expectation3, result3?);
    Ok(())
}
