# On-Disk Format v1 — RPFS Raw Segment Layout

> Áp dụng cho Hướng B (raw device) kết hợp Giai đoạn 2 (segment) — tức trạng thái trưởng thành cuối cùng của project. Hướng A (dùng NTFS/ext4) và Giai đoạn 1 (chunk-per-file) không cần format này, vẫn dùng `fs::write`/`fs::read` bình thường.

## 1. Nguyên tắc tối ưu chính (khác với đề xuất nháp ban đầu)

Đề xuất ban đầu (tài liệu RPSoft Format, mục 4) có 6 vùng: Superblock, Allocation Bitmap, **Object Table**, **Chunk Table**, Chunk Data, Segment Data.

Bản này **bỏ Object Table và Chunk Table khỏi raw disk**, chỉ còn:

```
Superblock  →  Segment Allocation Bitmap  →  Segment Region
```

Lý do: tài liệu Storage Engine đã chốt nguyên tắc "mọi lượt đọc tra metadata DB trên SSD rồi nhảy thẳng, không bao giờ scan HDD". Nếu vẫn giữ Object/Chunk Table trên HDD, mỗi lần ghi/xoá phải đồng bộ 2 index (SSD + HDD) và Chunk Table trên HDD sẽ bị ghi ngẫu nhiên (random I/O) — đúng thứ toàn bộ kiến trúc segment đang tránh. Thay vào đó, **mỗi chunk tự mang theo header nhỏ ngay trong segment** (chunk_id + length + crc32) — đủ để dựng lại metadata bằng cách quét tuần tự nếu SSD mất, mà không cần một bảng index thứ hai phải maintain thường trực.

Allocation Bitmap vẫn giữ, nhưng chỉ ở **mức segment** (segment nào free/used), không phải mức chunk — vì bên trong segment là append-only, không cần bitmap cấp byte.

## 2. Layout tổng thể trên đĩa

```
[Protective MBR + GPT Header + GPT Partition Table]   (~1MB, chuẩn GPT)
   Partition Entry: type GUID riêng "RPFS Reserved", span hết phần còn lại
   → giải quyết rủi ro #1 ở tài liệu RPSoft Format: ổ không bao giờ hiện
     "Uninitialized Disk" trong Windows, tránh bấm nhầm Initialize Disk.

[RPFS Partition]
  offset 0            Superblock A            (4096 byte, 1 sector)
  offset 4096          Segment Allocation Bitmap
  offset bitmap_end    Segment Region          (chiếm phần lớn dung lượng)
  offset (cuối - 4096) Superblock B (bản mirror)
```

`disk_uuid` trong Superblock kế thừa đúng UUID đang dùng trong `disk.json`. Lưu ý: ở Hướng B **không còn `disk.json` dạng file JSON nữa** — raw device không có filesystem để đặt file. Superblock **thay thế hoàn toàn** vai trò của `disk.json` (định danh + trạng thái RW/RO...), không phải chạy song song với nó.

## 3. Superblock (4096 byte, sector-aligned)

| Offset | Size | Field | Ghi chú |
|---|---|---|---|
| 0 | 4 | `magic` | `b"RPFS"` |
| 4 | 4 | `version` | u32 |
| 8 | 16 | `disk_uuid` | khớp `disk.json` |
| 24 | 8 | `generation` | u64, tăng dần mỗi lần commit; mount chọn bản có generation cao nhất **và** CRC hợp lệ (kiểu ZFS uberblock) |
| 32 | 8 | `created_at_ms` | u64 |
| 40 | 8 | `updated_at_ms` | u64 |
| 48 | 1 | `disk_state` | 0=RW, 1=RO, 2=Migrating, 3=Retired — đổi trạng thái ổ chỉ bằng field này, không cần sửa code/Docker |
| 49 | 1 | `sector_size_pow2` | vd 12 = 4096 (2^12) |
| 50 | 2 | `reserved0` | |
| 52 | 8 | `total_bytes` | dung lượng partition RPFS quản lý |
| 60 | 4 | `segment_size_bytes` | mặc định, xem mục 9 |
| 64 | 4 | `segment_count` | tính sẵn lúc format, tránh chia lại mỗi lần mount |
| 68 | 8 | `bitmap_offset` | |
| 76 | 8 | `bitmap_size_bytes` | |
| 84 | 8 | `segment_region_offset` | |
| 92 | 32 | `reserved1` | dự trữ mở rộng schema về sau, không phải migrate toàn bộ |
| 124 | 4 | `header_crc32` | CRC32C của 124 byte phía trên |
| 128 | 3968 | padding | lấp đủ 1 sector |

Ghi Superblock: viết bản mới, tính CRC, mới tăng `generation`, fsync — **không sửa in-place bản đang active**. Nếu crash giữa chừng ghi A, CRC của A sẽ sai → mount tự rơi về B (hoặc ngược lại). Đây là cơ chế transaction đơn giản thay cho một WAL riêng ở tầng ổ raw.

