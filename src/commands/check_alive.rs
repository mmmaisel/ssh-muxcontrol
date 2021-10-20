use super::{CommandError, MuxCmd, MUX_ALIVE_CHECK, MUX_IS_ALIVE};
use bytes::{Buf, BufMut, BytesMut};

#[derive(Debug)]
pub struct MuxCmdCheckAlive {
    request_id: u32,
}

impl MuxCmdCheckAlive {
    pub fn new(request_id: u32) -> MuxCmdCheckAlive {
        Self { request_id }
    }
}

impl MuxCmd for MuxCmdCheckAlive {
    fn serialize(&self, buffer: &mut BytesMut) {
        buffer.put_u32(MUX_ALIVE_CHECK);
        buffer.put_u32(self.request_id);
    }

    fn length(&self) -> usize {
        8
    }
}

#[derive(Debug)]
pub struct MuxRespCheckAlive {
    cmd: u32,
    request_id: u32,
    ssh_pid: u32,
}

impl MuxRespCheckAlive {
    pub fn is_valid(&self, request_id: u32) -> bool {
        self.cmd == MUX_IS_ALIVE && self.request_id == request_id
    }

    pub fn deserialize<T: Buf>(
        buffer: &mut T,
    ) -> Result<MuxRespCheckAlive, CommandError> {
        if buffer.remaining() != 12 {
            return Err(CommandError::new(format!(
                "Buffer has length {} but MuxRespCheckAlive is 12 bytes long",
                buffer.remaining()
            )));
        }

        Ok(MuxRespCheckAlive {
            cmd: buffer.get_u32(),
            request_id: buffer.get_u32(),
            ssh_pid: buffer.get_u32(),
        })
    }
}
