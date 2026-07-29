# Plan: Cấu trúc Format Lưu trữ (Storage Format) — v3

## 1. Bối cảnh & mục tiêu

Thiết kế lại cấu trúc file lưu trữ dạng segment/chunk sao cho:
- Không phụ thuộc SQLite để tính offset ghi tiếp theo trong vận hành bình thường.
- Segment tự chứa đủ metadata cần thiết (self-describing ở mức tối thiểu, không dư thừa).
- SQLite chỉ đóng đúng vai trò **index tra cứu cho việc đọc** (id → vị trí) và **tracking chunk đã xóa**, không còn là dependency bắt buộc của **write path**.

## 2. Tổng quan layout file

```
+----------+---------+------------------------+------------------------+-----+---------------------------+-----------------+
| Header   | Bitmap  | Segment 1              | Segment 2              | ... | Segment N (Còn lại)       | Header mirror   |
|          |         | [Header:4KB][Data:all] | [Header:4KB][Data:all] |     | [Header:4KB][Data:part]   |                 |
+----------+---------+------------------------+------------------------+-----+---------------------------+-----------------+
```

- Tổng dung lượng file được **biết trước** → toàn bộ segment được **pre-allocate** ngay lúc tạo file (không cần logic lazy-init).
- Mọi segment có `segment_size_bytes` cố định, **trừ segment cuối cùng** dùng `last_segment_size_bytes` (phần dư còn lại).
- `Header` (ngoài) lưu: `segment_size_bytes`, `last_segment_size_bytes`, tổng số segment, `free_segment_count`, ... và có `Header mirror` để chống crash/corruption ở tầng metadata toàn cục.

## 3. Bitmap

- **Granularity: segment-level**, không phải chunk-level.
- Mỗi bit tương ứng 1 segment: `1` = segment đã đầy (full), `0` = segment còn chỗ trống để append.
- Dùng để tìm nhanh segment khả dụng tiếp theo khi ghi (`find_non_full_segment`).
- Không xung đột với `chunk_count` trong segment header vì khác granularity (segment vs chunk).

## 4. Segment Header (4KB / segment)

### Các field đã loại bỏ và lý do

| Field bị loại | Lý do |
|---|---|
| `magic number` / `crc` | Header ngoài đã đảm nhiệm việc xác thực; không cần duplicate ở từng segment |
| `segment_id` | Suy ra được từ index qua bitmap/offset, không cần lưu |
| `flags` (open/sealed/full) | Suy ra được từ `chunk_count` so với `chunk_capacity` |
| `chunk_capacity` | Tính runtime: `segment_size_bytes / chunk_size` (hoặc `last_segment_size_bytes` với segment cuối) |
| `last_cursor_offset` | Suy ra trực tiếp từ `chunk_count * chunk_size` vì chunk size cố định, ghi tuần tự |

### Field giữ lại

```
+---------------------+----------+--------------------------------------------+
| Field                | Size     | Ý nghĩa                                     |
+---------------------+----------+--------------------------------------------+
| chunk_count          | 4/8 bytes| High-water mark — số chunk đã ghi tuần tự   |
|                       |          | → vị trí ghi tiếp theo = chunk_count * chunk_size |
| tombstone_count       | 4 bytes  | (optional) Số chunk đã bị đánh dấu xóa      |
|                       |          | trong segment này — dùng để quyết định      |
|                       |          | trigger compaction mà không cần query SQLite|
+---------------------+----------+--------------------------------------------+
```

Header vẫn giữ nguyên kích thước 4KB dù chỉ dùng vài chục byte, vì:
- Align theo page size (4KB) của OS/SSD, tốt cho ghi/đọc.
- Chừa chỗ mở rộng field sau này mà không phá vỡ offset layout hiện tại.

### Khởi tạo (eager init)

Vì tổng dung lượng biết trước, toàn bộ header của N segment được ghi **1 lần duy nhất lúc tạo file** (buffer zero 4KB cho mỗi segment, vì `chunk_count = 0` mặc định là toàn số 0). Không cần lazy-init hay check "segment đã init chưa" ở runtime.

## 5. Write Path (ghi chunk mới)

**Nguyên tắc bắt buộc**: chunk data phải được ghi (và fsync nếu cần durability mạnh) **trước khi** tăng `chunk_count`. Đảo ngược thứ tự này khiến header có thể "claim" một chunk chưa thực sự tồn tại trên đĩa sau khi crash.

```rust
fn write_chunk(data: &[u8]) -> Result<Location> {
    // Bước 1: còn segment chưa đầy? → append bình thường, KHÔNG đụng SQLite
    if let Some(seg) = bitmap.find_non_full_segment() {  // O(1) qua free_segment_count
        return append_to_segment(seg, data);
    }

    // Bước 2: hết chỗ append mới → mới tra "lỗ" (hole) đã xóa trong SQLite
    if let Some((seg, chunk_idx)) = sqlite.pop_one_hole() {
        return write_into_hole(seg, chunk_idx, data);
    }

    // Bước 3: hết cả lỗ → check riêng segment cuối (phần "Còn lại", size khác các segment khác)
    if let Some(slot) = last_segment.remaining_slot() {
        return append_to_segment(last_segment, data);
    }

    // Bước 4: thực sự hết chỗ
    Err(StorageFull)
}

fn append_to_segment(seg: SegmentRef, data: &[u8]) -> Result<Location> {
    let offset = seg.header.chunk_count * CHUNK_SIZE;
    write_at(seg, offset, data)?;
    // fsync nếu cần durability mạnh
    seg.header.chunk_count += 1;              // update SAU khi data đã ghi xong
    persist_segment_header(seg)?;
    if seg.header.chunk_count == seg.chunk_capacity() {
        bitmap.set_full(seg.index);            // segment đầy → set bit = 1
        header.free_segment_count -= 1;         // giảm counter O(1)
    }
    Ok(Location { segment: seg.index, chunk: seg.header.chunk_count - 1 })
}
```

