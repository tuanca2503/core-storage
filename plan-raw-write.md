# KẾ HOẠCH PHÁT TRIỂN: Raw Disk Storage Engine (RPSoft Format)

> Tổng hợp từ buổi thảo luận kiến trúc — chuyển từ lưu file qua NTFS/ext4 sang đọc/ghi trực tiếp thiết bị (raw device), tự quản lý layout dữ liệu như một storage engine/filesystem thu nhỏ.

---

## 1. Mục tiêu

Xây dựng một **storage engine tự quản lý toàn bộ ổ đĩa**, không phụ thuộc vào filesystem của hệ điều hành (NTFS/ext4). Ứng dụng sẽ:

- Mở trực tiếp thiết bị vật lý (`\\.\PhysicalDriveN` trên Windows, `/dev/sdX` trên Linux).
- Tự định nghĩa layout dữ liệu (superblock, bảng object, bảng chunk...).
- Tự quản lý cấp phát/thu hồi vùng trống, thay vì dựa vào NTFS/ext4.

**Không mục tiêu (out of scope ở giai đoạn đầu):** viết một filesystem tổng quát dùng được cho mọi loại dữ liệu như NTFS. Mục tiêu là một **object store chuyên dụng** cho project hiện tại (Disk → Object → Chunk → Segment).

---

## 2. Hai hướng tiếp cận — cần chốt trước khi code

| Tiêu chí | Hướng A: Dùng full dung lượng, vẫn giữ NTFS/ext4 | Hướng B: Raw disk, format riêng, bỏ NTFS/ext4 |
|---|---|---|
| Độ phức tạp | Thấp — vẫn dùng `fs::write`, `fs::read` | Rất cao — phải tự viết allocator, index, recovery |
| Hiệu năng | Giới hạn bởi filesystem OS | Có thể tối ưu tối đa theo nhu cầu riêng |
| An toàn dữ liệu | NTFS/ext4 đã lo journal, bad sector, crash recovery | Phải tự làm: WAL, checksum, chống mất điện |
| Người dùng thường mở nhầm | Có (Explorer thấy file, dễ xoá nhầm) | Không (Explorer không đọc được) |
| Rủi ro mất dữ liệu do thao tác nhầm (Initialize Disk...) | Không có | Có — cần cảnh báo rõ |
| Thời gian phát triển | Ngắn | Dài, cần nhiều vòng test |
| Phù hợp giai đoạn | MVP, demo, sản phẩm sớm | Sản phẩm trưởng thành, cần kiểm soát toàn bộ layout |

**Khuyến nghị:** Bắt đầu bằng **Hướng A** để có sản phẩm chạy được sớm, đồng thời thiết kế layer trừu tượng (`StorageBackend` trait) để sau này có thể cắm **Hướng B** vào mà không phải viết lại toàn bộ logic Object/Chunk/Segment.

---

## 3. Ba khái niệm cần phân biệt rõ (hay bị nhầm)

```
Physical Disk  (thiết bị vật lý — luôn được OS/phần cứng nhận diện)
      │
      ▼
Partition      (GPT/MBR — vùng chia trên đĩa, có hoặc không)
      │
      ▼
Filesystem     (NTFS/ext4/RPSoft — cách tổ chức dữ liệu bên trong partition)
```

- **Windows/Device Manager luôn thấy Physical Disk** — không thể ẩn ở mức phần mềm thông thường (user-mode).
- **Disk Management thấy Partition** — dù filesystem là gì, kể cả "Unknown"/"RAW".
- **Explorer chỉ hiện thứ đã mount được** — nếu filesystem không được Windows nhận dạng, ổ sẽ **không xuất hiện trong "This PC"**, nhưng vẫn thấy trong Disk Management dưới dạng `Unknown` hoặc `RAW`.

➡️ Kết luận: **Không thể "giấu ổ cứng"**. Cái đạt được là ổ trở thành thiết bị có định dạng mà Windows Explorer không hiểu, nên người dùng thường không thao tác nhầm qua Explorer. Admin/root vẫn luôn truy cập được raw device.

---

## 4. Thiết kế On-Disk Layout (đề xuất ban đầu)

```
Disk
│
├── Superblock            (offset 0, vài KB đầu — magic number, version, metadata gốc)
│
├── Allocation Bitmap      (theo dõi vùng trống/đã dùng)
│
├── Object Table           (Object ID -> vị trí Chunk)
│
├── Chunk Table            (Chunk ID -> offset + độ dài)
│
├── Chunk Data             (dữ liệu thật, chiếm phần lớn ổ)
│
└── Segment Data           (nếu tách riêng theo segment)
```

Ví dụ ánh xạ khi ghi Object ID = 100:

```
Object ID 100
   → Object Table: tra offset trong Chunk Table
   → Chunk Table:  Chunk #123, offset X, length Y
   → seek(X) trên raw device
   → write(data)
```

