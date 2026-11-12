---
name: "code-knowledge-keeper"
description: "Tự cập nhật và phát triển tài liệu sống về những điều quan trọng trong code (kiến trúc, quy ước, bẫy, lệnh, quyết định). Dùng khi làm việc trong một codebase, khám phá điều không hiển nhiên, user giải thích quy ước, hoặc cuối một task lớn."
---

# Code Knowledge Keeper

Giữ một file kiến thức ngắn, đúng, có nguồn về codebase, để lần làm sau không phải khám phá lại từ đầu. Khác với `LESSONS.md` (ghi **sai lầm** và quy tắc tránh), file này ghi **điều cần biết về code**. Áp dụng cho mọi ngôn ngữ.

Trả lời bằng ngôn ngữ user đang dùng. Ngắn gọn, thẳng, không giải thích dài dòng.

## Giới hạn trung thực

Claude không tự nhớ giữa các phiên và skill không tự sửa chính nó. "Tự cập nhật, tự phát triển" ở đây hoạt động nhờ file `KNOWLEDGE.md` của dự án, được ghi trong lúc làm việc và đọc lại ở lần sau. Không có file thì không có trí nhớ. Skill chỉ **đề xuất** sửa chính nó, user duyệt mới lưu. Nói rõ điều này khi cần.

## Nguyên tắc cốt lõi

**Code là sự thật, ghi chú là giả thuyết.** Mọi mục phải kiểm chứng được bằng code hoặc lệnh chạy, có nguồn, và bị xóa/sửa ngay khi code đổi hoặc mâu thuẫn.

## Cái gì đáng ghi

Chỉ ghi thứ **không hiển nhiên** và **tiết kiệm được công khám phá** cho người/phiên sau:

- **Kiến trúc & ranh giới**: module nào làm gì, phụ thuộc theo hướng nào, luồng dữ liệu chính.
- **Quy ước bất thành văn**: cách đặt tên, cấu trúc, cách xử lý lỗi, cách dùng localization, pattern được dùng nhất quán mà không được viết đâu.
- **Lệnh thật**: build, test (cả chạy một test), lint, migrate, chạy local, kèm điều kiện cần có.
- **Bẫy & gọi ý phản trực giác**: chỗ dễ hiểu nhầm, thứ tự phải đúng, side effect ẩn, hành vi khác với tên gọi.
- **Invariant & luật nghiệp vụ**: điều luôn phải đúng, ràng buộc dữ liệu, quy tắc tính toán.
- **Quyết định thiết kế + lý do**: vì sao chọn cách này, đã loại phương án nào.
- **Phụ thuộc ngoài & cấu hình**: dịch vụ, biến môi trường (chỉ tên biến, không ghi giá trị), phiên bản gây hành vi đặc biệt.
- **Vùng nguy hiểm**: code dễ vỡ, ít test, đã biết là nợ kỹ thuật.
- **Từ điển miền nghiệp vụ**: thuật ngữ của hệ thống (kể cả tiếng Nhật/Việt) và ý nghĩa.

## Cái gì không ghi

- Thứ đọc code là thấy ngay, hoặc đã có trong README/docs.
- Thứ hay đổi: số dòng, tên biến, danh sách file, tóm tắt từng file.
- Suy đoán chưa kiểm chứng.
- **Bí mật, token, mật khẩu, dữ liệu khách hàng, thông tin cá nhân.**
- Nhật ký công việc ("hôm nay sửa X"). Đó là việc của git log.
- Sai lầm của bản thân (viết vào `LESSONS.md`).

## Khi nào tự kích hoạt

- Khám phá điều không hiển nhiên mất công tìm (nhiều lần đọc/grep mới ra).
- User giải thích quy ước, lý do, hoặc quyết định.
- Thêm/đổi kiến trúc, dependency, lệnh, cấu hình quan trọng.
- Phát hiện tài liệu hoặc mục ghi chú cũ bị sai so với code.
- Cuối task lớn: quét một lần xem có gì đáng ghi.

Không chặn việc user đang cần. Làm xong việc trước, ghi sau, báo một dòng.

## Quy trình

1. **Đọc trước**: nếu có `KNOWLEDGE.md` (hoặc mục kiến thức trong CLAUDE.md), đọc phần liên quan. Trước khi dựa vào một mục, kiểm tra nhanh nguồn của nó còn đúng không.
2. **Trong lúc làm**: ghi nhớ ứng viên đáng ghi, chưa ghi vội.
3. **Cuối task, lọc ứng viên**:
   - **Kiểm chứng**: xác nhận bằng code hoặc chạy lệnh thật. Không kiểm chứng được thì bỏ hoặc đánh dấu "chưa kiểm chứng".
   - **Phản biện** (xem bên dưới).
