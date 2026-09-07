# Cơ chế ghi dữ liệu trên SSD và khuyến nghị cấu trúc lưu trữ

## 1. Cấu trúc vật lý của SSD (NAND Flash)

SSD được tổ chức theo thứ bậc:

```
Die → Plane → Block → Page
```

- **Page**: đơn vị đọc/ghi nhỏ nhất, thường **4KB, 8KB hoặc 16KB**.
- **Block**: gồm nhiều Page (thường 128–512 page), là đơn vị **xóa nhỏ nhất**, thường **1–4MB**.
- Một Die có nhiều Plane, một Plane có nhiều Block.

Điểm mấu chốt cần nhớ:

> **SSD có thể ghi theo Page, nhưng chỉ có thể xóa theo Block.**

## 2. Vì sao SSD không thể "ghi đè" như HDD

Trên HDD, ghi đè 1 byte ở vị trí cũ là chuyện bình thường — đầu từ chỉ việc từ hóa lại đúng vị trí đó.

Trên SSD, một ô nhớ NAND đã có dữ liệu (đã ở trạng thái "programmed") thì **không thể ghi trực tiếp đè lên** — bắt buộc phải **xóa cả Block chứa nó rồi mới ghi lại**. Đây gọi là cơ chế **Program/Erase (P/E) cycle**, và chính giới hạn số lần P/E này quyết định tuổi thọ ổ (mình đã nói ở phần trước — SLC/MLC/TLC/QLC).

Vì vậy SSD dùng cơ chế **ghi không tại chỗ (out-of-place write)**:
- Khi bạn "sửa" một phần dữ liệu, SSD không xóa-ghi ngay ô cũ.
- Nó ghi dữ liệu mới vào một **Page trống khác**, đánh dấu Page cũ là "invalid/stale".
- Việc dọn dẹp các Page rác này được xử lý sau, gọi là **Garbage Collection**.

## 3. FTL – Flash Translation Layer

Đây là phần "não" nằm trong controller của SSD, gần như một hệ điều hành mini bên trong ổ. FTL làm nhiệm vụ:

- Ánh xạ **địa chỉ logic** (LBA – Logical Block Address, mà OS/filesystem thấy) sang **địa chỉ vật lý thực tế** trong NAND.
- Quyết định ghi dữ liệu mới vào Page vật lý nào.
- Thực hiện **Wear Leveling**: rải đều lượt ghi ra khắp các Block, tránh việc 1 vùng bị ghi/xóa quá nhiều trong khi vùng khác gần như không đụng tới.
- Thực hiện **Garbage Collection**: gom các Page còn "sống" (valid) trong 1 Block sang Block khác, rồi xóa sạch Block cũ để tái sử dụng.

> Nói cách khác: **bạn (hay OS) không hề kiểm soát được vị trí vật lý thật sự mà dữ liệu được ghi vào NAND** — dù bạn có ghi "liền mạch" ở cấp logic, SSD vẫn có thể rải dữ liệu đó khắp nơi ở cấp vật lý.

Đây là khác biệt cốt lõi so với HDD, nơi bạn (hoặc filesystem) hoàn toàn kiểm soát được vị trí vật lý của dữ liệu trên đĩa từ.

## 4. Write Amplification (khuếch đại ghi)

Đây là khái niệm quan trọng nhất cần hiểu khi thiết kế cách ghi dữ liệu lên SSD.

**Write Amplification (WA)** = tỷ lệ giữa lượng dữ liệu **thực tế được ghi vào NAND** so với lượng dữ liệu mà ứng dụng/OS **yêu cầu ghi**.

Ví dụ: bạn chỉ muốn sửa 512 byte trong 1 file, nhưng vì Page nhỏ nhất là 4KB và Block nhỏ nhất là 1MB, SSD có thể phải:
1. Đọc cả Block 1MB chứa Page đó ra bộ nhớ đệm.
2. Gộp phần dữ liệu mới (512 byte) vào đúng vị trí.
3. Ghi lại toàn bộ các Page hợp lệ (kể cả những Page không hề thay đổi) sang một Block trống mới.
4. Xóa Block cũ.

