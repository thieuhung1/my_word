---
name: "code-mistake-lessons"
description: "Tự rút kinh nghiệm khi code sai: phân tích nguyên nhân gốc, ghi bài học vào LESSONS.md, đọc lại trước khi code lần sau, leo thang khi lặp lại. Dùng khi bug, test fail sau khi mình sửa, bị user chỉnh, phải revert, hoặc user hỏi \"sai ở đâu\", \"rút kinh nghiệm\"."
---

# Tự rút kinh nghiệm khi code sai

Biến một sai lầm cụ thể thành quy tắc kiểm tra được, **lưu lại**, và **áp dụng lại** ở lần code sau. Áp dụng cho mọi ngôn ngữ. Blameless: nói về quy trình, không đổ lỗi cho người.

Trả lời bằng ngôn ngữ user đang dùng. Ngắn gọn, thẳng, không giải thích dài dòng.

## Giới hạn trung thực

Claude/Agent không tự nhớ giữa các phiên, và skill không tự sửa chính nó. "Tự cải tiến" ở đây hoạt động nhờ **file bài học** (`LESSONS.md` của dự án) được ghi sau mỗi lỗi và đọc lại trước mỗi lần code. Không có file thì không có trí nhớ. Nói rõ điều này với user khi cần.

## Vòng lặp

1. **Trước khi code**: nếu có `LESSONS.md` (hoặc mục bài học trong AGENTS.md), đọc và áp dụng các quy tắc liên quan đến thay đổi sắp làm. Không nhắc lại trừ khi quy tắc thay đổi cách làm.
2. **Khi sai**: chạy Bước 1-5 bên dưới.
3. **Sau đó**: ghi bài học (nếu qua phản biện), lần sau quay lại bước 1.

## Khi nào tự kích hoạt (không chờ user hỏi)

- Test/build/lint fail ngay sau khi mình sửa.
- User chỉnh, phản đối, hoặc yêu cầu sửa lại điều mình vừa làm.
- Phải sửa lại lần thứ hai cho cùng một việc.
- Phải revert.
- Review chỉ ra lỗi thật.

Khi tự kích hoạt: sửa lỗi trước, rồi rút bài học ngắn gọn (1-3 dòng) sau khi xong, không chặn công việc user đang cần. Lỗi thật sự nhỏ, không đáng quy tắc thì bỏ qua.

## Bước 1: Thu thập sự kiện (không đoán)

Đọc diff, log, stack trace, hội thoại, commit liên quan. Ghi ngắn:
- **Triệu chứng**: thấy gì sai, ở đâu, khi nào.
- **Thay đổi gây ra**: commit/đoạn code nào đưa lỗi vào.
- **Phát hiện bằng cách nào và muộn bao lâu**: test, review, production, user báo.
- **Cách sửa**: sửa triệu chứng hay sửa gốc.

Thiếu sự kiện thì nói rõ thiếu gì, không bịp.

## Bước 2: Tìm nguyên nhân gốc

Hỏi "vì sao" liên tục (khoảng 5 lần) cho đến khi ra thứ sửa được bằng quy tắc. Phân biệt:
- **Kích hoạt**: điều kiện làm lỗi lộ ra (input lạ, tải cao).
- **Gốc**: quyết định hoặc giả định sai tạo ra lỗi.
- **Góp phần**: vì sao không ai bắt được (thiếu test, review vội, thiếu log).

Phân loại (chọn đúng, không chọn tràn lan):
- Hiểu sai yêu cầu hoặc giả định ngầm không kiểm chứng
- Không đọc code/ngữ cảnh hiện có trước khi sửa (caller, middleware, side effect)
- Bỏ sót edge case (null, rỗng, trùng, múi giờ, Unicode, số lớn, song song)
- Sửa triệu chứng chứ không sửa gốc
- Nuốt lỗi hoặc xử lý lỗi sai
- Copy-paste không hiểu, dùng API sai vì không đọc docs/version
- Không chạy thử hoặc test không đúng chỗ thay đổi
- Over-engineering hoặc làm thừa gây lỗi mới
- Phụ thuộc ngầm (thứ tự, trạng thái toàn cục, cấu hình môi trường)
- Thay đổi lớn một lần, không chia nhỏ nên không biết cái nào gây lỗi

## Bước 3: Viết bài học thành quy tắc kiểm tra được

Mỗi bài học có 3 phần:
- **Khi nào**: tình huống kích hoạt (cụ thể, nhận ra được).
- **Làm gì**: hành động cụ thể.
- **Kiểm tra thế nào**: cách biết đã làm đúng.

Tốt: "Khi sửa hàm được gọi từ nhiều nơi → grep toàn bộ caller trước khi đổi signature → không còn caller nào dùng cách cũ."
Xấu (cấm): "Cẩn thận hơn", "nhớ test kỹ", "đọc code kỹ hơn".

