# Storage Engine — Kế hoạch thiết kế (Design Plan)

> Tài liệu tổng hợp từ quá trình thảo luận thiết kế cơ chế lưu trữ chunk/segment cho storage engine.

## 1. Mục tiêu

- Thiết kế storage engine lưu dữ liệu lớn (hàng TB, hàng triệu object, hàng tỷ chunk) trên nhiều ổ đĩa, chủ yếu là HDD.
- Tối ưu theo đúng đặc tính vật lý của HDD: ưu tiên I/O tuần tự, hạn chế tối đa random seek.
- Metadata tập trung trên SSD để tra cứu nhanh; không bao giờ cần scan hay `read_dir()` trên HDD.
- Phát triển theo hướng rủi ro thấp: bắt đầu đơn giản, sau đó nâng cấp tầng lưu trữ mà không phá vỡ tầng API/metadata phía trên.

## 2. Các khái niệm cốt lõi

| Khái niệm | Định nghĩa |
|---|---|
| **Object** | Đơn vị dữ liệu ở tầng ứng dụng — file upload, blob database, bản backup, dataset AI, video... Storage engine không cần quan tâm đó là "file" hay gì khác. |
| **Chunk** | Đơn vị logic — một phần dữ liệu cắt nhỏ từ Object (ví dụ file 100MB → 25 chunk × 4MB). Là đơn vị mà metadata quản lý. |
| **Segment** | Đơn vị lưu trữ vật lý — một file dữ liệu lớn trên đĩa (`segmentXXXXXX.dat`), chứa nhiều chunk từ nhiều Object khác nhau, không có ranh giới nào ngoài offset. |
| **Disk** | Một ổ vật lý (HDD1, HDD2...), có định danh riêng qua `disk.json`. |

**Quan hệ:** Object → nhiều Chunk → mỗi Chunk nằm trong 1 Segment tại 1 cặp (Offset, Length) cụ thể.

**Nguyên tắc xuyên suốt:** tầng lưu trữ (disk/segment/chunk) không biết và không cần biết khái niệm "file" — đó là khái niệm của tầng ứng dụng. Nhờ vậy, cùng một storage engine có thể phục vụ nhiều loại dữ liệu khác nhau (file upload, blob database, backup, dataset AI, video...) mà không cần đổi lõi lưu trữ.

## 3. Lộ trình theo giai đoạn

### Giai đoạn 1 — Chunk-per-file (bắt đầu đơn giản)

```
HDD1
├── disk.json        # định danh ổ, cấu hình (UUID, read-only...)
├── chunks/          # dữ liệu đã commit
├── temp/            # dữ liệu đang ghi, chưa commit
├── lost+found/       # dữ liệu recovery chưa xác định thuộc object nào
└── trash/            # chờ xóa (hỗ trợ xóa bất đồng bộ)
```

- Mỗi chunk = 1 file, dùng **hash directory 2 cấp** để tránh quá nhiều entry trong một thư mục (cách Git, Docker, OCI Registry đều dùng):

```
chunks/
  ab/
    cd/
      <chunk_id>
```

- Không đặt tên file có ý nghĩa (không dùng `movie_chunk_001`) — chỉ dùng ID thuần, vì tên đầy đủ đã được index trong metadata DB trên SSD rồi. Ổ đĩa không cần biết chunk thuộc object nào.
- `disk.json` cho phép đổi cấu hình ổ (ví dụ chuyển sang read-only) chỉ bằng cách sửa 1 file, không cần sửa Docker/code rồi build lại.
- Lý do bắt đầu bằng giai đoạn này: dễ debug, dễ kiểm tra bằng mắt, recovery đơn giản; chưa cần garbage collection / allocator / compaction.

### Giai đoạn 2 — Segment (khi hệ thống đã ổn định)

```
HDD1
├── disk.json
├── segment000001.dat
├── segment000002.dat
└── segment000003.dat
```

