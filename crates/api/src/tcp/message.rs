use model::{Object, Reader, Writer};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, Error, ErrorKind, Result},
    net::tcp::{OwnedReadHalf, OwnedWriteHalf},
};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Info = 0x01,   //[SEND] Info
    Error = 0x02,  //[SEND] Error
    Object = 0x03, //[RECV] Object
    StreamMode = 0x04,  //[SEND] On Stream mode un check mesage type
}

impl TryFrom<u8> for MessageType {
    type Error = Error;
    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(MessageType::Info),
            0x02 => Ok(MessageType::Error),
            0x03 => Ok(MessageType::Object),
            other => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Unknown message type: {other:#x}"),
            )),
        }
    }
}
// STRUCT MSG
// [1    ,4         ,N   ]
// [Type ,Data len  ,Data]
pub struct Message {
    pub message_type: MessageType,
    pub data: Vec<u8>,
}

impl Message {
    pub fn info(message: impl Into<String>) -> Self {
        Self {
            message_type: MessageType::Info,
            data: message.into().into_bytes(),
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
        let extension = {
            let s = r.read_string()?;
            if s.is_empty() { None } else { Some(s) }
        };
        let mime_type = {
            let s = r.read_string()?;
            if s.is_empty() { None } else { Some(s) }
        };

        Ok(Object::new(
            r.read_string()?,
            extension,
            mime_type,
            r.read_array::<32>()?,
            r.read_u64()?,
        ))
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
}