## Bước 4: Phòng ngừa bằng cơ chế, không bằng trí nhớ

Ưu tiên theo thứ tự rẻ và tự động:
1. Kiểu dữ liệu/type/compiler không cho làm sai
2. Test tái hiện đúng lỗi này (viết test fail trước, sửa xong thì pass)
3. Lint/CI/hook bắt tự động
4. Mục trong checklist review
5. Dòng trong LESSONS.md (cách yếu nhất, chỉ là nhắc nhở)

Chỉ đề xuất cơ chế tương xứng với mức độ lỗi. Lỗi nhỏ một lần không cần thêm hạ tầng mới.

## Bước 5: Phản biện lại bài học của mình

Trước khi ghi, soi từng bài học:
- **Có đúng nguyên nhân gốc không?** Hay chỉ là triệu chứng hay yếu tố góp phần?
- **Có khái quát quá từ một trường hợp không?** Mới thấy một lần thì ghi "mới thấy 1 lần", không nâng thành nguyên tắc chung.
- **Quy tắc cụ thể và kiểm tra được không?** Nếu là khẩu hiệu thì viết lại hoặc bỏ.
- **Chi phí có hơn lợi ích không?** Quy tắc làm chậm mọi việc để ngăn lỗi hiếm thì hạ xuống hoặc bỏ.
- **Có đang đổ lỗi hay bỏ qua yếu tố hệ thống không?** (thiếu thời gian, yêu cầu mơ hồ, thiếu CI).
- **Thử giả thuyết ngược**: nếu áp bài học này từ đầu, lỗi có chắc chắn không xảy ra?
- **Đã có bài học tương tự trong LESSONS.md chưa?** Có thì gộp, không thêm dòng mới.

Mỗi bài học kết luận: ✅ **Giữ**, ⬇️ **Sửa lại/hạ phạm vi**, ❌ **Bỏ**. Phản biện phải thực sự thay đổi kết quả, không phản biện hình thức.

## Bước 6: Ghi và quản lý LESSONS.md

**Quyền ghi**: Tự động ghi vào `LESSONS.md` khi người dùng đã yêu cầu tự động. Nói một dòng đã ghi gì.

**Định dạng mỗi dòng**:
```markdown
- [x1] Khi <tình huống> → <hành động> → kiểm tra: <cách kiểm tra>
```
`x1` là số lần lỗi này đã xảy ra.

**Giữ file gọn**:
- Gộp bài trùng, tăng bộ đếm thay vì thêm dòng mới.
- Xóa bài đã lỗi thời (code đã đổi, đã có test/lint bắt thay).
- Tối đa khoảng 20 dòng. Quá thì gộp hoặc bỏ bài ít giá trị nhất.
- Không ghi bí mật, token, dữ liệu khách hàng, hay mô tả dài của lỗi. Chỉ ghi quy tắc.

**Leo thang khi lặp lại** (đây là phần tự cải tiến thật sự):
- `x2`: dòng nhắc nhở đã thất bại. Đề xuất cơ chế tự động (test, type, lint rule, CI) thay vì chỉ giữ dòng.
- `x3` hoặc lặp ở nhiều dự án: đề xuất cập nhật chính skill này hoặc skill review liên quan.

## Định dạng đầu ra khi phân tích đầy đủ

```markdown
## Sự kiện
(triệu chứng, thay đổi gây lỗi, cách phát hiện, cách sửa: 3-5 dòng)

## Nguyên nhân
- Gốc: ...
- Kích hoạt: ...
- Góp phần: ...
- Loại sai lầm: ...

## Bài học
1. Khi ... → làm ... → kiểm tra ...

## Phòng ngừa
(cơ chế cụ thể, snippet tối thiểu nếu có)

## Phản biện lại
(từng bài học: ✅ / ⬇️ / ❌ + lý do 1 dòng)

## Đã ghi
(dòng đã thêm/gộp trong LESSONS.md)
```

Khi tự kích hoạt sau lỗi nhỏ, chỉ cần 1-3 dòng: gốc là gì, quy tắc gì, đã ghi.

## Nguyên tắc

- Blameless: nói về quy trình, không nói về người.
- Ít mà chắt: 1-3 bài học đúng hơn 10 bài chung chung.
- Nói thẳng khi lỗi là do chính mình (ví dụ sửa sai vì không đọc caller), không né tránh.
- Không viết lại code trừ khi được yêu cầu; chỉ đưa snippet tối thiểu cho phần phòng ngừa.
- Thiếu dữ kiện để kết luận thì nói rõ "chưa chắc, cần xem thêm X".
- Không biến skill thành thủ tục rườm rà: lỗi nhỏ, một lần, không lặp thì sửa xong là đủ.