- Điều kiện chuyển: WAL, metadata, scheduler, recovery ở Giai đoạn 1 đã chạy ổn định.
- Mỗi ổ chỉ có vài file `segmentXXXXXX.dat` lớn, chứa nhiều chunk đóng gói liên tiếp.
- Vì mọi truy xuất đều đi qua `ChunkID → Metadata → vị trí vật lý` (không phụ thuộc tên file thật), việc đổi tầng lưu trữ từ chunk-file sang segment **không làm thay đổi API hay metadata schema** ở tầng trên — miễn abstraction được thiết kế đúng ngay từ Giai đoạn 1.
- Cơ chế ghi cụ thể cho segment (tần suất fsync, cách xử lý crash giữa chừng...) cần thiết kế chi tiết riêng — xem mục "Việc cần làm tiếp theo".

## 4. Thiết kế Segment

```
segment0001.dat
+------------------------------------------------------+
| A1 | B1 | A2 | A3 | B2 | C1 | D5 | ...                |
+------------------------------------------------------+
```
(A1, B1, A2... là các chunk thuộc những Object A, B... khác nhau, ghi nối tiếp nhau)

- **Kích thước segment đề xuất: 256MB – 1GB.** Không để quá lớn (hàng chục/trăm GB): segment càng lớn thì recovery càng chậm, compaction càng chậm, backup càng khó, verify checksum càng lâu.
- Ghi theo kiểu **append-only**, không sửa dữ liệu ở giữa file.
- Đọc một chunk: `open(segment) → seek(offset) → read(length)`.

## 5. Metadata Schema (trên SSD)

| Trường | Mô tả |
|---|---|
| ChunkID | Định danh chunk |
| ObjectID / FileID | Chunk thuộc Object nào |
| DiskID | Ổ vật lý chứa chunk |
| SegmentID | File segment chứa chunk (Giai đoạn 2) |
| Offset | Vị trí byte bắt đầu trong segment |
| Length | Độ dài dữ liệu |
| Checksum/CRC | Kiểm tra toàn vẹn dữ liệu của riêng chunk đó |

Ví dụ dữ liệu:

| ChunkID | ObjectID | Disk | Segment | Offset | Length |
|---|---|---|---|---|---|
| 1001 | A | HDD1 | seg001 | 0 | 4MB |
| 1002 | B | HDD1 | seg001 | 4MB | 4MB |
| 1003 | A | HDD1 | seg001 | 8MB | 4MB |

Mọi lượt đọc đều tra metadata DB trên SSD rồi nhảy thẳng đến đúng vị trí trên HDD — không bao giờ cần scan thư mục hay `read_dir()`.

## 6. Write Path (Giai đoạn 1 — chunk-per-file)

1. Ghi dữ liệu mới vào `temp/` (ví dụ `temp/tx000123.chunk`), **không** ghi thẳng vào `chunks/`.
2. `fsync()` để đảm bảo dữ liệu đã thực sự xuống đĩa.
3. Tính checksum, verify.
4. `rename()` từ `temp/` sang `chunks/`.
   - Trên Linux, `rename()` trong cùng filesystem gần như là thao tác atomic → tránh trạng thái dữ liệu dở dang khi có crash giữa chừng.

## 7. Read Path

1. Tra `ObjectID` → danh sách `ChunkID` trong metadata.
2. Với mỗi `ChunkID` → lấy `DiskID`, `SegmentID` (hoặc đường dẫn chunk), `Offset`, `Length`.
3. Mở file tương ứng, `seek()` tới offset, đọc đủ `length`.
4. (Tuỳ policy) verify checksum sau khi đọc.

## 8. WAL (Write-Ahead Log)

- Đặt trên SSD, cùng cấp với `metadata.db` và `allocator`.
- Ghi log thay đổi metadata trước khi coi là commit, phục vụ khôi phục khi hệ thống crash.

## 9. Vấn đề cần chốt: ChunkID — Auto-increment hay UUID?

