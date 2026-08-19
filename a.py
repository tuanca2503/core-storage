"""
Client Python don gian de test ket noi TCP toi server (port 7878).
Luong: gui MSG_INFO (0x01) -> nhan phan hoi tu server (ky vong type=0x17,
nghia la server da san sang nhan stream) -> gui du lieu file (raw, dung
bang total_size) -> gui MSG_END (0x03) kem hash -> nhan ACK (0x04) cuoi cung.
"""

import hashlib
import socket
import struct

HOST = "127.0.0.1"
PORT = 7878
TIMEOUT_SECONDS = 65

MSG_SC = 0x14

MSG_INFO = 0x01
MSG_END = 0x03
MSG_ACK = 0x04
MSG_STREAM_READY = 0x17  # server bao "da san sang nhan stream"


def recv_exact(sock: socket.socket, n: int) -> bytes:
    """Nhan dung n byte, tu loop vi 1 lan recv() khong dam bao du."""
    buf = b""
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        if not chunk:
            raise ConnectionError(
                "Server da dong ket noi truoc khi nhan du du lieu (early eof)"
            )
        buf += chunk
    return buf


def recv_message(sock: socket.socket) -> tuple[int, bytes]:
    """Doc 1 message theo format: 1 byte type + 4 byte length (BE) + payload."""
    header = recv_exact(sock, 5)
    msg_type = header[0]
    length = struct.unpack("<I", header[1:5])[0]
    payload = recv_exact(sock, length) if length > 0 else b""
    return msg_type, payload


def send_info(sock: socket.socket, file_data: bytes) -> bytes:
    """Gui MSG_INFO, tra ve checksum de dung lai khi gui MSG_END."""
    filename = "test.pdf"
    extension = "pdf"
    mime_type = "application/pdf"

    filename_data = filename.encode("utf-8") + b"\x00"
    extension_data = extension.encode("utf-8") + b"\x00"
    mime_data = mime_type.encode("utf-8") + b"\x00"

    checksum = hashlib.sha256(file_data).digest()
    total_size = len(file_data)

    data = (
        filename_data
        + extension_data
        + mime_data
        + checksum
        + struct.pack("<Q", total_size)
    )

    header = struct.pack(">BI", MSG_INFO, len(data))
    sock.sendall(header + data)
    print(f"Da gui MSG_INFO: filename={filename!r}, total_size={total_size}")

    return checksum


def send_fake_data(sock: socket.socket, file_data: bytes) -> None:
    """Gui raw data (KHONG co header) dung bang total_size da khai o MSG_INFO."""
    sock.sendall(file_data)
    print(f"Da gui {len(file_data)} byte du lieu (raw, khong header)")


def main() -> None:
    with socket.create_connection((HOST, PORT), timeout=TIMEOUT_SECONDS) as sock:
        print(f"Da ket noi toi {HOST}:{PORT}")

        file_data = b"""
        Hello from Python TCP client!Hello from Python TCP client!Hello from Python TCP client!
        Hello from Python TCP client!Hello from Python TCP client!Hello from Python TCP client!
        """
        checksum = send_info(sock, file_data)

        msg_type, payload = recv_message(sock)
        print(f"Nhan tu server: type=0x{msg_type:02x} payload={payload!r}")

        if msg_type != MSG_STREAM_READY:
            print(f"Server khong san sang nhan stream (type=0x{msg_type:02x})")
            return

        print("OK: server da san sang nhan stream (0x17)")

        send_fake_data(sock, file_data)

        msg_type, payload = recv_message(sock)
        
        if msg_type == MSG_SC:
            ok = bool(payload) and payload[0] == 1
            print(f"ACK cuoi cung tu server: {'THANH CONG' if ok else 'THAT BAI'}")
        else:
            print(f"Phan hoi khong mong doi: type=0x{msg_type:02x} payload={payload!r}")


if __name__ == "__main__":
    try:
        main()
    except ConnectionRefusedError:
        print(f"Khong ket noi duoc toi {HOST}:{PORT} -- server co dang chay khong?")
    except socket.timeout:
        print("Timeout khi cho phan hoi tu server.")
    except ConnectionError as e:
        print(f"Loi ket noi: {e}")