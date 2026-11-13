# KNOWLEDGE

## Kiến trúc & ranh giới
- [src/main.rs](src/main.rs): Entry point gọn nhẹ (~25 dòng), cấu hình `NativeOptions`, nạp font qua `platform::setup_fonts`, áp dụng giao diện qua `ui::ThemePalette::apply` và khởi chạy `app::MyWordApp` (xác minh: 2026-11-13).
- [src/app.rs](src/app.rs): `MyWordApp`, xử lý tuần tự luồng sự kiện bàn phím/Clipboard/TSF IME và điều phối `update` giữa editor, layout, platform, UI; chứa các hằng số chuẩn `CARET_BLINK_MS = 550`, `AUTOSAVE_INTERVAL_SECS = 20`, `UNIKEY_BURST_WINDOW_MS = 45` (xác minh: 2026-11-13).
- [src/editor.rs](src/editor.rs): `EditorState` đóng gói toàn bộ logic soạn thảo độc lập UI (grapheme backspace/delete, selection, Rich Text spans, căn lề đoạn, Telex, Undo/Redo bằng `VecDeque` $O(1)$ tối đa 50 bước gộp 800ms, Find/Replace) kèm bộ unit test riêng (xác minh: 2026-11-13).
- [src/layout.rs](src/layout.rs): `CachedLayout`, hằng số A4 (`595 x 842 pt`, lề `72 pt`), tính toán `Galley` và phân trang tự động chỉ chạy khi `layout_dirty` hoặc `zoom` đổi (xác minh: 2026-11-13).
- [src/ui.rs](src/ui.rs): `ThemePalette` (Light/Dark, `apply(&self, ctx)`), `render_header_bar`, `render_ribbon_bar`, `render_find_replace_bar`, `render_page_canvas`, `render_status_bar`, `render_recovery_banner`, `render_confirm_modal` (xác minh: 2026-11-13).
- [src/platform.rs](src/platform.rs): Nạp 4 biến thể `Segoe UI` trên `FontDefinitions::default()`, hộp thoại Mở/Lưu Windows bất đồng bộ (PowerShell STA thread không nháy CMD), đọc/ghi `.mwd`/`.txt`/`.md` và Auto-Save `%LOCALAPPDATA%\my_word\autosave.mwd`. Hoàn toàn độc lập với module `ui` (xác minh: 2026-11-13).
- [src/document.rs](src/document.rs): Mô hình tài liệu Rich Text với `StyleSpan`, căn lề đoạn (`Left`, `Center`, `Right`, `Justify`), `PAGE_BREAK_CHAR` (`\x0c`), tìm kiếm streaming UTF-8 (`find_matches`), và tuần tự hóa `.mwd` (`MYWORD_V1`) không phụ thuộc `serde` (xác minh: 2026-11-13).
- [src/telex.rs](src/telex.rs): Bộ gõ tiếng Việt Telex tích hợp (`try_apply_telex`) thao tác tại chỗ không `.clone()` dư thừa (xác minh: 2026-11-13).
- [src/icons.rs](src/icons.rs): Thư viện vẽ biểu tượng vector thuần bằng `egui::Painter` (`icon_button`, `draw_app_logo`), loại bỏ lỗi ô vuông `□` trên Windows.
- Đồ thị phụ thuộc module là DAG chuẩn (Directed Acyclic Graph, không vòng lặp):
  `main` ──► `platform`, `ui`, `app`
  `app`  ──► `editor`, `layout`, `platform`, `ui`, `document`
  `ui`   ──► `icons`, `layout`, `document`, `editor`, `app`
  `platform` ──► `document`, `editor`
  `editor` ──► `telex`, `document`
  `layout` ──► `document`
  `document`, `telex`, `icons` là các lá độc lập.
- Hồ sơ build `[profile.release]` trong [Cargo.toml](Cargo.toml): `opt-level = "s"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`, tạo file thực thi release ~1.97 MB; CI tách riêng gồm [.github/workflows/test.yml](.github/workflows/test.yml) (nhánh `test`) và [.github/workflows/dev.yml](.github/workflows/dev.yml) (nhánh `dev` chính thức) (xác minh: 2026-11-13).

## Quy ước
- Nhánh GitHub tách riêng chỉ gồm 2 nhánh: `test` (thử nghiệm & chạy test/lint) và `dev` (chính thức & build release); chỉ đưa code từ `test` sang `dev` sau khi kiểm thử đạt, không dùng `main` (nguồn: AGENTS.md; xác minh: 2026-11-12).
- Mọi thao tác xóa lùi (`backspace`) và xóa tiến (`delete_forward`) bắt buộc tính toán trên ranh giới Grapheme Cluster qua `unicode-segmentation`, không xóa theo byte thô để tránh vỡ dấu tiếng Việt (nguồn: src/main.rs:43; xác minh: 2026-11-08).
- Con trỏ Caret chỉ dừng ở các vị trí thỏa mãn `content.is_char_boundary(offset)`.

## Lệnh
- Chạy toàn bộ Unit Tests: `cargo test`
- Kiểm tra cú pháp nhanh: `cargo check`
- Khởi chạy chế độ tối ưu: `cargo run --release`

## Bẫy & lưu ý
- Trên Windows GNU toolchain (`x86_64-pc-windows-gnu`): Không dùng linker GCC UCRT ngoài với đường dẫn tuyệt đối vì gây xung đột MSVCRT/UCRT dẫn tới lỗi `0xc0000005` trong `__gcc_register_frame`. Luôn dùng `linker = "rust-lld"` trong `.cargo/config.toml` (nguồn: LESSONS.md; xác minh: 2026-11-08).

## Invariant & luật nghiệp vụ
- Khổ giấy A4 hiển thị: 595.0 pt x 842.0 pt. Lề trang chuẩn: 72.0 pt (1 inch). Chiều rộng khả dụng: 451.0 pt.

