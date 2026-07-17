# Chunk Recovery Plan

## Mục tiêu

Khi một chunk (4 MB) có checksum không khớp, hệ thống cố gắng phục hồi
dữ liệu trước khi kết luận chunk bị hỏng.

## Quy trình

1.  Đọc chunk.
2.  Tính checksum.
3.  Nếu checksum đúng:
    -   Trả dữ liệu.
4.  Nếu checksum sai:
    -   Retry đọc 5 lần.
    -   Nếu có lần checksum đúng:
        -   Sao chép chunk sang vị trí an toàn.
        -   Đánh dấu ổ cần bảo trì.
5.  Nếu vẫn thất bại:
    -   Retry sâu (ví dụ 100 lần hoặc nhiều hơn).
6.  Nếu dữ liệu giữa các lần đọc khác nhau:
    -   So sánh từng byte.
    -   Xác định vùng dao động.
    -   Thử majority voting để tạo candidate chunk.
    -   Kiểm tra checksum.
    -   Nếu checksum đúng thì migrate ngay.
7.  Nếu mọi lần đọc đều giống nhau nhưng checksum vẫn sai:
    -   Không đủ thông tin để khôi phục.
8.  Nếu firmware trả UNC:
    -   Đánh dấu chunk là Corrupted.

## Disk Maintenance

-   Chuyển ổ sang trạng thái Maintenance.
-   Không ghi dữ liệu mới lên ổ.
-   Vẫn cho phép đọc.
-   Backup toàn bộ dữ liệu còn đọc được.
-   Kiểm tra SMART và surface scan.
-   Migrate dữ liệu sang ổ mới.
-   Ghi log và cảnh báo cho developer.

## Ghi chú

-   Checksum chỉ dùng để phát hiện lỗi.
-   Đọc lặp nhiều lần có thể giúp lấy lại dữ liệu nếu lỗi đọc không ổn
    định.
-   Không có bảo đảm phục hồi nếu dữ liệu đã hỏng ổn định hoặc firmware
    không thể đọc sector.
