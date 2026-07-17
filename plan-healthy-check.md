# Disk Health Check Plan

## Mục tiêu

Theo dõi sức khỏe ổ đĩa với chi phí thấp, không cần đọc toàn bộ dữ liệu
thường xuyên.

## Tầng 1 - Firmware Health

Thu thập thông tin từ firmware (SMART/NVMe Health):

-   Temperature
-   Power On Hours
-   Reallocated Sector Count
-   Current Pending Sector
-   Offline Uncorrectable
-   UDMA CRC Error (HDD)
-   Percentage Used (SSD)
-   Available Spare (SSD)
-   Media Errors
-   SMART Passed

Trait đề xuất:

``` rust
pub trait PhysicalDisk {
    fn smart(&self) -> BaseResult<SmartInfo>;
    fn health(&self) -> BaseResult<DiskHealth>;
    fn statistics(&self) -> BaseResult<DiskStatistics>;
    fn self_test(&self, mode: SelfTestMode) -> BaseResult<TestResult>;
}
```

## Tầng 2 - Storage Statistics

Core Storage tự thu thập:

-   Read latency
-   Write latency
-   Checksum failures
-   Retry count
-   Total reads/writes
-   Last error

Mỗi lần đọc hoặc ghi đều cập nhật thống kê.

## Tầng 3 - Chunk Checksum

Thực hiện khi:

-   Client đọc dữ liệu.
-   Scrubber chạy định kỳ.
-   Migrate dữ liệu.
-   Recovery.

Không cần quét toàn bộ liên tục.

## Self Test

Định kỳ yêu cầu firmware chạy:

-   SMART Short Test.
-   SMART Extended Test.

## Disk Health Score

Đề xuất trọng số:

-   SMART: 30%
-   Chunk checksum: 30%
-   Read latency: 20%
-   Retry/Error: 10%
-   Temperature: 10%

Các trạng thái:

-   Healthy
-   Warning
-   Maintenance
-   Degraded
-   Replace Required

## Chính sách

-   Firmware báo lỗi nghiêm trọng hoặc checksum fail =\> Maintenance.
-   Không cấp phát dữ liệu mới.
-   Vẫn cho phép đọc để backup và migrate.
-   Sau khi migrate hoàn tất =\> Retired.

## Ghi chú

Không phụ thuộc hoàn toàn vào SMART.

Sức khỏe ổ được đánh giá từ ba nguồn:

1.  Firmware (SMART/NVMe Health)
2.  Storage Engine Statistics
3.  Chunk Checksum / Recovery