→ Bạn chỉ ghi 512 byte, nhưng NAND phải ghi lại có thể tới hàng trăm KB – vài MB. WA càng cao → hao mòn NAND càng nhanh, hiệu năng càng giảm.

**Nguyên nhân chính khiến WA tăng cao:**
- Ghi ngẫu nhiên (random write), kích thước nhỏ, không thẳng hàng (misaligned) với ranh giới Page/Block.
- Không dùng TRIM → SSD không biết vùng nào đã "xóa logic" để dọn dẹp sớm.
- Ổ gần đầy (ít Block trống để Garbage Collection làm việc hiệu quả).

## 5. TRIM – vì sao quan trọng

Khi bạn xóa 1 file ở cấp OS, thực chất filesystem chỉ đánh dấu vùng đó là "trống" trong bảng quản lý (giống HDD) — bản thân SSD **không hề biết** dữ liệu đó đã "vô nghĩa".

**TRIM** là lệnh mà OS/filesystem gửi xuống SSD để báo: "các Page ở LBA này không còn dùng nữa, có thể coi là rác, tùy ý dọn dẹp trước." Nhờ vậy:
- Garbage Collection hiệu quả hơn (không cần di chuyển dữ liệu rác).
- Write Amplification giảm.
- Hiệu năng ghi ổn định hơn theo thời gian.

**Điểm quan trọng cho câu hỏi của bạn:** TRIM chỉ hoạt động qua filesystem chuẩn (NTFS, ext4, APFS...). Nếu bạn ghi trực tiếp xuống **raw block device** (bỏ qua filesystem, tự quản lý cấu trúc byte riêng), OS **không thể gửi TRIM đúng cách** cho vùng dữ liệu bạn tự quản lý, trừ khi ứng dụng của bạn tự gọi API TRIM/UNMAP thủ công (phức tạp, ít ứng dụng làm điều này ngoài các database engine chuyên dụng).

## 6. So sánh: Custom byte-level format vs OS Filesystem trên SSD

### Trên HDD (cách bạn đang làm)
Ghi theo cấu trúc byte tự định nghĩa trên HDD là hợp lý vì:
- HDD không có khái niệm "Page/Block phải xóa nguyên khối".
- Không có Write Amplification kiểu NAND.
- Không có giới hạn số lần ghi (hao mòn HDD đến từ cơ, không phải từ ghi/xóa điện tử).
- Bạn kiểm soát hoàn toàn layout vật lý → tối ưu tốc độ seek nếu sắp xếp dữ liệu liền mạch hợp lý.

### Trên SSD — Custom raw byte format

| Ưu điểm | Nhược điểm |
|---|---|
| Có thể giảm overhead của filesystem (metadata, indexing) | **Không nhận được TRIM** trừ khi tự implement |
| Kiểm soát layout logic ở tầng ứng dụng | Vẫn **không kiểm soát được layout vật lý thật** (FTL quyết định hết) |
| Phù hợp cho database engine tối ưu cao (như InnoDB raw tablespace) | Dễ gây ghi nhỏ lẻ, lệch ranh giới Page/Block → tăng Write Amplification nếu không căn chỉnh cẩn thận |
| | Mất các lợi ích an toàn của filesystem: journaling, crash consistency, khôi phục sau mất điện |
| | Khó backup, khó debug, khó công cụ hỗ trợ (không mount được như file thường) |
| | Phức tạp hơn nhiều để làm đúng, dễ làm sai hơn là làm đúng |

### Trên SSD — Dùng file qua OS Filesystem (NTFS/ext4/...)