**Lưu ý thiết kế:**
- Superblock nên có **magic number riêng** (vd. `b"RPFS"`) + version, để dễ nhận diện và versioning về sau.
- Cần chừa vùng **reserved** ở đầu để mở rộng schema mà không phải migrate toàn bộ.
- Nên có ít nhất 2 bản copy của Superblock (đầu & cuối đĩa) để chống hỏng do crash giữa chừng ghi.

---

## 5. Vấn đề then chốt: Phát hiện ổ đĩa (Disk Detection)

Đây là điểm quan trọng nhất cần thay đổi trong code hiện tại.

### 5.1. Vấn đề với `sysinfo::Disks`

```rust
let disks = Disks::new_with_refreshed_list();
```

- `sysinfo::Disks` chỉ liệt kê **volume/filesystem đã mount**, **không** liệt kê physical disk.
- Khi ổ đã bị format sang RPSoft (không còn NTFS) → ổ đó **không mount được** → `sysinfo::Disks` **sẽ không còn thấy ổ này nữa**.
- ⚠️ Đây là lỗi kiến trúc nếu tiếp tục dùng `sysinfo` làm nguồn phát hiện ổ chính, vì logic sẽ "mất" đúng ổ mà ứng dụng cần quản lý.

### 5.2. Giải pháp: Viết `DiskEnumerator` riêng theo từng platform

| Platform | API cần dùng | Ghi chú |
|---|---|---|
| Windows | `SetupAPI`, `Configuration Manager`, `DeviceIoControl`, `SetupDiEnumDeviceInfo`, hoặc WMI | Lấy danh sách `Disk0, Disk1, Disk2...` bất kể có mount hay không |
| Linux | Đọc `/sys/block/` | Liệt kê toàn bộ block device vật lý |
| macOS | `IOKit` | Tương đương SetupAPI trên Windows |

Đây cũng là cách các phần mềm thực tế đã làm: **Rufus, BalenaEtcher, CrystalDiskInfo** đều enumerate theo physical device, không dựa vào danh sách volume đã mount — vì vậy chúng vẫn thấy được ổ chưa format, ổ RAW, ổ chưa có filesystem.

### 5.3. Việc cần làm trong code

- [ ] Tạo trait `DiskEnumerator` với method `list_physical_disks() -> Vec<PhysicalDiskInfo>`.
- [ ] Implement riêng cho Windows (feature-gated `#[cfg(windows)]`) dùng SetupAPI/WMI.
- [ ] Implement riêng cho Linux (`#[cfg(target_os = "linux")]`) đọc `/sys/block`.
- [ ] Giữ `sysinfo` chỉ để lấy thông tin **volume đã mount** (dùng cho các ổ NTFS/ext4 bình thường), **không** dùng cho ổ RPSoft.
- [ ] Viết lớp phân loại: với mỗi physical disk phát hiện được → thử đọc Superblock ở offset 0 → xác định là `NTFS | RPSoft | Unknown | RAW`.

```
Disk0 → đọc Superblock → "NTFS"    → dùng path Hướng A
Disk1 → đọc Superblock → "RPFS"    → dùng path Hướng B (raw engine)
Disk2 → đọc Superblock → "Unknown" → cảnh báo user, không tự động đụng vào
```

---

## 6. Rủi ro & Lưu ý an toàn (bắt buộc đọc trước khi triển khai)

1. **Không tạo GPT/MBR + không có gì trên ổ** → Windows sẽ hiện "Uninitialized Disk" và mời `Initialize Disk`. Nếu người dùng bấm nhầm → **dữ liệu bị phá hỏng vĩnh viễn**.
   - ➡️ Giảm rủi ro: luôn tạo tối thiểu một **GPT partition entry** (kể cả với type/GUID riêng), để ổ không rơi vào trạng thái "chưa khởi tạo" trần trụi.
2. **Admin/root luôn truy cập được raw device** — đây không phải cơ chế bảo mật, chỉ là tránh thao tác nhầm qua Explorer thông thường.
3. **Không có NTFS/ext4 nghĩa là mất toàn bộ cơ chế an toàn có sẵn**: journal, chống mất điện giữa chừng, bad sector remapping ở mức filesystem, cache flush... → phải tự làm lại từ đầu (xem Phase 5 bên dưới).
4. **Chưa có ổ thật để test** → bắt buộc phải có môi trường test an toàn trước khi đụng vào ổ thật (xem Phase 0).
5. Format nhầm ổ hệ thống (chứa `C:`) là rủi ro nghiêm trọng nhất — cần safeguard ở tầng code (kiểm tra disk không phải boot disk trước khi cho phép format).

---

## 7. Roadmap triển khai theo giai đoạn

