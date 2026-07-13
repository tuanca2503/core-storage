# plan-platform.md — core-storage-platform: Kiến trúc & Cách build theo OS

> Crate này là **Platform Abstraction Layer (PAL)**: chỉ giao tiếp với hệ điều hành
> (enumerate disk, mở raw device, đọc/ghi sector, flush, lock...). Không chứa logic
> Storage Engine (Object/Chunk/Segment/Allocation/SuperBlock) — phần đó thuộc crate
> `core-storage` gọi vào đây qua trait.

---

## 1. Kiến trúc tổng thể

```
Application
      │
      ▼
core-storage            (Storage Engine: Object/Chunk/Segment/Allocation...)
      │  chỉ gọi qua trait, KHÔNG biết OS bên dưới là gì
      ▼
core-storage-platform   (OS Abstraction — crate này)
      │
 ┌────┼─────────┐
 ▼    ▼         ▼
windows linux  macos
```

Nguyên tắc: **Storage Engine không bao giờ gọi thẳng API của Windows/Linux/macOS.**
Mọi thứ đi qua trait định nghĩa trong `traits/`.

---

## 2. Cấu trúc thư mục

```
core-storage-platform/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── error.rs
    ├── types.rs
    │
    ├── traits/
    │   ├── mod.rs
    │   ├── disk.rs
    │   ├── volume.rs
    │   └── raw.rs
    │
    ├── common/                 (dùng chung mọi OS — KHÔNG chứa code phụ thuộc OS)
    │   ├── mod.rs
    │   ├── sector.rs
    │   ├── alignment.rs
    │   ├── buffer.rs
    │   ├── endian.rs
    │   └── checksum.rs
    │
    ├── windows/
    │   ├── mod.rs
    │   ├── disk/        { mod.rs, enumerate.rs, information.rs, system.rs }
    │   ├── volume/       { mod.rs, enumerate.rs, mount.rs }
    │   ├── raw/          { mod.rs, open.rs, read.rs, write.rs, flush.rs, lock.rs }
    │   ├── format/       { mod.rs, erase.rs, initialize.rs }
    │   └── util/         { mod.rs, handle.rs, path.rs }
    │
    ├── linux/            (cấu trúc tương tự windows/)
    │   ├── mod.rs
    │   ├── disk/ volume/ raw/ format/ util/
    │
    └── macos/            (cấu trúc tương tự windows/)
        ├── mod.rs
        ├── disk/ volume/ raw/ format/ util/
```

Cấu trúc `disk / volume / raw / format / util` lặp lại giống nhau ở cả 3 OS — đây là
điểm hay của thiết kế gốc, mình giữ nguyên. Điểm mình chỉnh lại nằm ở **mục 5** (cách
chọn platform lúc build) vì phần đó trong đề bài chưa khả thi như hình dung ban đầu.

---

## 3. traits/ — hợp đồng chung cho mọi OS

```rust
// traits/disk.rs
use crate::error::PlatformError;
use crate::types::PhysicalDiskInfo;

pub trait DiskPlatform {
    fn enumerate() -> Result<Vec<PhysicalDiskInfo>, PlatformError>;
    fn is_system_disk(disk_id: u32) -> Result<bool, PlatformError>;
}
```

```rust
// traits/raw.rs
use crate::error::PlatformError;

pub trait RawDiskPlatform: Sized {
    fn open(disk_id: u32) -> Result<Self, PlatformError>;
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), PlatformError>;
    fn write_at(&mut self, offset: u64, data: &[u8]) -> Result<(), PlatformError>;
    fn flush(&mut self) -> Result<(), PlatformError>;
    fn lock(&mut self) -> Result<(), PlatformError>;   // khoá volume trước khi ghi raw
    fn unlock(&mut self) -> Result<(), PlatformError>;
}
```

```rust
// traits/volume.rs
use crate::error::PlatformError;
use crate::types::VolumeInfo;

pub trait VolumePlatform {
    fn list_volumes() -> Result<Vec<VolumeInfo>, PlatformError>;
    fn unmount(mount_point: &str) -> Result<(), PlatformError>;
}
```

`core-storage` chỉ import các trait này — không `use` bất kỳ thứ gì trong `windows/`,
`linux/`, `macos/` trực tiếp.

---

## 4. error.rs / types.rs (dùng chung, không phụ thuộc OS)