| Ưu điểm | Nhược điểm |
|---|---|
| TRIM hoạt động tự động, đúng chuẩn | Có overhead nhỏ từ metadata/filesystem (không đáng kể với file cỡ 1-5GB) |
| Filesystem hiện đại (NTFS, ext4, APFS, F2FS...) đã được tối ưu sẵn cho SSD (căn chỉnh block, giảm ghi thừa) | Ít quyền kiểm soát layout logic tuyệt đối |
| Có journaling/crash-consistency → an toàn hơn khi mất điện đột ngột | |
| Dễ backup, dễ quản lý, công cụ hỗ trợ đầy đủ | |
| SQLite, hầu hết database phổ biến đều được thiết kế để chạy trên file hệ thống chuẩn | |

## 7. Khuyến nghị cho trường hợp của bạn

Với workload bạn mô tả — file 1–5GB, tần suất ghi 1–2 tuần/lần, có cơ chế dọn dẹp theo lịch thay vì xóa ngay — thì:

**Nên dùng file trên OS filesystem chuẩn, không nên tự làm cấu trúc byte-level riêng trên SSD.**

Lý do cụ thể:
1. **TRIM là lợi ích lớn nhất bạn sẽ mất** nếu dùng raw format — mà với SSD, TRIM ảnh hưởng trực tiếp đến hiệu năng ổn định lâu dài.
2. Với dung lượng và tần suất ghi thấp như vậy, **overhead của filesystem là không đáng kể** — bạn sẽ không tiết kiệm được gì đáng kể về hiệu năng hay hao mòn khi tự làm raw format.
3. Filesystem hiện đại đã tự động căn chỉnh ghi theo ranh giới 4KB (page size phổ biến) — bạn không cần tự lo việc này.
4. SQLite vốn được thiết kế và kiểm thử kỹ để chạy tối ưu trên file hệ thống chuẩn (kể cả cơ chế WAL, rollback journal đều dựa trên giả định file API chuẩn của OS). Việc đặt SQLite lên raw block device không mang lại lợi ích tương xứng với độ phức tạp và rủi ro tăng thêm.
5. An toàn dữ liệu (crash consistency) khi mất điện — filesystem + WAL mode của SQLite đã được kiểm chứng rất kỹ, tự làm raw format bạn phải tự đảm bảo toàn bộ phần này.

**Khi nào raw byte-level format trên SSD mới thực sự đáng cân nhắc:**
- Hệ thống ghi liên tục khối lượng cực lớn (hàng trăm GB–TB/ngày), độ trễ (latency) là yếu tố sống còn (ví dụ: OLTP database enterprise, time-series database tần suất cao).
- Đội ngũ có đủ năng lực tự implement wear-aware allocation, TRIM/UNMAP thủ công, và cơ chế crash-recovery riêng.
- Đây là lý do vì sao chỉ có số ít hệ thống lớn (một số database engine cấp doanh nghiệp) chọn raw block device — và ngay cả họ cũng cần đội ngũ chuyên biệt để làm đúng.

Với quy mô và tần suất ghi bạn mô tả, việc này không cần thiết và rủi ro (mất TRIM, mất crash-safety) lớn hơn lợi ích nhận được.

## 8. Nếu vẫn muốn thử nghiệm custom format sau này

Một vài nguyên tắc nếu bạn muốn tối ưu thêm ở tầng ứng dụng dù vẫn dùng file (không cần raw device):

- Căn chỉnh kích thước ghi theo bội số của **4KB** (page size phổ biến nhất hiện nay).
- Ưu tiên **ghi tuần tự (sequential append)** thay vì ghi ngẫu nhiên rải rác trong file.
- Gộp nhiều thay đổi nhỏ thành 1 lần ghi lớn hơn thay vì ghi nhiều lần nhỏ liên tiếp (giảm số lần fsync).
- Với SQLite: dùng **WAL mode** để có write pattern tuần tự hơn, giảm write amplification tự nhiên.