### Phase 0 — Môi trường test an toàn (làm trước tiên, không cần ổ thật)
- [ ] Dùng **file ảo (loopback file)** thay cho physical disk: tạo file 1–10 GB, `seek`/`read`/`write` như thể là raw device.
- [ ] Trên Windows: có thể test qua **VHD** (Virtual Hard Disk) mount bằng `diskpart`.
- [ ] Trên Linux: dùng `losetup` để gắn file thành block device giả (`/dev/loopX`).
- [ ] Toàn bộ logic Superblock/Object Table/Chunk Table viết và test trên file ảo trước.

### Phase 1 — DiskEnumerator (thay thế phần phụ thuộc `sysinfo` cho ổ raw)
- [ ] Thiết kế trait `DiskEnumerator` (mục 5.3).
- [ ] Implement Windows + Linux.
- [ ] Viết logic nhận diện Superblock (NTFS / RPFS / Unknown).

### Phase 2 — On-disk Format cơ bản
- [ ] Định nghĩa struct `Superblock` (magic, version, offsets bảng con).
- [ ] Định nghĩa `ObjectTable`, `ChunkTable`, `AllocationBitmap`.
- [ ] Serialize/deserialize bằng `bincode`/`postcard` hoặc tự viết layout tay (khuyến nghị tự viết layout tay vì cần offset cố định, dễ debug bằng hex editor).

### Phase 3 — Raw Read/Write Engine
- [ ] Mở raw device (`\\.\PhysicalDriveN` / `/dev/sdX`) với quyền phù hợp.
- [ ] API nội bộ: `seek(offset)`, `read_exact(buf)`, `write_all(buf)`.
- [ ] API tầng trên: `write_object(id, data)`, `read_object(id) -> data`.

### Phase 4 — Quản lý cấp phát (Allocator)
- [ ] Cấp phát chunk mới khi ghi object.
- [ ] Thu hồi chunk khi xoá object.
- [ ] Cập nhật Allocation Bitmap đồng bộ với Object/Chunk Table (tránh lệch trạng thái).

### Phase 5 — Độ tin cậy (Reliability) — **không được bỏ qua**
- [ ] Write-Ahead Log (WAL) hoặc cơ chế transaction đơn giản trước khi ghi thật vào Chunk Data.
- [ ] Checksum (CRC32/xxhash) cho mỗi chunk để phát hiện hỏng dữ liệu.
- [ ] Cơ chế phục hồi khi khởi động lại sau mất điện giữa chừng ghi (kiểm tra WAL, replay/rollback).
- [ ] Có ít nhất 2 bản Superblock (đầu/cuối đĩa), chọn bản mới nhất khi mount.

### Phase 6 — Safety Guard (bắt buộc trước khi test trên ổ thật)
- [ ] Kiểm tra disk không phải boot disk / disk chứa `C:` trước khi cho phép format.
- [ ] Yêu cầu xác nhận rõ ràng (gõ tên ổ, không chỉ bấm "Yes") trước khi ghi đè.
- [ ] Log lại toàn bộ thao tác format/ghi đè để có thể audit.

### Phase 7 — Test trên ổ thật
- [ ] Test trên **USB rời** trước (rủi ro thấp nếu hỏng).
- [ ] Sau đó mới test trên ổ SATA phụ (không phải ổ hệ điều hành).
- [ ] Kiểm tra hành vi thực tế: Disk Management hiện gì, Explorer có hiện không, `DiskEnumerator` có thấy ổ sau khi format không.

---

## 8. Tham khảo hệ thống thực tế (đối chiếu kiến trúc)

| Hệ thống | Cách quản lý ổ |
|---|---|
| VMware ESXi | VMFS — Windows không đọc được |
| Ceph | Bluestore — object trực tiếp trên raw device |
| TrueNAS | ZFS — Windows thấy "Unknown" |
| Oracle ASM | ASM Header + Allocation Unit riêng |

Kiến trúc hiện tại của project (Disk → Object → Chunk → Segment, SQLite chỉ lưu metadata, data ghi theo chunk) đã gần giống một **object storage engine** hơn là một ứng dụng lưu file thông thường — hướng đi Phase 0–7 ở trên là lộ trình hợp lý để hiện thực hoá điều đó dần dần, thay vì nhảy thẳng vào raw disk khi chưa có hạ tầng test an toàn.

---

## 9. Việc cần quyết định ngay (trước khi viết code Phase 1)

- [ ] Chọn Hướng A hay Hướng B cho MVP đầu tiên (khuyến nghị: A trước, B sau — xem mục 2).
- [ ] Xác nhận có cần giữ tương thích ngược để đọc ổ NTFS thường song song với ổ RPFS hay không.
- [ ] Xác nhận môi trường test Phase 0 (loopback file / VHD / USB rời) trước khi đụng ổ thật.