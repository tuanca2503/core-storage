"""
Client Python don gian de test ket noi TCP toi server (port 7878).
Chi lam dung phan handshake: server gui "HELLO\n" truoc, client gui lai
"HELLO\n" de hoan tat bat tay -- khop voi Server::handshake() ben Rust.
"""

import hashlib
import socket
import struct
import time

HOST = "127.0.0.1"
PORT = 7878
TIMEOUT_SECONDS = 65


def main() -> None:
    with socket.create_connection((HOST, PORT), timeout=TIMEOUT_SECONDS) as sock:
        print(f"Da ket noi toi {HOST}:{PORT}")
        # Gui lai HELLO de hoan tat handshake
        
        send_info(sock)
        greeting = sock.recv(1024)
        print(f"Nhan tu server: {greeting!r}")
        # time.sleep(5)
        # sock.sendall(b"HELLO\n")
        # greeting = sock.recv(1024)
        # print(f"Nhan tu server: {greeting!r}")


def send_info(sock: socket.socket):
    filename = "test.pdf"
    extension = "pdf"
    mime_type = "application/pdf"

    # Test data
    file_data = b"Hello from Python TCP client!"

    filename_data = filename.encode("utf-8") + b"\x00"
    extension_data = extension.encode("utf-8") + b"\x00"
    mime_data = mime_type.encode("utf-8") + b"\x00"

    # SHA-256
    checksum = hashlib.sha256(file_data).digest()
    total_size = len(file_data)

    # DATA
    data = (
        filename_data
        + extension_data
        + mime_data
        + checksum
        + struct.pack("<Q", total_size)
    )

    # HEADER
    # 1 byte  : message type
    # 4 bytes : data length (big endian)
    header = struct.pack(
        ">BI",
        0x03,
        len(data),
    )
    print("send")

    sock.sendall(header + data)
    print("send success")
    

if __name__ == "__main__":
    try:
        main()
    except ConnectionRefusedError:
        print(f"Khong ket noi duoc toi {HOST}:{PORT} -- server co dang chay khong?")
    except socket.timeout:
        print("Timeout khi cho phan hoi tu server.")