```rust
// error.rs
#[derive(Debug)]
pub enum PlatformError {
    DiskNotFound(u32),
    PermissionDenied,
    DeviceBusy,
    InvalidSector,
    UnsupportedPlatform,
    Io(std::io::Error),
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DiskNotFound(id) => write!(f, "Không tìm thấy disk_id {id}"),
            Self::PermissionDenied => write!(f, "Không đủ quyền (cần Administrator/root)"),
            Self::DeviceBusy => write!(f, "Thiết bị đang được sử dụng"),
            Self::InvalidSector => write!(f, "Sector không hợp lệ"),
            Self::UnsupportedPlatform => write!(f, "Hệ điều hành chưa được hỗ trợ"),
            Self::Io(e) => write!(f, "Lỗi I/O: {e}"),
        }
    }
}
impl std::error::Error for PlatformError {}
impl From<std::io::Error> for PlatformError {
    fn from(e: std::io::Error) -> Self { Self::Io(e) }
}
```

```rust
// types.rs
#[derive(Debug, Clone)]
pub struct PhysicalDiskInfo {
    pub disk_id: u32,
    pub path: String,
    pub size_bytes: u64,
    pub sector_size: u32,
    pub is_removable: bool,
}

#[derive(Debug, Clone)]
pub struct VolumeInfo {
    pub mount_point: String,
    pub disk_id: u32,
    pub filesystem: Option<String>,
}
```

---

## 5. ⚠️ Phần cần sửa lại: "cargo build --window" **không tồn tại**

Đây là chỗ không khả thi trong hình dung ban đầu, mình giải thích lại cho đúng cách
Cargo/Rust hoạt động.

### Sự thật: bạn **không cần** truyền flag gì cả trong trường hợp thông thường

Rust có sẵn cơ chế `#[cfg(target_os = "...")]`. Khi bạn chạy `cargo build` **trên
chính máy Windows**, Cargo tự biết target là `x86_64-pc-windows-msvc` → mọi
`#[cfg(target_os = "windows")]` tự động **bật**, còn `linux`/`macos` tự động **tắt**
(code trong đó thậm chí không được compile, không tốn thời gian build).

Tương tự khi build trên máy Linux → tự động chọn nhánh `linux`. Trên macOS → tự động
chọn `macos`. **Không có `--window`, không có `--linux`** — việc này Cargo lo hết,
dựa vào chính hệ điều hành bạn đang build trên đó (hoặc `--target` nếu cross-compile,
xem mục 5.2).

### 5.1. Cách hiện thực trong `lib.rs`

```rust
// lib.rs
pub mod error;
pub mod types;
pub mod traits;
pub mod common;

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use windows as platform;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use linux as platform;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub use macos as platform;

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
compile_error!("core-storage-platform: hệ điều hành này chưa được hỗ trợ");
```

`core-storage` (crate gọi vào) chỉ cần viết:

```rust
use core_storage_platform::platform;

let disks = platform::disk::enumerate()?;
let mut disk = platform::raw::open(disk_id)?;
disk.write_at(0, &superblock)?;
disk.flush()?;
```

**Đúng như bạn muốn** ("build trên Windows thì code phải lấy code Windows") — chỉ
khác là cơ chế chọn diễn ra **tự động theo `target_os`**, không cần cờ `--window` do
bạn tự đặt ra.

### 5.2. Nếu thật sự muốn build chéo (cross-compile) — ví dụ đang ở Linux nhưng muốn build ra file .exe cho Windows

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --target x86_64-pc-windows-gnu
```

Lúc này bạn truyền **target triple** (`--target ...`), không phải flag tự chế
`--window`. Cargo dựa vào target này để suy ra `target_os = "windows"` và build đúng
nhánh. Lưu ý: build chéo chỉ tạo ra **file thực thi cho Windows**, vẫn phải chạy trên
máy Windows thật để test raw disk I/O (không test được logic mở `\\.\PhysicalDriveN`
ngay trên Linux).

### 5.3. Cargo.toml — dependency riêng theo OS cũng tự động, không cần chọn tay

```toml
[package]
name = "core-storage-platform"
version = "0.1.0"
edition = "2021"

[dependencies]

[target.'cfg(windows)'.dependencies]
windows-sys = { version = "0.52", features = [
    "Win32_Storage_FileSystem",
    "Win32_Foundation",
    "Win32_System_IO",
    "Win32_System_Ioctl",
] }

