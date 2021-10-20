use super::{CommandError, MUX_EXIT_MESSAGE};
use bytes::Buf;

#[derive(Debug)]
pub struct MuxRespExit {
    cmd: u32,
    session_id: u32,
    exit_code: u32,
}

impl MuxRespExit {
    pub fn is_valid(&self, session_id: u32) -> bool {
        self.cmd == MUX_EXIT_MESSAGE && self.session_id == session_id
    }

    pub fn exit_code(&self) -> u32 {
        self.exit_code
    }

    pub fn deserialize<T: Buf>(
        buffer: &mut T,
    ) -> Result<MuxRespExit, CommandError> {
        if buffer.remaining() < 8 {
            return Err(CommandError::new(
                "At least 8 bytes are required in buffer.".into(),
            ));
        }

        let cmd = buffer.get_u32();
        let session_id = buffer.get_u32();

        let exit_code = if cmd == MUX_EXIT_MESSAGE {
            if buffer.remaining() < 4 {
                return Err(CommandError::new(
                    "Received MUX_EXIT_MESSAGE but missing exit code in buffer.".
                    into()));
            }
            buffer.get_u32()
        } else {
            return Err(CommandError::new(format!(
                "Received invalid response: {}",
                cmd
            )));
        };

        if buffer.has_remaining() {
            Err(CommandError::new("Garbage at end of buffer".into()))
        } else {
            Ok(MuxRespExit {
                cmd,
                session_id,
                exit_code,
            })
        }
    }
}
