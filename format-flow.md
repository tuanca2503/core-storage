# Quick Format Flow

## 1. Kiểm tra thiết bị

- Mở Physical Disk với quyền Read/Write.
- Đọc thông tin thiết bị:
  - Capacity
  - Logical Sector Size
  - Physical Sector Size
- Kiểm tra:
  - Không phải system disk.
  - Không bị lock bởi tiến trình khác.
  - Capacity đủ lớn.

---

## 2. Chuẩn bị

- Lock Volume.
- Dismount tất cả Volume trên disk.
- Flush cache.
- (Không xóa dữ liệu cũ.)

---

## 3. Tính toán Layout

Tính:

- `total_bytes`
- `segment_size_bytes`
- `segment_count`
- `bitmap_offset`
- `bitmap_size_bytes`
- `segment_region_offset`

Ví dụ:

```
+-------------------------------+
| Header (4 KiB)                |
+-------------------------------+
| Bitmap                        |
+-------------------------------+
| Segment 0                     |
+-------------------------------+
| Segment 1                     |
+-------------------------------+
| ...                           |
+-------------------------------+
```

---

## 4. Tạo Header

Sinh:

- magic
- version
- uuid
- created_at_ms
- state = Active
- total_bytes
- segment_size_bytes
- segment_count
- bitmap_offset
- bitmap_size_bytes
- segment_region_offset

Tính CRC32C.

---

## 5. Ghi Header

Ghi Header vào offset:

```
0
```

Flush.

---

## 6. Khởi tạo Bitmap

Ghi toàn bộ Bitmap = 0.

Ví dụ:

```
000000000000000000000000
```

(0 = Chunk trống)

Flush.

---

## 7. Khởi tạo Segment Header

Lặp:

```
for segment in segments
```

Mỗi Segment:

- tạo Segment Header
- tính CRC
- ghi vào đầu Segment

Không cần ghi toàn bộ Segment.

Flush.

---

## 8. Đồng bộ

- Flush Device.
- Unlock Volume.

---

## Hoàn thành

Storage chuyển sang:

```
State = Active
```

Format hoàn tất.