## 4. Segment Allocation Bitmap

1 bit / segment slot (0 = free, 1 = used). Kích thước = `ceil(segment_count / 8)` byte — với ổ 10TB, segment 512MB → 20.000 segment → bitmap ~2.5KB, không đáng kể. Cập nhật bitmap chỉ xảy ra khi **cấp phát segment mới** hoặc **thật sự giải phóng segment** (mục 10 tài liệu Storage Engine) — tần suất thấp, không phải mỗi lần ghi chunk.

Segment ở trạng thái `pending_trash` (xem mục 5) vẫn tính là "used" trong bitmap — bit chỉ chuyển về 0 khi một tiến trình nền quét thấy `pending_free_at_ms` đã qua, lúc đó mới tăng `epoch` và cho phép cấp phát lại slot.

## 5. Segment — header + chunk record tự mô tả

### Segment header (64 byte, đầu mỗi slot)

| Offset | Size | Field | Ghi chú |
|---|---|---|---|
| 0 | 4 | `magic` | `b"SEG1"` |
| 4 | 4 | `segment_id` | u32 — đồng thời là **chỉ số slot** (xem công thức mục 6) |
| 8 | 4 | `epoch` | u32 — tăng mỗi lần slot này được **tái cấp phát** cho vòng đời dữ liệu mới (sau compaction giải phóng). Metadata.db cần lưu kèm `SegmentEpoch` bên cạnh `SegmentID`; khi đọc, nếu epoch đọc được từ header khác epoch trong metadata → coi là stale, từ chối đọc thay vì trả nhầm dữ liệu của lần dùng slot trước |
| 12 | 16 | `disk_uuid` | phải khớp Superblock — chống đọc nhầm khi ổ bị tráo |
| 28 | 8 | `created_at_ms` | |
| 36 | 1 | `state` | 0=empty, 1=active, 2=sealed, 3=compacting, 4=pending_trash, 5=obsolete |
| 37 | 8 | `pending_free_at_ms` | mốc thời gian slot đủ điều kiện bị ghi đè thật; 0 nếu không ở trạng thái `pending_trash`. Đây là phần tương đương `trash/` ở Giai đoạn 1 — segment vẫn giữ nguyên (bitmap vẫn đánh dấu "used"), chỉ thật sự tái cấp phát sau khi qua mốc này |
| 45 | 15 | `reserved` | |
| 60 | 4 | `header_crc32` | |

### Chunk record (lặp lại append-only ngay sau header)

| Offset (tương đối) | Size | Field | Ghi chú |
|---|---|---|---|
| 0 | 1 | `record_magic` | `0xC5` — cờ nhận diện điểm bắt đầu record, phục vụ scan phục hồi |
| 1 | 8 | `chunk_id` | u64, **cùng giá trị ChunkID auto-increment trong metadata.db** — đây là phần "tự mô tả" |
| 9 | 4 | `length` | u32 |
| 13 | 4 | `data_crc32` | CRC32C của payload |
| 17 | length | `payload` | dữ liệu chunk thật |

Overhead 17 byte/chunk — với chunk 4MB là 0.0004%, không đáng kể.

**Vì sao dùng CRC32C (Castagnoli) thay vì CRC32 thường hay xxhash:** có instruction phần cứng tăng tốc trên cả x86 (SSE4.2 `CRC32`) lẫn ARM, và là lựa chọn đã được ext4/btrfs/iSCSI dùng cho việc tương tự — vừa nhanh vừa đủ mạnh để phát hiện bit rot, không cần thêm dependency như xxhash. Giải quyết luôn mục "CRC32/xxhash — cần chọn" ở Phase 5 tài liệu RPSoft Format.

## 6. Công thức tính offset vật lý (không cần bảng index trên HDD)

```
physical_offset = segment_region_offset
                 + segment_id * segment_size_bytes   // segment_id = index slot
                 + 64                                 // bỏ qua segment header
                 + offset_within_segment              // lấy từ metadata.db (field Offset)
```

Metadata.db trên SSD không đổi schema so với tài liệu Storage Engine mục 5 (`DiskID, SegmentID, Offset, Length, Checksum`) — chỉ cần đảm bảo `SegmentID` được sinh trùng với `segment_id` (index slot) ở đây. Read path vẫn đúng y hệt mục 7 tài liệu đó: tra metadata → `seek()` → `read()`, không thêm bước nào.

## 7. Recovery / Scrub — khi cần quét thay vì tra bảng

Hai tình huống bắt buộc phải quét tuần tự thay vì tin metadata:

