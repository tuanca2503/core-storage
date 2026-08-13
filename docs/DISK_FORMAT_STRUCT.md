# Sơ Đồ Cấu Trúc On-Disk Format — `quick_format`

## 1. Tổng quan layout

Toàn bộ block device được chia thành: **Superblock chính**, các **Segment** dữ liệu, và **Superblock mirror** (dự phòng cuối thiết bị).

```mermaid
flowchart LR
  SB1["<b>Superblock Primary</b><br/>4KB<br/>offset 0"]
  SEG1["<b>Segment 1</b><br/>Header 4KB + Data"]
  SEG2["<b>Segment 2</b><br/>Header 4KB + Data"]
  DOTS(["⋮"])
  SEGN["<b>Segment N</b><br/>(partial)<br/>Header 4KB + Data part"]
  SB2["<b>Superblock Mirror</b><br/>4KB<br/>offset = size - 4KB"]

  SB1 --> SEG1 --> SEG2 --> DOTS --> SEGN --> SB2

```

- **Superblock (Primary)**: offset `0`, kích thước cố định `4KB`.
- **Superblock (Mirror)**: offset `size_of_device - 4KB`, bản sao dự phòng để phục hồi khi superblock chính hỏng.
- **Segment**: đơn vị cấp phát lớn, mỗi segment có **Header riêng (4KB)** + vùng **Data**.
- Segment cuối cùng có thể không đầy (`partial segment`) nếu dung lượng thiết bị không chia hết.

---

## 2. Cấu trúc Superblock (4KB)

```mermaid
packet-beta
title Superblock — 4KB (offset tính theo byte)
0-11: "magic (12B)"
12-27: "uuid (16B)"

28-31: "version (4B)"
32-35: "state (4B)"

36-43: "logical_sector_size (4B)"
44-51: "physical_sector_size (4B)"

52-59: "capacity_bytes (8B)"
60-67: "segment_count (8B)"
68-75: "active_segment_index (8B)"
76-83: "last_segment_size_bytes (8B)"
84-91: "mirror_offset (8B)"
92-99: "created_at_ms (8B)"
100-127: "reserved / padding"
```

> Tổng các field = 92 byte; phần `reserved / padding` trên sơ đồ chỉ vẽ tượng trưng, thực tế đệm liên tục từ offset 92 đến hết 4095 (4KB).

| Field | Kiểu Rust | Size | Mô tả |
|---|---|---|---|
| magic | `[u8; 12]` | 12 bytes | Magic number nhận diện định dạng |
| uuid | `[u8; 16]` | 16 bytes | Định danh duy nhất của volume |
| version | `u32` | 4 bytes | Version format |
| state | `StorageState` | 4 bytes | Trạng thái storage (enum, repr u32) |
| logical_sector_size | `u32` | 4 bytes | Kích thước logical sector |
| physical_sector_size | `u32` | 4 bytes | Kích thước physical sector |
| capacity_bytes | `u64` | 8 bytes | Tổng dung lượng thiết bị (bytes) |
| last_segment_size_bytes | `u64` | 8 bytes | Kích thước segment cuối (có thể partial) |
| segment_count | `u64` | 8 bytes | Số lượng segment |
| active_segment_index | `u64` | 8 bytes | Index của segment đang active |
| mirror_offset | `u64` | 8 bytes | Offset của superblock mirror |
| created_at_ms | `u64` | 8 bytes | Timestamp tạo (millisecond epoch) |
| reserved | — | padding | Đệm cho đủ 4KB |

> Superblock Primary và Mirror phải **đồng nhất nội dung** (trừ khi đang trong quá trình ghi transaction); khi mount, hệ thống so sánh `magic` + `version` của cả hai để chọn bản hợp lệ.

---

## 3. Cấu trúc Segment
 
