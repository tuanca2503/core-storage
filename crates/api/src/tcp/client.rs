use std::path::Path;
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, Error, ErrorKind, Result};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

use crate::tcp::{Message, MessageType};
use model::CHUNK_SIZE;
use platform::paths;

pub async fn new(file_path: String, ip: String, port: String) -> Result<()> {
    // Get file information
    let metadata = fs::metadata(&file_path).await?;
    if !metadata.is_file() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("'{file_path}' it is not valid file"),
        ));
    }
    let total_size = metadata.len();
    let path = Path::new(&file_path);
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Tên file không hợp lệ (UTF-8)"))?
        .to_string();
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_string());
    let mime_type = extension.as_deref().map(|ext| {
        mime_guess::from_ext(ext)
            .first_or_octet_stream()
            .to_string()
    });
    // END
    // Connect, send new object and handle
    let (mut reader, mut writer) = get_connection(ip, port).await?;
    Message::new(filename, extension, mime_type, total_size)
        .send(&mut writer)
        .await?;
    let msg = Message::from_reader(&mut reader).await?;
    match msg.message_type {
        MessageType::Stream => {
            let uuid = msg.as_string()?;
            create_tmp(&uuid, &file_path).await?;
            // Start sending
            let file = File::open(file_path).await?;
            let mut file_reader = BufReader::new(file);
            let mut buf = vec![0u8; CHUNK_SIZE as usize];
            loop {
                let n = file_reader.read(&mut buf).await?;
                if n == 0 {
                    break; // Len = 0 stop
                }
                writer.write_all(&buf[..n]).await?; // Send correctly n byte reading not all buf
            }
            // END
            remove_tmp(&uuid).await?; // Remove temp file
            println!("OK {}", uuid);
            Ok(())
        }
        MessageType::Error => {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Server error: {}", msg.as_string()?),
            ));
        }
        _ => {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Unvalid message '{}' from server", msg.message_type as u8),
            ));
        }
    }
    // END
}

pub async fn resume(file_path: String, uuid: String, ip: String, port: String) -> Result<()> {
    // Get file information
    let metadata = fs::metadata(&file_path).await?;
    if !metadata.is_file() {
        remove_tmp(&uuid).await?;
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("'{file_path}' it is not valid file"),
        ));
    }
    let total_size = metadata.len();
    let path = Path::new(&file_path);
    // END
    // Connect, send resume object and handle
    let (mut reader, mut writer) = get_connection(ip, port).await?;
    Message::resume(uuid).send(&mut writer).await?;
    let msg = Message::from_reader(&mut reader).await?;
    //TODO: send server when receive null > remove tmp file
    //      when has return check receive data same > continue
    match msg.message_type {
        
        MessageType::Error => {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Server error: {}", msg.as_string()?),
            ));
        }
        _ => {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Unvalid message '{}' from server", msg.message_type as u8),
            ));
        }
    }
}

pub async fn close(){
    //TODO: send uuid to remove
    //      in there call when user dont want continue sending
}

//
async fn get_connection(
    ip: String,
    port: String,
) -> Result<(BufReader<OwnedReadHalf>, OwnedWriteHalf)> {
    let addr = format!("{}:{}", ip, port);
    let stream = TcpStream::connect(&addr).await?; //127.0.0.1:7878
    stream.set_nodelay(true)?;
    let (reader, writer) = stream.into_split();
    Ok((BufReader::new(reader), writer))
}
async fn create_tmp(uuid: &str, file_path: &str) -> Result<()> {
    let path = paths::app_file(&format!("{uuid}.tmp"))?;
    fs::write(path, file_path).await
}
async fn remove_tmp(uuid: &str) -> Result<()> {
    let path = paths::app_file(&format!("{uuid}.tmp"))?;
    fs::remove_file(path).await
}
pub async fn get_tmp() -> Result<Option<(String, String)>> {
    let dir = paths::app_directory()?;
    let mut entries = fs::read_dir(&dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("tmp")
            && let Some(filename) = path.file_stem()
        {
            let filename = filename.to_string_lossy().into_owned();
            let content = std::fs::read_to_string(&path)?;
            return Ok(Some((filename, content)));
        }
    }
    Ok(None)
}