1. **Crash giữa lúc ghi**: segment có `state = active` khi mount lại → record cuối cùng có thể ghi dở. Quét từng record bằng `record_magic (0xC5) → length → verify data_crc32`; gặp record đầu tiên không hợp lệ → đó là write cursor thật, **truncate** segment tại đó, báo lại cho metadata.db (giống cơ chế Kafka dùng cho segment log).
2. **Mất/hỏng metadata.db**: quét toàn bộ segment, mỗi record hợp lệ cho ra đúng bộ `(chunk_id, disk_uuid, segment_id, offset, length, checksum)` cần thiết để dựng lại một dòng metadata — không cần Object/Chunk Table dự phòng trên HDD như bản nháp ban đầu.
3. **Chunk quét được nhưng không khớp entry nào trong metadata.db còn sống** (ví dụ compaction crash giữa chừng: đã copy sang segment mới, cập nhật metadata, nhưng segment cũ chưa kịp chuyển `pending_trash` thì mất điện) — đây là ca tương đương `lost+found/` ở Giai đoạn 1. Không tự xoá; đánh dấu "orphan chờ xác nhận" để công cụ soát riêng xử lý trước khi cho phép slot chuyển sang `pending_trash`.

## 8. (Tuỳ chọn) Checkpoint index cục bộ — tăng tốc mount

Quét toàn ổ mỗi lần mount chỉ chấp nhận được khi cần recovery, không nên là đường mount bình thường. Có thể ghi thêm 1 vùng nhỏ (vài MB, đặt sau Superblock B hoặc trong `reserved1`) chứa snapshot `segment_id → (state, write_cursor)`, **fsync theo chu kỳ** (mỗi N segment được sealed, không phải mỗi write) — không tạo random I/O vì tần suất thấp. Đây chỉ là cache tăng tốc, **không phải nguồn chân lý** (canonical vẫn là SSD) nên không vi phạm nguyên tắc ở mục 1.

## 9. Giá trị mặc định đề xuất

| Tham số | Giá trị đề xuất | Căn cứ |
|---|---|---|
| `segment_size_bytes` | 512MB | giữa khoảng 256MB–1GB đã chốt ở tài liệu Storage Engine mục 4 |
| `sector_size` | 4096 (Advanced Format) | tương thích ổ HDD hiện đại, alignment cho O_DIRECT |
| Checksum | CRC32C | xem mục 5 |
| ChunkID | u64 auto-increment | đã chốt ở tài liệu Storage Engine mục 9 |
| Số bản Superblock | 2 (đầu + cuối partition) | theo Phase 5 tài liệu RPSoft Format |

## 10. Đã giải quyết vs vẫn còn mở

**Giải quyết bằng format này:**
- Không cần WAL riêng ở tầng data-path trên HDD — record tự mô tả + CRC đã cho crash-safety tương đương (WAL trên SSD ở tài liệu Storage Engine mục 8 chỉ còn phục vụ metadata mutation, đúng vai trò ban đầu).
- Bỏ được Object Table/Chunk Table khỏi raw disk → không còn nguy cơ lệch trạng thái giữa 2 index (SSD và HDD) mà Phase 4 tài liệu RPSoft Format lo ngại.
- Rủi ro "Uninitialized Disk" (mục 6.1 tài liệu RPSoft Format) được xử lý ở layer GPT, không phải layer RPFS.
- Có cơ chế grace-period trước khi xoá thật (`pending_trash` + `pending_free_at_ms`), tương đương `trash/` ở Giai đoạn 1.
- Có nhánh xử lý chunk mồ côi không khớp metadata, tương đương `lost+found/` ở Giai đoạn 1.
- Có `epoch` chống đọc nhầm dữ liệu cũ khi một slot segment bị tái sử dụng nhiều lần trong vòng đời ổ đĩa.

**Vẫn còn mở, chưa đủ dữ kiện để chốt ở đây:**
- Thuật toán chọn segment nào để ghi tiếp khi có nhiều segment `active` đồng thời (nếu hệ thống cho ghi song song nhiều writer).
- Ngưỡng khi nào chuyển `state: active → sealed` (theo dung lượng đầy hay theo thời gian).
- **Tần suất fsync khi ghi từng chunk record** (không phải Superblock) — CRC chỉ giúp *phát hiện* ghi dở dang, không quyết định *mất bao nhiêu dữ liệu* là chấp nhận được giữa 2 lần fsync. Vẫn là quyết định policy, chưa giải quyết bằng format.
- **Lịch trình kích hoạt scrubbing chủ động** (định kỳ, không đợi crash) — mục 7 mới có cơ chế quét, chưa có lịch/threshold trigger.
- Giá trị grace period cho `pending_free_at_ms` (bao lâu thì thật sự tái sử dụng slot) — tương tự việc `trash/` ở Giai đoạn 1 cũng chưa có thời hạn cụ thể.
- Chi tiết migrate dữ liệu từ Giai đoạn 1 (chunk-per-file trên filesystem thường) sang layout raw segment này — cần một tool đọc `chunks/` cũ và ghi tuần tự vào Segment Region theo đúng format trên.