| Phương án | Ưu điểm | Nhược điểm |
|---|---|---|
| Auto-increment (int64) | Gọn (8 byte), index B-Tree hiệu quả, thân thiện ghi tuần tự | Cần bộ sinh ID tập trung → có thể thành điểm nghẽn nếu scale nhiều node ghi song song |
| UUID v4 | Sinh phân tán, không cần điều phối | Dài (16 byte / 36 ký tự), ngẫu nhiên hoàn toàn → dễ gây phân mảnh index |
| UUID v7 / ULID | Vẫn phân tán, nhưng sắp theo thời gian → thân thiện index hơn UUID v4 | Vẫn dài hơn int64 |

**Đề xuất:** ở tầng đĩa, chunk không còn cần tên file có ý nghĩa hay ID dài (mọi thứ định vị qua offset trong metadata), nên độ dài ID không còn là vấn đề lớn như lo ngại ban đầu. Ưu tiên **auto-increment int64 làm ChunkID chính trong metadata DB** để tối ưu index; chỉ cân nhắc UUID (ưu tiên v7/ULID) nếu sau này cần nhiều node sinh ID độc lập, không qua điều phối trung tâm.

## 10. Garbage Collection / Compaction

Khi xoá 1 chunk nằm giữa segment, segment để lại "lỗ trống" chứ không tự co lại. Cần tiến trình Compaction định kỳ:

1. Quét các chunk còn sống (chưa xoá) trong segment cũ.
2. Copy các chunk đó sang segment mới.
3. Cập nhật metadata trỏ sang vị trí mới.
4. Xoá segment cũ.

## 11. Độ tin cậy & khôi phục

- Mỗi chunk lưu checksum/CRC riêng trong metadata — không chỉ dựa vào checksum của cả segment — để giới hạn thiệt hại khi một segment (có thể hàng trăm MB – 1GB) bị hỏng.
- `lost+found/`: chứa dữ liệu recovery được nhưng chưa xác định thuộc Object nào.
- `trash/`: hỗ trợ xoá bất đồng bộ, có thể khôi phục trong thời gian ân hạn trước khi xoá thật.

## 12. Vì sao mô hình Segment phù hợp với HDD

- HDD: sequential read ~220MB/s nhưng random read chỉ ~2MB/s — chênh lệch rất lớn.
- Chunk-per-file: mỗi lần đọc phải `open() → seek inode → read → close()`, lặp lại hàng triệu lần → nhiều random I/O.
- Segment: chỉ `open()` một lần, sau đó `seek + read` liên tục theo offset → tận dụng tối đa băng thông tuần tự, giảm seek.
- Linux Page Cache: đọc gần các offset liền kề thường đã được cache sẵn từ lần đọc trước.
- Nhiều hệ thống thực tế dùng chung hướng tiếp cận này: Kafka (Segment Log), LevelDB/RocksDB (SSTable), Bitcask (Data File), HDFS (Block), Ceph BlueStore (Blob/Extent).

## 13. Kiến trúc tổng thể mục tiêu

```
SSD
├── metadata.db     # ChunkID → DiskID → SegmentID → Offset → Length → Checksum
├── wal/
└── allocator

HDD1
├── disk.json
├── segment000001.dat
├── segment000002.dat
└── segment000003.dat

HDD2
├── disk.json
├── segment000001.dat
└── segment000002.dat
```

## 14. Việc cần làm tiếp theo

- [ ] Chốt phương án ChunkID (mục 9)
- [ ] Định nghĩa schema chi tiết cho `disk.json`
- [ ] Định nghĩa format record cho WAL
- [ ] Benchmark để chọn kích thước segment cụ thể (256MB / 512MB / 1GB)
- [ ] Thiết kế write path chi tiết cho segment (tần suất fsync, xử lý crash giữa chừng)
- [ ] Thiết kế thuật toán Compaction/GC (thời điểm trigger, ngưỡng % dữ liệu rác)
- [ ] Thiết kế cơ chế verify checksum định kỳ (scrubbing) để phát hiện bit rot sớm
- [ ] Lên kế hoạch migration cụ thể từ Giai đoạn 1 (chunk-file) sang Giai đoạn 2 (segment)
