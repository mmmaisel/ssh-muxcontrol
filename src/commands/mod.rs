use bytes::{BufMut, BytesMut};
use std::{error::Error, fmt};

/// Low level SSH control socket protocol errors.
#[derive(Debug)]
pub struct CommandError {
    details: String,
}

impl CommandError {
    fn new(details: String) -> Self {
        Self { details }
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for CommandError {}

pub trait MuxCmd {
    fn serialize(&self, buffer: &mut BytesMut);
    fn length(&self) -> usize;
}

mod new_session;
pub use new_session::{MuxCmdNewSession, MuxRespNewSession};
mod hello;
pub use hello::{MuxCmdHello, MuxRespHello};
mod exit;
pub use exit::MuxRespExit;
mod check_alive;
pub use check_alive::{MuxCmdCheckAlive, MuxRespCheckAlive};

#[derive(Debug)]
pub struct MuxCmdMessage {
    pub request: u32,
    pub param: u32,
}

// https://github.com/openbsd/src/blob/master/usr.bin/ssh/mux.c
pub const MUX_VERSION: u32 = 4;
pub const MUX_MSG_HELLO: u32 = 1;
pub const MUX_NEW_SESSION: u32 = 0x10000002;
pub const MUX_ALIVE_CHECK: u32 = 0x10000004;

pub const MUX_IS_ALIVE: u32 = 0x80000005;
pub const MUX_SESSION_OPENED: u32 = 0x80000006;
pub const MUX_EXIT_MESSAGE: u32 = 0x80000004;

impl MuxCmd for MuxCmdMessage {
    fn serialize(&self, buffer: &mut BytesMut) {
        buffer.put_u32(self.request);
        buffer.put_u32(self.param);
    }

    fn length(&self) -> usize {
        8
    }
}
