# Project Rules & Guidelines (`my_word`)

## Tự động rút kinh nghiệm khi code sai (code-mistake-lessons)
Quy tắc tự động kích hoạt bắt buộc:
1. **Trước khi code**: Đọc file [LESSONS.md](LESSONS.md) và áp dụng các quy tắc liên quan đến phần sắp sửa đổi.
2. **Tự động kích hoạt khi có sai sót**: Không chờ user nhắc nhở, tự động kích hoạt quy trình khi:
   - Build/Test/Lint bị fail sau khi chỉnh sửa code.
   - User chỉnh sửa, phản đối, hoặc yêu cầu sửa lại code vừa viết.
   - Phải revert hoặc sửa lại lần thứ hai cho cùng một việc.
3. **Quy trình xử lý**:
   - Sửa lỗi trước để không làm nghẽn công việc của user.
   - Phân tích nguyên nhân gốc (5 Whys), tránh chỉ sửa triệu chứng.
   - Tự động ghi/cập nhật bài học rút ra vào [LESSONS.md](LESSONS.md) theo định dạng:
     `- [xN] Khi <tình huống> → <hành động> → kiểm tra: <cách kiểm tra>`
   - Báo cáo ngắn gọn 1-3 dòng về nguyên nhân và bài học đã lưu.

## Quy ước nhánh GitHub (`dev` & `test` tách riêng)
- **Chỉ sử dụng 2 nhánh tách biệt**: `dev` (nhánh chính thức) và `test` (nhánh thử nghiệm); không dùng hay tạo lại nhánh `main`.
- **Quy trình bắt buộc**: Mọi thay đổi mới, sửa lỗi hoặc thử nghiệm phải thực hiện và kiểm thử trên nhánh `test` trước (`cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, và kiểm tra UI nếu sửa giao diện). Chỉ sau khi vượt qua kiểm thử mới gộp (`merge`/`fast-forward`) từ `test` sang nhánh chính thức `dev`.
- **CI tách riêng**: [.github/workflows/test.yml](.github/workflows/test.yml) chạy cho `test`; [.github/workflows/dev.yml](.github/workflows/dev.yml) chạy kiểm tra + build `release` cho `dev`.