[target.'cfg(target_os = "linux")'.dependencies]
libc = "0.2"

[target.'cfg(target_os = "macos")'.dependencies]
libc = "0.2"
core-foundation = "0.9"
```

`[target.'cfg(windows)'.dependencies]` nghĩa là: chỉ tải `windows-sys` khi build cho
Windows. Build trên Linux sẽ **không** tải `windows-sys` — Cargo tự lọc, bạn không
cần làm gì thêm.

---

## 6. Ví dụ hiện thực từng OS (đã kiểm tra cú pháp bằng `rustc`)

### 6.1. `common/` — dùng chung, không phụ thuộc OS

```rust
// common/sector.rs
pub const DEFAULT_SECTOR_SIZE: usize = 512;
```

```rust
// common/alignment.rs
pub fn is_aligned(offset: u64, sector_size: u64) -> bool {
    offset % sector_size == 0
}
```

### 6.2. `linux/disk/enumerate.rs` — liệt kê ổ vật lý qua `/sys/block`

```rust
use crate::error::PlatformError;
use crate::traits::disk::DiskPlatform;
use crate::types::PhysicalDiskInfo;
use std::fs;

pub struct LinuxDiskPlatform;

impl DiskPlatform for LinuxDiskPlatform {
    fn enumerate() -> Result<Vec<PhysicalDiskInfo>, PlatformError> {
        let mut disks = Vec::new();
        let entries = fs::read_dir("/sys/block")?;

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("loop") || name.starts_with("ram") {
                continue; // bỏ qua thiết bị ảo
            }

            let size_sectors: u64 = fs::read_to_string(entry.path().join("size"))
                .ok()
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);

            disks.push(PhysicalDiskInfo {
                disk_id: disks.len() as u32,
                path: format!("/dev/{name}"),
                size_bytes: size_sectors * 512,
                sector_size: 512,
                is_removable: false, // TODO: đọc /sys/block/<name>/removable
            });
        }
        Ok(disks)
    }

    fn is_system_disk(_disk_id: u32) -> Result<bool, PlatformError> {
        // TODO: đối chiếu với thiết bị chứa "/" qua /proc/mounts
        Ok(false)
    }
}
```

### 6.3. `linux/raw/open.rs` — mở raw device, đọc/ghi theo offset

```rust
use crate::error::PlatformError;
use crate::traits::raw::RawDiskPlatform;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

pub struct RawDiskLinux {
    file: File,
    disk_id: u32,
}

impl RawDiskPlatform for RawDiskLinux {
    fn open(disk_id: u32) -> Result<Self, PlatformError> {
        // path thật nên lấy từ enumerate() (vd "/dev/sdb"),
        // đây chỉ minh hoạ mapping đơn giản.
        let letter = (b'a' + disk_id as u8) as char;
        let path = format!("/dev/sd{letter}");
        let file = OpenOptions::new().read(true).write(true).open(&path)?;
        Ok(Self { file, disk_id })
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), PlatformError> {
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(buf)?;
        Ok(())
    }

    fn write_at(&mut self, offset: u64, data: &[u8]) -> Result<(), PlatformError> {
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(data)?;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), PlatformError> {
        self.file.flush()?;
        Ok(())
    }

    fn lock(&mut self) -> Result<(), PlatformError> {
        // TODO: flock(fd, LOCK_EX) qua crate `libc`
        Ok(())
    }

    fn unlock(&mut self) -> Result<(), PlatformError> {
        // TODO: flock(fd, LOCK_UN)
        Ok(())
    }
}
```

### 6.4. `windows/raw/open.rs` — cùng logic, khác path thiết bị

```rust
use crate::error::PlatformError;
use crate::traits::raw::RawDiskPlatform;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

pub struct RawDiskWindows {
    file: File,
    disk_id: u32,
}

