// disk_format_test — công cụ TEST đơn giản:
//   1) Liệt kê các ổ đĩa vật lý (\\.\PhysicalDrive0, \\.\PhysicalDrive1, ...)
//   2) Cho chọn disk_id, xác nhận 2 lần
//   3) Ghi thử 1 "superblock" tối giản (magic + version) vào sector đầu tiên
//   4) Đọc lại để verify
//
// ⚠️ CHỈ DÙNG CHO WINDOWS + Ổ CỨNG TEST (ổ rời). Phải chạy với quyền Administrator.
// ⚠️ Ghi vào offset 0 sẽ PHÁ HUỶ partition table/filesystem hiện có trên ổ đó.
//    Đây là hành vi CHỦ ĐÍCH của "format thử" — không dùng nhầm lên ổ đang chứa dữ liệu quan trọng.

use std::fs::OpenOptions;
use std::io::{self, Read, Seek, SeekFrom, Write};

const SECTOR_SIZE: usize = 512;
const MAGIC: &[u8; 4] = b"RPFS";
const VERSION: u32 = 1;

/// Thông tin 1 ổ đĩa vật lý phát hiện được.
#[derive(Debug)]
struct PhysicalDiskInfo {
    disk_id: u32,
    path: String,
    size_bytes: u64,
}

/// Quét \\.\PhysicalDrive0 .. PhysicalDrive{max_index - 1}.
/// Ổ nào mở được (tồn tại + đủ quyền) sẽ xuất hiện trong danh sách.
///
/// Lưu ý: đây là cách enumerate ĐƠN GIẢN cho mục đích test.
/// Bản chính thức sau này nên thay bằng SetupAPI/WMI (xem phần DiskEnumerator trong plan)
/// để lấy thêm model, serial, và không phụ thuộc vào việc "đoán" số thứ tự ổ.
fn list_physical_disks(max_index: u32) -> Vec<PhysicalDiskInfo> {
    let mut disks = Vec::new();

    for i in 0..max_index {
        let path = format!(r"\\.\PhysicalDrive{i}");

        let mut file = match OpenOptions::new().read(true).open(&path) {
            Ok(f) => f,
            Err(_) => continue, // không tồn tại hoặc không đủ quyền -> bỏ qua
        };

        // Trick: seek tới cuối handle của physical drive sẽ trả về đúng
        // dung lượng thật của ổ đĩa trên Windows (không phải EOF theo nghĩa file thường).
        if let Ok(size) = file.seek(SeekFrom::End(0)) {
            disks.push(PhysicalDiskInfo {
                disk_id: i,
                path,
                size_bytes: size,
            });
        }
    }

    disks
}

fn print_disks(disks: &[PhysicalDiskInfo]) {
    println!("== Danh sách ổ đĩa vật lý phát hiện được ==");
    if disks.is_empty() {
        println!("  (Không phát hiện ổ nào — kiểm tra lại quyền Administrator.)");
    }
    for d in disks {
        let gb = d.size_bytes as f64 / 1_073_741_824.0;
        println!("  Disk {:>2}  |  {:>9.2} GB  |  {}", d.disk_id, gb, d.path);
    }
    println!("=============================================");
    println!("⚠️  Kiểm tra kỹ dung lượng để chắc chắn đây là ổ TEST, không phải ổ chứa Windows (C:).");
}

/// Tạo 1 sector (512 byte) superblock tối giản: magic + version + phần còn lại để 0 (reserved).
fn build_superblock() -> [u8; SECTOR_SIZE] {
    let mut buf = [0u8; SECTOR_SIZE];
    buf[0..4].copy_from_slice(MAGIC);
    buf[4..8].copy_from_slice(&VERSION.to_le_bytes());
    // byte 8 trở đi: để trống, dành cho mở rộng schema sau này (offset các bảng, v.v.)
    buf
}

/// Ghi superblock vào offset 0 của ổ đĩa theo disk_id.
/// Đây là bản "format thử" tối giản — chỉ ghi ĐÚNG 1 sector đầu tiên (512 byte),
/// không đụng tới phần còn lại của ổ.
fn write_test_superblock(disk_id: u32) -> io::Result<()> {
    let path = format!(r"\\.\PhysicalDrive{disk_id}");

    let mut file = OpenOptions::new().read(true).write(true).open(&path)?;

    let superblock = build_superblock();

    file.seek(SeekFrom::Start(0))?;
    file.write_all(&superblock)?;
    file.flush()?;

    println!("✅ Đã ghi superblock thử nghiệm vào {path} (offset 0, {SECTOR_SIZE} byte).");
    Ok(())
}

/// Đọc lại sector đầu tiên để xác nhận đã ghi đúng magic + version.
fn verify_superblock(disk_id: u32) -> io::Result<bool> {
    let path = format!(r"\\.\PhysicalDrive{disk_id}");
    let mut file = OpenOptions::new().read(true).open(&path)?;

    let mut buf = [0u8; SECTOR_SIZE];
    file.seek(SeekFrom::Start(0))?;
    file.read_exact(&mut buf)?;

    let magic_ok = &buf[0..4] == MAGIC;
    let version_ok = u32::from_le_bytes(buf[4..8].try_into().unwrap()) == VERSION;

    Ok(magic_ok && version_ok)
}

fn read_u32_from_stdin(prompt: &str) -> io::Result<u32> {
    println!("{prompt}");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    input
        .trim()
        .parse::<u32>()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Giá trị nhập không hợp lệ"))
}

fn main() -> io::Result<()> {
    // Quét Disk 0..15 — tăng số này nếu máy test có nhiều hơn 16 ổ.
    let disks = list_physical_disks(16);
    print_disks(&disks);

    let disk_id = read_u32_from_stdin("\nNhập disk_id muốn format thử (ví dụ: 1):")?;
    let confirm_id = read_u32_from_stdin(&format!(
        "Bạn vừa chọn Disk {disk_id}. Gõ LẠI đúng số này để XÁC NHẬN ghi đè:"
    ))?;

    if confirm_id != disk_id {
        println!("❌ Giá trị xác nhận không khớp. Huỷ thao tác — KHÔNG ghi gì cả.");
        return Ok(());
    }

    write_test_superblock(disk_id)?;

    match verify_superblock(disk_id) {
        Ok(true) => println!("✅ Đọc lại kiểm tra: superblock hợp lệ (magic + version khớp)."),
        Ok(false) => println!("⚠️ Đọc lại nhưng dữ liệu không khớp — kiểm tra lại logic ghi/đọc."),
        Err(e) => println!("Lỗi khi đọc lại: {e}"),
    }

    Ok(())
}