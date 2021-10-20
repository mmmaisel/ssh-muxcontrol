use std::convert::TryInto;
use std::error::Error;
use std::fmt;
use std::os::unix::io::AsRawFd;

use bytes::{BufMut, BytesMut};
use sendfd::SendWithFd;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
use tokio_pipe::{PipeRead, PipeWrite};

use crate::commands::{
    MuxCmd, MuxCmdCheckAlive, MuxCmdHello, MuxCmdNewSession, MuxRespCheckAlive,
    MuxRespExit, MuxRespHello, MuxRespNewSession,
};
use crate::SshctlError;

/// A simple struct which contains the stdout, stderr and exit code
/// of a completed remote command.
#[derive(PartialEq, Debug, Clone, Eq)]
pub struct ShellResult {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: u32,
}

/// SSH control socket errors.
#[derive(Debug)]
pub struct MuxError {
    details: String,
}

impl MuxError {
    fn new(details: String) -> Self {
        Self { details }
    }
}

impl fmt::Display for MuxError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for MuxError {}

/// Runs a given shell command on the remote hosts default shell
/// though an existing SSH UNIX control socket.
/// The SSH control socket is created
/// outside of this crate by an existing SSH connection.
pub async fn run(
    path: &str,
    command: &str,
) -> Result<ShellResult, SshctlError> {
    run_stdin(path, command, None).await
}

/// Runs a given shell command on the remote hosts default shell
/// though an existing SSH UNIX control socket.
/// The SSH control socket is created
/// outside of this crate by an existing SSH connection.
///
/// This function is the same as `run` but custom data is supplied to
/// the remote commands STDIN.
pub async fn run_stdin(
    ctlpath: &str,
    command: &str,
    stdin: Option<Vec<u8>>,
) -> Result<ShellResult, SshctlError> {
    //#[cfg(debug_assertions)]
    //eprintln!("Mux::request_session: {}", ctlpath);
    let mut socket = UnixStream::connect(ctlpath).await?;

    hello(&mut socket).await?;
    let request_id = check_mux_alive(&mut socket, 0).await?;
    let (session_id, local_stdin, local_stdout, local_stderr) =
        new_session(&mut socket, request_id, command).await?;

    let stdin_data = stdin.unwrap_or_default();

    let (rx_rc, tx_stdin, rx_stdout, rx_stderr) = tokio::join! {
        wait(&mut socket, session_id),
        write_stdin(local_stdin, &stdin_data[..]),
        read_ssh_pipe(local_stdout),
        read_ssh_pipe(local_stderr),
    };

    tx_stdin?;

    Ok(ShellResult {
        stdout: rx_stdout?,
        stderr: rx_stderr?,
        exit_code: rx_rc?,
    })
}

async fn read_packet_response(
    socket: &mut UnixStream,
) -> Result<Vec<u8>, std::io::Error> {
    let mut buffer = [0; 4];

    socket.read_exact(&mut buffer).await?;
    let length = u32::from_be_bytes(buffer) as usize;
    //eprintln!("Received length: {}", &length);
    let mut response = vec![0; length];
    socket.read_exact(&mut response[0..length]).await?;
    Ok(response)
}

async fn write_command<T: MuxCmd>(
    socket: &mut UnixStream,
    command: &T,
) -> Result<(), std::io::Error> {
    let mut buffer = BytesMut::with_capacity(command.length() + 4);
    buffer.put_u32(command.length().try_into().unwrap());
    command.serialize(&mut buffer);

    //#[cfg(debug_assertions)]
    //eprintln!("writing mux command: {:?}", &buffer);

    socket.write(&buffer).await.map(|_| ())
}

async fn hello(socket: &mut UnixStream) -> Result<(), SshctlError> {
    //#[cfg(debug_assertions)]
    //eprintln!("Mux::hello");

    let response = match read_packet_response(socket).await {
        Ok(x) => MuxRespHello::deserialize(&mut x.as_slice())?,
        Err(e) => {
            return Err(MuxError::new(format!(
                "Read MuxRespHello failed: {:?}",
                e
            ))
            .into())
        }
    };

    if !response.is_valid() {
        return Err(MuxError::new(format!(
            "Received invalid hello message: {:?}",
            response
        ))
        .into());
    }

    let command = MuxCmdHello {};

    if let Err(e) = write_command(socket, &command).await {
        return Err(MuxError::new(format!(
            "Write MuxCmdHello failed: {:?}",
            e
        ))
        .into());
    }
    Ok(())
}

