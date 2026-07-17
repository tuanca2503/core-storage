# Plan: Custom Disk Format Structure

Tài liệu chốt lại toàn bộ thiết kế cấu trúc lưu trữ tùy chỉnh (segment/chunk storage engine) sau quá trình thảo luận.

---

## 1. Tổng quan

- Đơn vị cấp phát lớn: **segment** (kích thước cố định, ví dụ 300MB), quản lý bằng bitmap ngoài (segment-level).
- Đơn vị cấp phát nhỏ: **chunk** (kích thước cố định, ví dụ 4MB), nằm bên trong segment, vị trí xác định bằng **slot_index cố định** (Phương án 1 — không lưu offset rời rạc).
- Metadata/index (object → chunk → vị trí vật lý) lưu trong **SQLite**, đóng vai trò **nguồn sự thật (source of truth)** duy nhất cho việc cấp phát.
- Ghi/đọc thao tác trực tiếp trên raw device/partition, bỏ qua filesystem chuẩn của OS.

---

## 2. Cấu trúc tổng thể ổ đĩa

```
[ Disk header ] [ Bitmap ngoài (segment existence) ] [ Segment 0 ] [ Segment 1 ] ... [ Segment N-1 ] [ Superblock mirror (backup) ]
```

---

## 3. Disk header (đầu ổ)

Lưu 1 lần khi format, không đổi trong suốt vòng đời ổ (trừ khi migrate version):

| Field | Ý nghĩa |
|---|---|
| `magic` | Nhận diện định dạng, tránh đọc nhầm ổ khác |
| `version` | Version của on-disk format, phục vụ migrate sau này |
| `segment_length` | Kích thước cố định của 1 segment |
| `chunk_length` | Kích thước cố định của 1 chunk |
| `total_segments` | Tổng số segment trên ổ |

---

## 4. Bitmap ngoài (segment existence bitmap)

- Nằm ngay sau disk header, kích thước nhỏ (1 bit / segment — ví dụ ổ 1TB / segment 300MB ≈ 3400 segment ≈ 425 byte).
- **Công dụng duy nhất**: đánh dấu segment đã được khởi tạo (có header hợp lệ, có thể đọc) hay chưa từng được tạo (đất trống).
- **Không dùng để tìm segment còn chỗ trống để ghi** — việc đó là của `free_count` trong từng segment (mục 5.1).
- Chỉ cập nhật khi 1 segment **chuyển trạng thái tồn tại**: khởi tạo mới (bit 0→1) hoặc giải phóng hoàn toàn (bit 1→0, chỉ áp dụng khi giải phóng do dung lượng — **không** áp dụng khi segment bị nghi ngờ lỗi vật lý, xem mục 9).
- Load toàn bộ vào RAM khi mount, vì kích thước rất nhỏ.

---

## 5. Segment

### 5.1 Segment header (nội bộ)

- Vai trò: **cache suy ra được** (derived cache), không phải nguồn sự thật — vì SQLite đã được chọn làm nguồn sự thật cho việc cấp phát chunk (mục 7.3). Có thể rebuild lại từ SQLite khi cần.
- Nội dung: bitmap nội bộ (N bit, 1 bit/slot), `free_count`, `magic`/`version` riêng của segment.
- Kích thước header được làm tròn theo bội số sector (512B hoặc 4096B — alignment) để `data_start` luôn nằm trên ranh giới sector sạch:

```
header_size_thô = size(bitmap N bit + free_count + magic/version)
header_size     = round_up(header_size_thô, alignment)   // ví dụ 512
data_start      = segment_start + header_size
```

### 5.2 Vùng data (chunk slots)

- Chia thành N slot kích thước bằng nhau (`chunk_length`), đánh index từ 0.
- Phần dư (nếu `segment_length - header_size` không chia hết `chunk_length`) nằm ở **cuối** segment, bỏ trống — không ảnh hưởng tới header hay slot nào khác.