- Mỗi **segment = 64GiB**.
- Trong segment gồm: **Header 4KiB** + vùng **Data** chứa các **chunk**, mỗi chunk **32MiB**.
- Số chunk trong 1 segment (phần Data) = `(64GiB - 4KiB) / 32MiB` ≈ **2047 chunk** (còn dư một phần nhỏ **32MiB - 4KiB**, chấp nhận bỏ vì không đủ 1 chunk).
 
```mermaid
flowchart LR
  H["Header<br/>4KiB"] --> C1["Chunk 1<br/>32MiB"] --> C2["Chunk 2<br/>32MiB"] --> Dots(["⋮"]) --> C2047["Chunk 2047<br/>32MiB"] --> Waste["Phần dư<br/>32MiB - 4KiB<br/>(chấp nhận bỏ)"]
```

### 3.1 Segment Header (4KiB)
 
```mermaid
packet-beta
title Segment Header — 4KiB (offset tính theo byte)
0-7: "chunk_count (8B)"
8-15: "chunk_capacity (8B)"
16-27: "reserved / padding"
```
 
> Tổng field = 16 byte; phần `reserved / padding` chỉ vẽ tượng trưng, thực tế đệm liên tục tới hết offset 4095 (4KiB).
 
| Field | Kiểu Rust | Size | Mô tả |
|---|---|---|---|
| chunk_count | `u64` | 8 bytes | Số lượng chunk trong segment, đồng thời là index cho lần ghi kế tiếp |
| chunk_capacity | `u64` | 8 bytes | Sức chứa (kích thước) của mỗi chunk |
| reserved | — | padding | Đệm cho đủ 4KiB |

---

## 4. Sơ đồ tổng hợp (chi tiết, dạng phân cấp)

```mermaid
flowchart TB
  subgraph Device["Block Device — offset 0 → capacity_bytes"]
    SBP["Superblock PRIMARY — 4KiB\nmagic, uuid, version, state,\nlogical/physical_sector_size, capacity_bytes,\nlast_segment_size_bytes, segment_count,\nactive_segment_index, mirror_offset, created_at_ms"]
    subgraph Seg0["Segment 0 — 64GiB"]
      direction TB
      H0["Segment Header — 4KiB\nchunk_count, chunk_capacity"]
      D0["Data: Chunk1 | Chunk2 | ... | Chunk2047 | phần dư bỏ\n(64GiB - 4KiB, chunk 32MiB)"]
      H0 --> D0
    end
    DotsSeg["⋮ (Segment 1 .. N-1, cùng cấu trúc)"]
    subgraph SegN["Segment N (cuối, last_segment_size_bytes, có thể partial)"]
      direction TB
      HN["Segment Header — 4KiB"]
      DN["Data (có thể ngắn hơn nếu không chia hết)"]
      HN --> DN
    end
    SBM["Superblock MIRROR — 4KiB\nbản sao dự phòng, offset = mirror_offset"]
 
    SBP --> Seg0 --> DotsSeg --> SegN --> SBM
  end
```

---

## 5. Ghi chú thiết kế

- **Fixed-point layout**: `segment_count`, offset từng segment và vị trí mirror superblock được tính cố định 1 lần khi format, không cần bảng chỉ mục động → tra cứu offset bằng công thức `O(1)`.
- **CRC32C**: áp dụng cho cả Superblock và Segment Header để phát hiện corruption sớm khi mount.
- **Dual superblock**: nếu Primary hỏng (checksum sai), fallback đọc Mirror ở cuối device để phục hồi.
- **flock**: tiến trình mount cần giữ exclusive lock (`flock`) trên block device để tránh 2 tiến trình ghi đồng thời.
- **Partial segment cuối**: nếu `device_size` không chia hết cho `segment_size`, segment cuối vẫn có Header đầy đủ 4KB nhưng vùng Data bị cắt ngắn theo dung lượng còn lại.

---

*File này mô tả layout tổng quát dựa trên thiết kế hiện tại của `quick_format`. Điều chỉnh lại field/size cho khớp với struct thực tế trong code Rust nếu có sai lệch.*