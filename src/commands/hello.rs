use super::{CommandError, MuxCmd, MUX_MSG_HELLO, MUX_VERSION};
use bytes::{Buf, BufMut, BytesMut};

#[derive(Debug)]
pub struct MuxCmdHello {}

impl MuxCmd for MuxCmdHello {
    fn serialize(&self, buffer: &mut BytesMut) {
        buffer.put_u32(MUX_MSG_HELLO);
        buffer.put_u32(MUX_VERSION);
    }

    fn length(&self) -> usize {
        8
    }
}

#[derive(Debug)]
pub struct MuxRespHello {
    cmd: u32,
    version: u32,
}

impl MuxRespHello {
    pub fn is_valid(&self) -> bool {
        self.cmd == MUX_MSG_HELLO && self.version == MUX_VERSION
    }

    pub fn deserialize<T: Buf>(
        buffer: &mut T,
    ) -> Result<MuxRespHello, CommandError> {
        if buffer.remaining() != 8 {
            return Err(CommandError::new(format!(
                "Buffer has length {} but MuxRespHello is 8 bytes long",
                buffer.remaining()
            )));
        }

        Ok(MuxRespHello {
            cmd: buffer.get_u32(),
            version: buffer.get_u32(),
        })
    }
}