4. **Ghi**: gộp vào mục có sẵn, không tạo dòng trùng.
5. **Dọn**: xóa/sửa mục sai hoặc lỗi thời, kiểm tra đường dẫn nguồn còn tồn tại.

## Phản biện trước khi ghi

- **Đáng ghi không?** Người mới đọc code 10 phút có tự biết không? Nếu có thì bỏ.
- **Có đúng không?** Đã kiểm chứng bằng code/lệnh chưa, hay chỉ đoán từ một đoạn?
- **Sẽ cũ nhanh không?** Thứ hay đổi thì đừng ghi, hoặc chỉ ghi nguyên tắc bất biến đằng sau nó.
- **Đã có ở nơi khác chưa?** README, docs, comment, CLAUDE.md: nếu có thì trỏ tới, đừng chép.
- **Có lộ thông tin nhạy cảm không?**
- **Có nên thuộc file này không?** Sai lầm của mình → LESSONS.md. Quyết định đội ngũ → docs/ADR.

Mỗi ứng viên kết luận ✅ **Ghi**, ⬇️ **Sửa lại/thu hẹp**, ❌ **Bỏ**. Phản biện phải thực sự loại bớt, không phản biện hình thức.

## Định dạng KNOWLEDGE.md

Chỉ tạo mục khi thật sự có nội dung:

```
# KNOWLEDGE

## Kiến trúc & ranh giới
## Quy ước
## Lệnh
## Bẫy & lưu ý
## Invariant & luật nghiệp vụ
## Quyết định thiết kế
## Phụ thuộc ngoài & cấu hình
## Vùng nguy hiểm
## Từ điển
## Đề xuất cải tiến skill
```

Mỗi mục một dòng:
```
- <điều cần biết> — vì: <lý do> (nguồn: path:dòng; xác minh: YYYY-MM-DD)
```
Ngày xác minh lấy từ ngày thật của hệ thống (chạy `date`), không đoán.

**Giữ gọn**: tối đa khoảng 150 dòng. Vượt thì tách theo module (`docs/knowledge/<module>.md`) và để file chính chỉ là mục lục. Gộp trùng, bỏ mục ít giá trị nhất trước.

## Quyền ghi

Lần đầu trong một dự án, hỏi user một câu ("Ghi phần này vào KNOWLEDGE.md nhé?"). Đồng ý hoặc đã bảo cứ ghi thì những lần sau tự ghi và báo một dòng đã ghi gì. Từ chối thì không ghi. Để mọi phiên tự đọc, gợi ý thêm `@KNOWLEDGE.md` vào CLAUDE.md; không tự sửa CLAUDE.md khi user chưa đồng ý. Thông tin về codebase thuộc file dự án, không đưa vào bộ nhớ cá nhân của user.

## Tự phát triển (có kiểm soát)

Mỗi lần dùng, tự hỏi: **skill này có thiếu loại thông tin quan trọng nào không? Có mục nào luôn bị bỏ trống hay luôn bị user sửa?**

- Ghi nhận vào mục `Đề xuất cải tiến skill` trong KNOWLEDGE.md, mỗi dòng một ý, kèm số lần gặp `[x1]`.
- Khi một ý đạt `x3`, lặp ở nhiều dự án, hoặc user yêu cầu: **đề xuất cập nhật skill** bằng propose_skills, gửi toàn bộ SKILL.md mới. User duyệt mới lưu.
- Không bao giờ tự sửa skill âm thầm. Không thêm loại thông tin chỉ vì "có thể hữu ích".
- Nếu một mục skill chưa bao giờ được dùng qua nhiều dự án, đề xuất bỏ nó.

## Khi user hỏi về code

Trả lời từ KNOWLEDGE.md nếu mục liên quan còn đúng, nhưng **xác nhận lại với code** khi câu trả lời quan trọng. Nếu code mâu thuẫn, tin code, sửa mục ghi chú, và nói một dòng.

## Định dạng báo cáo sau khi ghi

1-3 dòng:
```
Đã ghi: <mục> — <điều quan trọng>
Đã sửa/xóa: <mục cũ sai>
(nếu có) Đề xuất cải tiến skill: <ý>
```
Không có gì đáng ghi thì không nói gì, đừng ghi cho có.

## Nguyên tắc

- Ít mà chắt: 10 dòng đúng và có nguồn hơn 100 dòng chung chung.
- Code là sự thật. Ghi chú sai thì xóa ngay.
- Không ghi bí mật hay dữ liệu nhạy cảm, kể cả khi user nói trong hội thoại.
- Không đổi code, chỉ đổi tài liệu, trừ khi user yêu cầu.
- Thiếu dữ kiện để kết luận thì nói rõ "chưa kiểm chứng" thay vì ghi như sự thật.
- Không biến skill thành thủ tục rườm rà: không có gì mới đáng ghi thì làm xong việc là đủ.