async fn check_mux_alive(
    socket: &mut UnixStream,
    request_id: u32,
) -> Result<u32, SshctlError> {
    //#[cfg(debug_assertions)]
    //eprintln!("Mux::check_alive");

    let command = MuxCmdCheckAlive::new(request_id);

    if let Err(e) = write_command(socket, &command).await {
        return Err(MuxError::new(format!(
            "Write check alive request failed: {:?}",
            e
        ))
        .into());
    }

    let response = match read_packet_response(socket).await {
        Ok(x) => MuxRespCheckAlive::deserialize(&mut x.as_slice())?,
        Err(e) => {
            return Err(MuxError::new(format!(
                "Read MuxRespCheckAlive failed: {:?}",
                e
            ))
            .into())
        }
    };

    if !response.is_valid(request_id) {
        return Err(MuxError::new(format!(
            "Received invalid check_alive message: {:?}",
            response
        ))
        .into());
    }

    Ok(request_id + 1)
}

async fn new_session(
    socket: &mut UnixStream,
    request_id: u32,
    command: &str,
) -> Result<(u32, PipeWrite, PipeRead, PipeRead), SshctlError> {
    //#[cfg(debug_assertions)]
    //eprintln!("Mux::new_session");

    let command = MuxCmdNewSession::new(request_id, command.into());

    if let Err(e) = write_command(socket, &command).await {
        return Err(MuxError::new(format!(
            "Write new session request failed: {:?}",
            e
        ))
        .into());
    }

    let (remote_stdin, local_stdin) = tokio_pipe::pipe()?;
    let (local_stdout, remote_stdout) = tokio_pipe::pipe()?;
    let (local_stderr, remote_stderr) = tokio_pipe::pipe()?;

    let fds: [i32; 1] = [remote_stdin.as_raw_fd()];
    if let Err(e) = socket.send_with_fd(b" ", &fds) {
        return Err(
            MuxError::new(format!("send_with_fd failed: {:?}", e)).into()
        );
    }

    let fds: [i32; 1] = [remote_stdout.as_raw_fd()];
    if let Err(e) = socket.send_with_fd(b" ", &fds) {
        return Err(
            MuxError::new(format!("send_with_fd failed: {:?}", e)).into()
        );
    }

    let fds: [i32; 1] = [remote_stderr.as_raw_fd()];
    if let Err(e) = socket.send_with_fd(b" ", &fds) {
        return Err(
            MuxError::new(format!("send_with_fd failed: {:?}", e)).into()
        );
    }

    let response = match read_packet_response(socket).await {
        Ok(x) => MuxRespNewSession::deserialize(&mut x.as_slice())?,
        Err(e) => {
            return Err(MuxError::new(format!(
                "Read MuxRespNewSession failed: {:?}",
                e
            ))
            .into())
        }
    };

    if !response.is_valid(request_id) {
        return Err(MuxError::new(format!(
            "Received invalid new_session message: {:?}",
            response
        ))
        .into());
    }

    Ok((
        response.session_id(),
        local_stdin,
        local_stdout,
        local_stderr,
    ))
}

async fn wait(
    socket: &mut UnixStream,
    session_id: u32,
) -> Result<u32, SshctlError> {
    let response = match read_packet_response(socket).await {
        Ok(x) => MuxRespExit::deserialize(&mut x.as_slice())?,
        Err(e) => {
            return Err(MuxError::new(format!(
                "Read MuxRespExit failed: {:?}",
                e
            ))
            .into())
        }
    };

    if !response.is_valid(session_id) {
        return Err(MuxError::new(format!(
            "Received invalid exit message: {:?}",
            response
        ))
        .into());
    }

    Ok(response.exit_code())
}

async fn write_stdin(
    mut local_stdin: PipeWrite,
    buffer: &[u8],
) -> Result<(), MuxError> {
    if let Err(e) = local_stdin.write_all(buffer).await {
        return Err(MuxError::new(format!("Write stdin failed: {:?}", e)));
    }
    Ok(())
}

async fn read_ssh_pipe(mut pipe: PipeRead) -> Result<Vec<u8>, MuxError> {
    let mut data = Vec::<u8>::with_capacity(1024);
    let mut buffer = [0; 1024];
    loop {
        match pipe.read(&mut buffer).await {
            Ok(count) => {
                //#[cfg(debug_assertions)]
                //eprintln!("received from pipe: {}: {:?}", count, data);
                if count == 0 {
                    return Ok(data);
                }
                data.append(&mut buffer[..count].to_vec());
            }
            Err(e) => {
                return Err(MuxError::new(format!(
                    "Read stdout failed: {:?}",
                    e
                )))
            }
        };
    }
}