```
num_slots = floor((segment_length - header_size) / chunk_length)
```

### 5.3 Công thức tính offset

```
offset(slot_index) = data_start + slot_index * chunk_length
```

Không cần lưu offset riêng cho từng chunk — tính trực tiếp từ `slot_index`.

---

## 6. Superblock mirror (backup)

- Lưu 1 bản sao của disk header tại cuối ổ: `offset = disk_size - 4096`.
- Mục đích: nếu vùng đầu ổ (disk header + bitmap ngoài) bị hỏng, vẫn phục hồi được từ bản mirror — tương tự cơ chế backup GPT.

---

## 7. Index / metadata (SQLite)

### 7.1 Bảng `objects`

| Cột | Ý nghĩa |
|---|---|
| `object_id` | Định danh object |
| `name` | Tên object |
| `chunk_count` | Số chunk của object |
| `total_size` | Tổng kích thước |

### 7.2 Bảng `chunks`

| Cột | Ý nghĩa |
|---|---|
| `object_id` | Object sở hữu chunk |
| `chunk_index` | Thứ tự chunk trong object |
| `segment_id` | Segment chứa chunk |
| `slot_index` | Vị trí slot trong segment |
| `checksum` | Checksum của chunk (CRC32/xxhash), tính lúc ghi |
| `status` | `ok` \| `corrupt` — xem mục 9 |

### 7.3 Nguồn sự thật (source of truth)

- **SQLite là nguồn sự thật duy nhất** cho việc "slot nào đang được dùng, bởi object nào".
- Bitmap/`free_count` trong segment header chỉ là cache tăng tốc tra cứu (tránh phải `SELECT COUNT(*)` mỗi lần cần biết segment còn chỗ trống hay không), có thể rebuild lại từ SQLite nếu nghi ngờ sai lệch.
- Giải quyết được vấn đề dual source of truth (2 nơi cùng lưu trạng thái nhưng không đồng bộ khi crash).

---

## 8. Write flow & crash consistency

Nguyên tắc: **ghi data trước, fsync, rồi mới commit vào SQLite. Không bao giờ commit SQLite trước khi data ghi xong.**

```
1. Ghi chunk data vào offset(slot_index)
2. fsync
3. Insert/update row vào bảng chunks (SQLite) + commit
4. Cập nhật bitmap nội bộ + free_count của segment (cache)
```

- Nếu crash ở bước 1–2: SQLite chưa biết gì về slot đó → coi như chưa từng ghi, lần sau tái sử dụng slot — **chấp nhận drop dữ liệu dở**, đúng chủ trương đã chọn (ưu tiên đơn giản, không cần WAL riêng cho engine).
- Không được phép xảy ra chiều ngược lại: SQLite trỏ tới slot nhưng data chưa ghi xong — thứ tự ghi trên đã loại trừ trường hợp này.

---

## 9. Checksum & data integrity

### 9.1 Vị trí lưu checksum

- **Lưu ở từng chunk** (cột `checksum` trong bảng `chunks`), không chỉ ở object — vì cần biết chính xác chunk nào/segment nào hỏng để cách ly đúng vùng, không chỉ biết "object này hỏng".
- Có thể có thêm checksum tổng ở object-level như lớp kiểm tra nhanh, nhưng không thay thế checksum theo chunk.

### 9.2 Trạng thái (state machine 2 cấp)

- Cấp chunk: `status ∈ {ok, corrupt}` — set `corrupt` khi verify checksum lúc đọc phát hiện sai lệch.
- Cấp segment: `status ∈ {active, maintain}` — set `maintain` ngay khi có ≥1 chunk trong segment đó `corrupt` (khoanh vùng cả segment vì không chắc lỗi vật lý có lan sang chunk lân cận hay không).

### 9.3 Xử lý khi 1 segment chuyển sang `maintain`

