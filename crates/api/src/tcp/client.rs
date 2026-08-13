use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::watch;

#[tokio::main]
async fn main() -> io::Result<()> {
    let addr = std::env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:7878".to_string());

    let stream = TcpStream::connect(&addr).await?;
    stream.set_nodelay(true)?;
    println!("Đã kết nối tới {addr}");
    println!("Gõ nội dung rồi Enter để gửi. Gõ 'quit' để thoát.\n");

    let (reader, mut writer) = stream.into_split();

    // Báo hiệu khi server đóng kết nối, để vòng lặp chính tự dừng
    // thay vì gọi thẳng process::exit (không cho phần còn lại kịp dọn dẹp).
    let (shutdown_tx, mut shutdown_rx) = watch::channel(false);

    tokio::spawn(async move {
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    println!("\nServer đã đóng kết nối.");
                    let _ = shutdown_tx.send(true);
                    break;
                }
                Ok(_) => print!("{line}"),
                Err(e) => {
                    eprintln!("Lỗi đọc từ server: {e}");
                    let _ = shutdown_tx.send(true);
                    break;
                }
            }
        }
    });

    let stdin = io::stdin();
    let mut stdin_reader = BufReader::new(stdin);
    let mut input = String::new();

    loop {
        input.clear();

        tokio::select! {
            result = stdin_reader.read_line(&mut input) => {
                let bytes_read = result?;
                if bytes_read == 0 {
                    break; // Ctrl+D / EOF
                }

                writer.write_all(input.as_bytes()).await?;

                if input.trim() == "quit" {
                    break;
                }
            }

            _ = shutdown_rx.changed() => {
                break; // server đã đóng kết nối
            }
        }
    }

    Ok(())
}