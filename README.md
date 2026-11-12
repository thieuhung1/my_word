# My Word (`my_word`)

Trình soạn thảo văn bản tiếng Việt gọn nhẹ viết bằng **Rust** (`egui` / `eframe`), hỗ trợ định dạng Rich Text theo vùng chọn, phân trang khổ giấy A4 theo thời gian thực, bộ gõ tiếng Việt Telex tích hợp và tương thích Unikey / Windows TSF IME.

[![Rust 2024](https://img.shields.io/badge/Rust-Edition_2024-orange.svg)](https://www.rust-lang.org/)
[![egui 0.28](https://img.shields.io/badge/GUI-egui%200.28-2563eb.svg)](https://github.com/emilk/egui)
[![Release Size](https://img.shields.io/badge/Release_Binary-~2.0_MB-16a34a.svg)](#tối-ưu-bộ-nhớ--build)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

---

## Tính năng chính

### Soạn thảo tiếng Việt chuẩn xác
- **Telex tích hợp + Tương thích bộ gõ hệ thống:** Hỗ trợ gõ tiếng Việt Telex trực tiếp (`Telex: Tự động` với dấu mũ/móc, `đ`, 5 thanh điệu, phím `z` xóa dấu, escape lặp phím tiếng Anh) hoặc chuyển sang chế độ `Telex: Hệ thống` cho Unikey / EVKey / Windows TSF IME.
- **Chuẩn hóa Unicode NFC & an toàn Grapheme Cluster:** Tự động chuẩn hóa NFC qua `unicode-normalization`; xóa ký tự (`Backspace` / `Delete`) và di chuyển con trỏ tính theo ranh giới grapheme (`unicode-segmentation`) để không làm vỡ dấu tiếng Việt.

### Định dạng Rich Text & Bố cục trang A4
- **Rich Text theo vùng chọn:** Đậm (`B`), Nghiêng (`I`), Gạch chân (`U`), Gạch ngang chữ (`S`), tăng/giảm cỡ chữ (`8–48 pt`), màu chữ và tô sáng `Highlight`.
- **Phân trang A4 tự động (`595 × 842 pt`, lề `72 pt`):** Tự động ngắt trang khi tràn nội dung, hỗ trợ chèn ngắt trang thủ công (`Ctrl+Enter`), căn lề từng đoạn (`Left`, `Center`, `Right`, `Justify`), bật/tắt góc căn lề và thu phóng `60%–150%`.
- **Tìm kiếm & Thay thế (`Ctrl+F` / `Ctrl+H`):** Tìm kiếm UTF-8 tiếng Việt (tùy chọn phân biệt hoa/thường `Aa`), điều hướng kết quả Trước/Sau và Thay thế từng mục hoặc Thay tất cả.
- **Lưu/Mở tệp & Auto-Save:** Hỗ trợ định dạng `.mwd` (`MYWORD_V1`, bảo toàn toàn bộ Rich Text + căn lề từng đoạn) và `.txt` / `.md`, kèm tự động sao lưu định kỳ mỗi 20 giây vào `%LOCALAPPDATA%\my_word\autosave.mwd`.
- **Giao diện Sáng / Tối & Icon Vector thuần:** Toàn bộ icon trên Header và Ribbon được vẽ bằng `egui::Painter`, không phụ thuộc glyph emoji của hệ điều hành.

---

## Phím tắt

| Phím tắt | Chức năng |
| :--- | :--- |
| `Ctrl+B` / `Ctrl+I` / `Ctrl+U` | Bật/tắt In đậm / In nghiêng / Gạch chân |
| `Ctrl+Z` / `Ctrl+Y` (`Ctrl+Shift+Z`) | Hoàn tác / Làm lại |
| `Ctrl+A` / `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Chọn tất cả / Sao chép / Cắt / Dán |
| `Ctrl+Backspace` | Xóa lùi theo từ |
| `Ctrl+Enter` | Chèn ngắt trang |
| `Ctrl+F` / `Ctrl+H` / `Esc` | Tìm kiếm / Tìm & Thay thế / Đóng thanh tìm kiếm |
| `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S` | Mở tệp / Lưu / Lưu mới (Save As) |

---

## Cấu trúc mã nguồn

```text
my_word/
├── .github/workflows/
│   ├── dev.yml                # CI nhánh chính thức dev (fmt, clippy, test, release build)
│   └── test.yml               # CI nhánh thử nghiệm test (fmt, clippy, test)
├── .cargo/config.toml         # Cấu hình rust-lld cho target Windows GNU
├── Cargo.toml                 # Phụ thuộc tối giản + profile.release (LTO, strip)
└── src/
    ├── main.rs                # Entry point gọn nhẹ khởi tạo NativeOptions & font
    ├── app.rs                 # MyWordApp, vòng lặp sự kiện bàn phím/clipboard/IME
    ├── editor.rs              # EditorState: soạn thảo, selection, Undo/Redo, Telex, tests
    ├── document.rs            # Mô hình Rich Text spans, tìm kiếm UTF-8, định dạng .mwd
    ├── telex.rs               # Bộ chuyển đổi quy tắc gõ tiếng Việt Telex
    ├── layout.rs              # CachedLayout & phân trang A4 tự động (không tính lại khi rỗi)
    ├── ui.rs                  # ThemePalette, Header, Ribbon, Find/Replace, Canvas A4, Status bar
    ├── platform.rs            # Nạp phông hệ thống Windows, Open/Save dialog, Auto-Save
    └── icons.rs               # Bộ icon vector vẽ bằng egui::Painter
```

---

## Tối ưu bộ nhớ & build

- Tắt `default-features` của `eframe` / `egui` để loại bỏ phông nhúng mặc định (~2.2 MB) và cây accessibility không dùng tới; nạp trực tiếp phông `Segoe UI` từ Windows qua `FontDefinitions::empty()`.
- `CachedLayout` lưu đệm `Galley` và vị trí phân trang A4, chỉ tính lại khi nội dung, định dạng hoặc tỷ lệ zoom thay đổi.
- Gộp chuỗi gõ liên tiếp trong `800ms` thành một snapshot Undo (giới hạn 50 bước) và tìm kiếm streaming không cấp phát `Vec` cho toàn văn bản.
- Cấu hình `[profile.release]` (`opt-level = "s"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`) cho file thực thi ~2 MB.

---

## Cài đặt & phát triển

### Yêu cầu
- [Rust toolchain](https://rustup.rs/) (Edition 2024).
- Trên Windows với target `x86_64-pc-windows-gnu`: đã cấu hình sẵn `linker = "rust-lld"` trong [.cargo/config.toml](.cargo/config.toml).

### Lệnh thường dùng

```bash
# Chạy kiểm tra chuẩn chất lượng đầy đủ
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test

# Khởi chạy ứng dụng
cargo run
cargo run --release
```

### Quy ước nhánh Git
- `dev`: Nhánh chính thức (`release` ổn định).
- `test`: Nhánh thử nghiệm tách riêng để kiểm tra tính năng và CI trước khi đưa sang `dev`.

---

## Giấy phép

Phân phối theo [MIT License](LICENSE).