1. **Chặn ghi mới** vào segment này.
2. Chunk đã `corrupt`: giữ nguyên, chấp nhận mất object chứa nó — đánh đổi để bảo vệ phần dữ liệu còn lại.
3. Chunk còn `ok` trong cùng segment: áp dụng lại cơ chế **compaction** (mục 10) để di dời sang segment khác đang `active`, cập nhật lại `segment_id`/`slot_index` trong SQLite.
4. Sau khi di dời xong, segment `maintain` bị bỏ hẳn — **không trả bit về 0 trong bitmap ngoài** (khác với compaction thông thường), tránh cấp phát lại vùng nghi ngờ lỗi vật lý.
5. Yêu cầu vận hành: báo hiệu để developer/admin backup ổ và kiểm tra vật lý nếu có thể.

### 9.4 Giới hạn đã biết

- Phát hiện lỗi là **thụ động (lazy)** — chỉ xảy ra khi có request đọc trúng chunk hỏng. Không có background scrub job ở giai đoạn này (có thể bổ sung sau nếu cần phát hiện sớm hơn).

---

## 10. Compaction / vacuum

**Lý do cần**: khi xoá một phần chunk trong segment (không xoá hết), segment không được giải phóng dù tỷ lệ sử dụng thấp — chunk trống nằm rải rác, không liên tục, không tái sử dụng được cho segment khác. Theo thời gian gây lãng phí dung lượng lớn dù bitmap ngoài báo "hết chỗ tạo segment mới".

**Điều kiện kích hoạt**:
- Segment có tỷ lệ sử dụng thấp (ví dụ < 20%).
- Segment bị đánh dấu `maintain` do checksum fail (mục 9.3).

**Thuật toán**:
1. Đọc toàn bộ chunk còn sống (`status = ok`) trong segment nguồn.
2. Ghi các chunk đó sang 1 segment khác đang `active` còn chỗ.
3. Cập nhật lại `segment_id`, `slot_index` tương ứng trong bảng `chunks` (SQLite).
4. Giải phóng segment nguồn:
   - Nếu compact do tỷ lệ sử dụng thấp (segment vẫn khỏe mạnh về vật lý) → trả bit về 0 trong bitmap ngoài, cho phép tái sử dụng.
   - Nếu compact do `maintain` (nghi ngờ lỗi vật lý) → **không** trả bit về 0, loại bỏ vĩnh viễn (xem 9.3).

---

## 11. Concurrency (hiện tại)

- Hệ thống chỉ có **1 luồng đọc/ghi duy nhất** tại một thời điểm → chưa cần lock/atomic CAS cho bitmap hay `free_count`.
- Ghi chú cho tương lai: nếu mở rộng thành multi-thread/multi-process, cần quay lại bổ sung cơ chế khoá cho thao tác cập nhật bitmap/`free_count`/SQLite để tránh race condition (double-allocate cùng 1 slot).

---

## 12. Version field

- Thêm `version` vào disk header (chi phí gần như bằng 0 do header vốn đã có khoảng trống từ alignment/padding).
- Mục đích: cho phép migrate khi tương lai đổi `chunk_length`, `segment_length`, hoặc cấu trúc on-disk mà không làm hỏng dữ liệu cũ — phân biệt được ổ định dạng cũ/mới.

---

## 13. Object nhỏ / dữ liệu quan trọng — ngoài phạm vi engine này

- Quyết định: **forward sang một ổ SSD khác** thay vì tự xây thêm luồng "small object pool" bên trong engine.
- Engine này chỉ tối ưu cho object lớn, kích thước tương đối đồng đều (chunk cố định 4MB).

---

## 14. Việc chưa làm / có thể mở rộng sau

- Background scrub job (quét chủ động toàn bộ checksum thay vì chỉ verify lúc đọc).
- Cơ chế lock/concurrency nếu chuyển sang multi-thread hoặc multi-process.
- Thuật toán chọn segment đích khi compaction (hiện chỉ định hướng "segment khác đang active còn chỗ", chưa chọn best-fit/first-fit cụ thể).