impl RawDiskPlatform for RawDiskWindows {
    fn open(disk_id: u32) -> Result<Self, PlatformError> {
        let path = format!(r"\\.\PhysicalDrive{disk_id}");
        let file = OpenOptions::new().read(true).write(true).open(&path)?;
        Ok(Self { file, disk_id })
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> Result<(), PlatformError> {
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(buf)?;
        Ok(())
    }

    fn write_at(&mut self, offset: u64, data: &[u8]) -> Result<(), PlatformError> {
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(data)?;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), PlatformError> {
        self.file.flush()?;
        Ok(())
    }

    fn lock(&mut self) -> Result<(), PlatformError> {
        // TODO: DeviceIoControl(FSCTL_LOCK_VOLUME) qua crate `windows-sys`
        // Bắt buộc trước khi ghi raw nếu ổ đang có volume được Windows mount,
        // nếu không request write có thể bị từ chối hoặc gây bất đồng bộ cache.
        Ok(())
    }

    fn unlock(&mut self) -> Result<(), PlatformError> {
        // TODO: DeviceIoControl(FSCTL_UNLOCK_VOLUME)
        Ok(())
    }
}
```

> Phần thân `open/read_at/write_at/flush` dùng `std::fs`/`std::io` nên **giống hệt
> nhau về logic** giữa Windows và Linux — chỉ khác chuỗi path thiết bị. Phần `lock`
> thật sự khác nhau (FSCTL trên Windows, `flock`/`ioctl` trên Linux) nên để TODO,
> triển khai bằng `windows-sys`/`libc` ở bước sau.

---

## 7. Vài chỗ khác cũng cần điều chỉnh so với đề xuất gốc

1. **macOS: có 2 loại device path, không chỉ 1.**
   `/dev/diskN` là block device (có buffer cache), `/dev/rdiskN` mới là **raw/character
   device** (I/O trực tiếp, không qua cache) — nên dùng `rdiskN` cho storage engine.
   Bản gốc chưa nói rõ điểm này, cần thêm vào `macos/raw/open.rs`.

2. **Windows: chỉ mở `\\.\PhysicalDriveN` là chưa đủ an toàn nếu ổ đang có volume
   mounted.** Cần gọi `FSCTL_LOCK_VOLUME` (và có thể `FSCTL_DISMOUNT_VOLUME`) trước
   khi ghi raw, nếu không Windows có thể từ chối ghi hoặc dữ liệu cache/volume cũ
   không đồng bộ với dữ liệu mới ghi trực tiếp xuống ổ. Đây là lý do `lock.rs` nằm
   trong `raw/` — không phải tính năng phụ, mà **bắt buộc** trước khi format thật.

3. **Enumerate ở Windows nên dùng SetupAPI/WMI, không nên "đoán" theo index
   `PhysicalDrive0..N`.** Cách đoán index (dùng ở bản test nhanh trước đó) chỉ phù hợp
   để thử nghiệm cá nhân; bản chính thức trong `windows/disk/enumerate.rs` nên gọi
   SetupAPI để lấy đúng danh sách + model/serial, tránh lệch số thứ tự khi cắm/rút ổ.

---

## 8. Nguyên tắc thiết kế (giữ nguyên, vẫn hợp lý)

- **Platform Only** — crate này không biết Storage Engine hoạt động ra sao.
- **Cross Platform** — API public giống nhau trên cả 3 OS, khác biệt nằm trong
  implementation, được `cfg(target_os = ...)` chọn tự động lúc build.
- **Raw First** — thao tác trên Physical Disk, không phụ thuộc Drive Letter/Mount
  Point/Filesystem.
- **Không chứa Storage Logic** — Object/Chunk/Segment/Allocation/Compression thuộc
  `core-storage`, không đặt ở đây.

---

## 9. Checklist việc cần làm tiếp

- [ ] Tạo skeleton thư mục đúng như mục 2 trong repo `core-storage-platform`.
- [ ] Copy các file mẫu ở mục 6 vào đúng vị trí (đã kiểm tra cú pháp bằng `rustc`,
      nhánh Linux/portable — nhánh Windows/macOS cần build thật trên máy tương ứng
      để kiểm tra do phụ thuộc `windows-sys`/`libc`/IOKit).
- [ ] Thêm `Cargo.toml` với `[target.'cfg(...)'.dependencies]` như mục 5.3.
- [ ] Triển khai `lock()`/`unlock()` thật cho Windows (FSCTL_LOCK_VOLUME) trước khi
      cho phép `format/initialize.rs` ghi superblock lên ổ đang mounted.
- [ ] Thay enumerate kiểu "đoán index" bằng SetupAPI/WMI (Windows) và bổ sung
      `removable`/`is_system_disk` thật cho Linux (`/proc/mounts`) và macOS (IOKit).
- [ ] Viết test tích hợp trên máy test thật (ổ rời) cho từng OS trước khi
      `core-storage` bắt đầu dùng crate này.