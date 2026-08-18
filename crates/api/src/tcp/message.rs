use std::time::Duration;

use model::{Object, Reader, Writer};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, Error, ErrorKind, Result}, net::tcp::{OwnedReadHalf, OwnedWriteHalf}, time::timeout,
};
const READ_TIMEOUT: Duration = Duration::from_secs(60);

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    //0-20
    Close = 0x00,  //[RECV] Close object
    New = 0x01,    //[RECV] New object
    Resume = 0x02, //[RECV] Resume object
    //20 - n
    Success = 0x14, //[SEND] Success
    Info = 0x15,    //[SEND] Info
    Error = 0x16,   //[SEND] Error
    Stream = 0x17,  //[SEND] On Stream mode un check mesage type
}

impl TryFrom<u8> for MessageType {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x00 => Ok(MessageType::Close),
            0x01 => Ok(MessageType::New),
            0x02 => Ok(MessageType::Resume),
            other => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Unknown message type: {other:#x}"),
            )),
        }
    }
}
//       STRUCT MSG
// [1    ,4         ,N   ]
// [Type ,Data len  ,Data]
pub struct Message {
    pub message_type: MessageType,
    pub data: Vec<u8>,
}

impl Message {
    pub fn stream(message: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Stream,
            data: message.into().into_bytes(),
        }
    }
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Info,
            data: message.into().into_bytes(),
        }
    }
    pub fn success() -> Self {
        Self {
            message_type: MessageType::Success,
            data: vec![],
        }
    }
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Error,
            data: message.into().into_bytes(),
        }
    }
    pub fn as_object(&self) -> Result<Object> {
        let mut r = Reader::new(&self.data);
        let original_filename = r.read_string()?;
        let extension = {
            let s = r.read_string()?;
            if s.is_empty() { None } else { Some(s) }
        };
        let mime_type = {
            let s = r.read_string()?;
            if s.is_empty() { None } else { Some(s) }
        };
        Ok(Object::new(
            original_filename,
            extension,
            mime_type,
            r.read_array::<32>()?,
            r.read_u64()?,
        ))
    }

    pub fn get_string(&self) -> Result<String> {
        String::from_utf8(self.data.clone())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub async fn from_reader(reader: &mut BufReader<OwnedReadHalf>) -> Result<Self> {
        let mut header = [0u8; 5]; // header 1 | len 4(~4GB)
        reader.read_exact(&mut header).await?;

        let message_type = MessageType::try_from(header[0])?;
        let len = u32::from_be_bytes([header[1], header[2], header[3], header[4]]) as usize;

        let mut data = vec![0u8; len];
        reader.read_exact(&mut data).await?;

        Ok(Message { message_type, data })
    }
    pub async fn send(&self, writer: &mut OwnedWriteHalf) -> Result<()> {
        let data_len = self.data.len();
        let mut w = Writer::with_capacity(5 + data_len);
        w.write_u8(self.message_type as u8);
        w.write_u32(data_len as u32);
        w.write_slice(&self.data);
        writer.write_all(&w.into_bytes()).await?;
        Ok(())
    }

    //

    pub async fn read_data_chunk(
        reader: &mut BufReader<OwnedReadHalf>,
        buf: &mut [u8],
    ) -> Result<()> {
        timeout(READ_TIMEOUT, reader.read_exact(buf)) // read_exact tự loop bên trong rồi
            .await
            .map_err(|_| {
                Error::new(
                    ErrorKind::TimedOut,
                    "không nhận được dữ liệu trong thời gian chờ",
                )
            })??;
        Ok(())
    }
}