**Ghi header hiệu quả**: update `chunk_count` chỉ cần `pwrite` đúng offset field đó (vài byte), không cần rewrite toàn bộ 4KB mỗi lần.

**`free_segment_count`**: giữ 1 counter ở header ngoài (không phải scan bitmap mỗi lần) để check "còn segment free không" trong O(1).

## 6. Delete Path (xóa chunk)

- Đã chốt: **xóa hiếm/ít**, không cần tái sử dụng không gian ngay lập tức → ưu tiên write path đơn giản, không phụ thuộc SQLite.
- Xóa chunk **không xóa record trong SQLite**, mà **move sang 1 bảng khác** (ví dụ `deleted_chunks`) — không có job xóa vì ghi đè (overwrite record khi hole được tái sử dụng) tốt hơn.
- Không đụng tới bitmap hoặc segment header khi xóa (trừ khi có dùng `tombstone_count` optional để tăng lên, phục vụ quyết định compaction).

```
Xóa chunk tại (segment_index, chunk_index):
  1. Move record từ bảng "in_use" sang bảng "deleted_chunks" trong SQLite
  2. (optional) tăng tombstone_count trong header segment tương ứng
  3. Không đụng bitmap, không đụng chunk_count
```

## 7. Hole-fill (tái sử dụng chunk đã xóa)

Chỉ kích hoạt khi **không còn segment nào free để append mới** (bước 2 trong Write Path) — không phải khi bitmap toàn `1111...1111` mới xử lý (2 cách diễn đạt tương đương, nhưng dùng `free_segment_count == 0` làm điều kiện trigger thì rẻ hơn vì check O(1)).

```sql
BEGIN IMMEDIATE;
SELECT id, segment_index, chunk_index FROM deleted_chunks LIMIT 1;
DELETE FROM deleted_chunks WHERE id = ?;   -- hoặc move ngược lại sang bảng "in_use"
COMMIT;
```

Dùng transaction để tránh race condition khi có nhiều writer song song cùng tranh 1 hole.

Trong thực tế vận hành (xóa hiếm), bước hole-fill gần như **không bao giờ được kích hoạt** — SQLite chỉ tham gia khi hệ thống đã dùng gần hết dung lượng pre-allocate.

## 8. Compaction (dọn dẹp, chạy nền — để dành xử lý sau)

Không có job xóa ngay; dọn dẹp/tái chế không gian rác được xử lý độc lập, tách khỏi write path:

1. Chỉ compact **segment đã sealed** (bitmap = 1, đầy) — không đụng segment đang active để tránh race với writer.
2. **Trigger**: khi tỉ lệ `tombstone_count / chunk_count` của 1 segment vượt ngưỡng (ví dụ >30%).
3. **Quy trình**: đọc tuần tự các chunk còn sống (không tombstone) trong segment cũ → ghi liền mạch vào 1 segment trống khác → update SQLite trỏ id sang vị trí mới → reset bitmap bit của segment cũ về `0` (trống, sẵn sàng tái sử dụng vì tổng dung lượng cố định, không cần mở rộng file).

> Mục này chỉ là định hướng, **chưa đi vào chi tiết implement** — để xử lý ở giai đoạn sau.

## 9. Bảng tóm tắt vai trò từng thành phần

| Thành phần | Vai trò | Phụ thuộc SQLite trên write path? |
|---|---|---|
| Header ngoài + mirror | Metadata toàn cục, chống crash tầng metadata | Không |
| Bitmap | Segment nào đầy/trống (segment-level) | Không |
| Segment header `chunk_count` | Vị trí ghi tiếp theo trong segment (high-water mark) | Không |
| Segment header `tombstone_count` (optional) | Trigger compaction mà không cần SELECT SQLite | Không |
| SQLite (bảng in_use) | Index tra cứu id → (segment, chunk) cho việc đọc | — |
| SQLite (bảng deleted_chunks) | Danh sách hole để tái sử dụng khi hết chỗ append mới | Chỉ khi `free_segment_count == 0` |

## 10. Việc còn để mở (chưa chốt trong phiên thảo luận này)

- File có dùng `fallocate`/`ftruncate` để cấp phát thật toàn bộ dung lượng lúc tạo, hay để sparse (data region chưa chạm đĩa cho tới khi ghi thật)?
- Có cần checksum riêng cho `chunk_count` (field duy nhất thay đổi liên tục, hiện chưa có cơ chế phát hiện corruption) hay chấp nhận rủi ro nhỏ để giữ đơn giản?
- Chi tiết implement compaction (mục 8) — thuật toán chọn segment đích, xử lý lỗi giữa chừng khi đang compact, v.v.