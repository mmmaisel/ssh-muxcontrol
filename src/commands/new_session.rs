use super::{CommandError, MuxCmd, MUX_NEW_SESSION, MUX_SESSION_OPENED};
use bytes::{Buf, BufMut, BytesMut};
use std::convert::TryInto;

#[derive(Debug)]
pub struct MuxCmdNewSession {
    request_id: u32,
    reserved: String,
    tty_flags: u32,
    forward_x11: u32,
    forward_agent: u32,
    subsystem_flag: u32,
    escape_char: u32,
    term: String,
    command: String,
}

impl MuxCmdNewSession {
    pub fn new(request_id: u32, command: String) -> Self {
        Self {
            request_id,
            reserved: String::new(),
            tty_flags: 0,
            forward_x11: 0,
            forward_agent: 0,
            subsystem_flag: 0,
            escape_char: 0xffffffff, // disabled
            term: String::new(),
            command,
        }
    }
}

impl MuxCmd for MuxCmdNewSession {
    fn serialize(&self, buffer: &mut BytesMut) {
        buffer.put_u32(MUX_NEW_SESSION);
        buffer.put_u32(self.request_id);

        buffer.put_u32(self.reserved.len().try_into().unwrap());
        buffer.put_slice(self.reserved.as_bytes());

        buffer.put_u32(self.tty_flags);
        buffer.put_u32(self.forward_x11);
        buffer.put_u32(self.forward_agent);
        buffer.put_u32(self.subsystem_flag);
        buffer.put_u32(self.escape_char);

        buffer.put_u32(self.term.len().try_into().unwrap());
        buffer.put_slice(self.term.as_bytes());

        buffer.put_u32(self.command.len().try_into().unwrap());
        buffer.put_slice(self.command.as_bytes());
    }

    fn length(&self) -> usize {
        10 * 4 + self.reserved.len() + self.term.len() + self.command.len()
    }
}

#[derive(Debug)]
pub struct MuxRespNewSession {
    cmd: u32,
    request_id: u32,
    session_id: u32,
}

impl MuxRespNewSession {
    pub fn is_valid(&self, request_id: u32) -> bool {
        self.cmd == MUX_SESSION_OPENED && self.request_id == request_id
    }

    pub fn session_id(&self) -> u32 {
        self.session_id
    }

    pub fn deserialize<T: Buf>(
        buffer: &mut T,
    ) -> Result<MuxRespNewSession, CommandError> {
        if buffer.remaining() != 12 {
            return Err(CommandError::new(format!(
                "Buffer has length {} but MuxRespCheckAlive is 12 bytes long",
                buffer.remaining()
            )));
        }

        Ok(MuxRespNewSession {
            cmd: buffer.get_u32(),
            request_id: buffer.get_u32(),
            session_id: buffer.get_u32(),
        })
    }
}
