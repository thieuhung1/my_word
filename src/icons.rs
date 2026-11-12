use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

/// Loại biểu tượng vector trên thanh công cụ
#[derive(Clone, Copy)]
pub enum IconKind {
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    AlignLeft,
    AlignCenter,
    AlignRight,
    AlignJustify,
    Margins,
    FolderOpen,
    Save,
    Search,
    PageBreak,
    Highlight,
    Sun,
    Moon,
}

/// Nút bấm chỉ có biểu tượng vector sắc nét (không phụ thuộc glyph font, không bao giờ lỗi ô vuông □)
pub fn icon_button(
    ui: &mut egui::Ui,
    kind: IconKind,
    tooltip: &str,
    active: bool,
    dark: bool,
) -> egui::Response {
    let size = Vec2::new(28.0, 26.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let enabled = ui.is_enabled();

    let accent = Color32::from_rgb(37, 99, 235);
    let bg = if !enabled {
        Color32::TRANSPARENT
    } else if active {
        if dark {
            Color32::from_rgb(30, 58, 138)
        } else {
            Color32::from_rgb(238, 242, 255)
        }
    } else if response.hovered() {
        if dark {
            Color32::from_rgb(48, 52, 66)
        } else {
            Color32::from_rgb(241, 245, 249)
        }
    } else {
        Color32::TRANSPARENT
    };

    let stroke = if active && enabled {
        Stroke::new(1.0_f32, accent)
    } else {
        Stroke::NONE
    };

    if bg != Color32::TRANSPARENT || stroke != Stroke::NONE {
        ui.painter().rect(rect, Rounding::same(6.0), bg, stroke);
    }

    let fg = if !enabled {
        if dark {
            Color32::from_rgb(75, 85, 99)
        } else {
            Color32::from_rgb(190, 198, 210)
        }
    } else if active {
        if dark {
            Color32::from_rgb(147, 197, 253)
        } else {
            Color32::from_rgb(29, 78, 216)
        }
    } else if dark {
        Color32::from_rgb(226, 232, 240)
    } else {
        Color32::from_rgb(51, 65, 85)
    };

    let icon_rect = rect.shrink2(Vec2::new(6.0, 5.0));
    draw_icon(ui.painter(), icon_rect, kind, fg);

    response.on_hover_text(tooltip)
}

/// Vẽ logo ứng dụng dạng thẻ chữ W bo góc chuyên nghiệp
pub fn draw_app_logo(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(24.0, 24.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(6.0), Color32::from_rgb(37, 99, 235));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "W",
        egui::FontId::proportional(14.0),
        Color32::WHITE,
    );
}

pub fn draw_icon(painter: &egui::Painter, r: Rect, kind: IconKind, color: Color32) {
    let s = Stroke::new(1.5_f32, color);
    let thin = Stroke::new(1.2_f32, color);
    let cx = r.center().x;
    let cy = r.center().y;
    let w = r.width();
    let h = r.height();

    match kind {
        IconKind::Undo => {
            // Mũi tên cong sang trái
            let pts = vec![
                Pos2::new(r.min.x + w * 0.20, cy - h * 0.12),
                Pos2::new(r.min.x + w * 0.55, cy - h * 0.12),
                Pos2::new(r.max.x - w * 0.12, cy + h * 0.05),
                Pos2::new(r.max.x - w * 0.12, cy + h * 0.30),
                Pos2::new(r.min.x + w * 0.45, cy + h * 0.30),
            ];
            painter.add(Shape::line(pts, s));
            // Đầu mũi tên
            let tip = Pos2::new(r.min.x + w * 0.14, cy - h * 0.12);
            painter.line_segment([tip, Pos2::new(tip.x + w * 0.22, tip.y - h * 0.22)], s);
            painter.line_segment([tip, Pos2::new(tip.x + w * 0.22, tip.y + h * 0.22)], s);
        }
        IconKind::Redo => {
            // Mũi tên cong sang phải
            let pts = vec![
                Pos2::new(r.max.x - w * 0.20, cy - h * 0.12),
                Pos2::new(r.min.x + w * 0.45, cy - h * 0.12),
                Pos2::new(r.min.x + w * 0.12, cy + h * 0.05),
                Pos2::new(r.min.x + w * 0.12, cy + h * 0.30),
                Pos2::new(r.max.x - w * 0.45, cy + h * 0.30),
            ];
            painter.add(Shape::line(pts, s));
            let tip = Pos2::new(r.max.x - w * 0.14, cy - h * 0.12);
            painter.line_segment([tip, Pos2::new(tip.x - w * 0.22, tip.y - h * 0.22)], s);
            painter.line_segment([tip, Pos2::new(tip.x - w * 0.22, tip.y + h * 0.22)], s);
        }
        IconKind::Cut => {
            // Cây kéo đơn giản (2 vòng tròn + 2 lưỡi chéo)
            let r_c = w * 0.16;
            painter.circle_stroke(Pos2::new(r.min.x + w * 0.24, r.max.y - h * 0.22), r_c, thin);
            painter.circle_stroke(Pos2::new(r.max.x - w * 0.24, r.max.y - h * 0.22), r_c, thin);
            painter.line_segment(
                [
                    Pos2::new(r.min.x + w * 0.30, r.max.y - h * 0.34),
                    Pos2::new(r.max.x - w * 0.20, r.min.y + h * 0.12),
                ],
                thin,
            );
            painter.line_segment(
                [
                    Pos2::new(r.max.x - w * 0.30, r.max.y - h * 0.34),
                    Pos2::new(r.min.x + w * 0.20, r.min.y + h * 0.12),
                ],
                thin,
            );
        }
        IconKind::Copy => {
            // Hai trang giấy chồng nhau
            let back = Rect::from_min_max(
                Pos2::new(r.min.x + w * 0.12, r.min.y + h * 0.05),
                Pos2::new(r.max.x - w * 0.26, r.max.y - h * 0.25),
            );
            let front = Rect::from_min_max(
                Pos2::new(r.min.x + w * 0.30, r.min.y + h * 0.25),
                Pos2::new(r.max.x - w * 0.08, r.max.y - h * 0.05),
            );
            painter.rect_stroke(back, Rounding::same(2.0), thin);
            painter.rect_stroke(front, Rounding::same(2.0), s);
        }
        IconKind::Paste => {
            // Bảng kẹp giấy (Clipboard)
            let board = Rect::from_min_max(
                Pos2::new(r.min.x + w * 0.16, r.min.y + h * 0.18),
                Pos2::new(r.max.x - w * 0.16, r.max.y - h * 0.04),
            );
            let clip = Rect::from_min_max(
                Pos2::new(cx - w * 0.18, r.min.y + h * 0.04),
                Pos2::new(cx + w * 0.18, r.min.y + h * 0.24),
            );
            painter.rect_stroke(board, Rounding::same(2.0), thin);
            painter.rect_filled(clip, Rounding::same(1.5), color);
        }
        IconKind::AlignLeft => {
            let ys = [0.18, 0.40, 0.62, 0.84];
            let lens = [0.90, 0.60, 0.85, 0.50];
            for (y_f, len_f) in ys.iter().zip(lens.iter()) {
                let y = r.min.y + h * y_f;
                painter.line_segment(
                    [
                        Pos2::new(r.min.x + w * 0.05, y),
                        Pos2::new(r.min.x + w * (0.05 + len_f * 0.9), y),
                    ],
                    s,
                );
            }
        }
        IconKind::AlignCenter => {
            let ys = [0.18, 0.40, 0.62, 0.84];
            let lens = [0.90, 0.56, 0.82, 0.46];
            for (y_f, len_f) in ys.iter().zip(lens.iter()) {
                let y = r.min.y + h * y_f;
                let half = (w * len_f) * 0.5;
                painter.line_segment([Pos2::new(cx - half, y), Pos2::new(cx + half, y)], s);
            }
        }
        IconKind::AlignRight => {
            let ys = [0.18, 0.40, 0.62, 0.84];
            let lens = [0.90, 0.60, 0.85, 0.50];
            for (y_f, len_f) in ys.iter().zip(lens.iter()) {
                let y = r.min.y + h * y_f;
                painter.line_segment(
                    [
                        Pos2::new(r.max.x - w * (0.05 + len_f * 0.9), y),
                        Pos2::new(r.max.x - w * 0.05, y),
                    ],
                    s,
                );
            }
        }
        IconKind::AlignJustify => {
            let ys = [0.18, 0.40, 0.62, 0.84];
            for y_f in ys {
                let y = r.min.y + h * y_f;
                painter.line_segment(
                    [
                        Pos2::new(r.min.x + w * 0.05, y),
                        Pos2::new(r.max.x - w * 0.05, y),
                    ],
                    s,
                );
            }
        }
        IconKind::FolderOpen => {
            let folder = Rect::from_min_max(
                Pos2::new(r.min.x + w * 0.08, r.min.y + h * 0.24),
                Pos2::new(r.max.x - w * 0.08, r.max.y - h * 0.10),
            );
            painter.rect_stroke(folder, Rounding::same(2.0), thin);
            let tab = [
                Pos2::new(r.min.x + w * 0.08, r.min.y + h * 0.24),
                Pos2::new(r.min.x + w * 0.16, r.min.y + h * 0.10),
                Pos2::new(r.min.x + w * 0.46, r.min.y + h * 0.10),
                Pos2::new(r.min.x + w * 0.54, r.min.y + h * 0.24),
            ];
            painter.add(Shape::line(tab.to_vec(), thin));
        }
        IconKind::Save => {
            let box_r = r.shrink(1.0);
            painter.rect_stroke(box_r, Rounding::same(2.0), thin);
            let top_slot = Rect::from_min_max(
                Pos2::new(box_r.min.x + 3.0, box_r.min.y),
                Pos2::new(box_r.max.x - 4.0, box_r.min.y + box_r.height() * 0.38),
            );
            painter.rect_stroke(top_slot, Rounding::ZERO, thin);
        }
        IconKind::Search => {
            let center = Pos2::new(r.min.x + w * 0.42, r.min.y + h * 0.42);
            let rad = w.min(h) * 0.28;
            painter.circle_stroke(center, rad, s);
            let offset = rad * 0.72;
            painter.line_segment(
                [
                    Pos2::new(center.x + offset, center.y + offset),
                    Pos2::new(r.max.x - w * 0.10, r.max.y - h * 0.10),
                ],
                s,
            );
        }
        IconKind::PageBreak => {
            let top_r = Rect::from_min_max(
                Pos2::new(r.min.x + w * 0.16, r.min.y + h * 0.06),
                Pos2::new(r.max.x - w * 0.16, cy - h * 0.14),
            );
            let bot_r = Rect::from_min_max(
                Pos2::new(r.min.x + w * 0.16, cy + h * 0.14),
                Pos2::new(r.max.x - w * 0.16, r.max.y - h * 0.06),
            );
            painter.rect_stroke(top_r, Rounding::same(1.0), thin);
            painter.rect_stroke(bot_r, Rounding::same(1.0), thin);
            // Đường đứt ở giữa
            painter.line_segment(
                [
                    Pos2::new(r.min.x + 1.0, cy),
                    Pos2::new(r.min.x + w * 0.36, cy),
                ],
                thin,
            );
            painter.line_segment(
                [
                    Pos2::new(r.max.x - w * 0.36, cy),
                    Pos2::new(r.max.x - 1.0, cy),
                ],
                thin,
            );
        }
        IconKind::Highlight => {
            // Bút dạ quang + vạch highlight bên dưới
            let pen = vec![
                Pos2::new(r.min.x + w * 0.22, r.max.y - h * 0.28),
                Pos2::new(r.max.x - w * 0.26, r.min.y + h * 0.10),
                Pos2::new(r.max.x - w * 0.10, r.min.y + h * 0.26),
                Pos2::new(r.min.x + w * 0.38, r.max.y - h * 0.20),
                Pos2::new(r.min.x + w * 0.22, r.max.y - h * 0.28),
            ];
            painter.add(Shape::line(pen, thin));
            painter.line_segment(
                [
                    Pos2::new(r.min.x + w * 0.12, r.max.y - h * 0.08),
                    Pos2::new(r.max.x - w * 0.12, r.max.y - h * 0.08),
                ],
                Stroke::new(2.2_f32, Color32::from_rgb(234, 179, 8)),
            );
        }
        IconKind::Margins => {
            // Khung trang kèm dấu góc lề
            let page = r.shrink(1.0);
            painter.rect_stroke(page, Rounding::same(2.0), thin);
            let inner = page.shrink(3.5);
            let d = 3.0;
            // 4 góc L
            painter.line_segment(
                [inner.left_top(), inner.left_top() + Vec2::new(d, 0.0)],
                thin,
            );
            painter.line_segment(
                [inner.left_top(), inner.left_top() + Vec2::new(0.0, d)],
                thin,
            );
            painter.line_segment(
                [
                    inner.right_bottom(),
                    inner.right_bottom() - Vec2::new(d, 0.0),
                ],
                thin,
            );
            painter.line_segment(
                [
                    inner.right_bottom(),
                    inner.right_bottom() - Vec2::new(0.0, d),
                ],
                thin,
            );
        }
        IconKind::Sun => {
            painter.circle_stroke(Pos2::new(cx, cy), w.min(h) * 0.24, thin);
            let r1 = w.min(h) * 0.34;
            let r2 = w.min(h) * 0.48;
            for i in 0..8 {
                let ang = (i as f32) * std::f32::consts::FRAC_PI_4;
                let dir = Vec2::new(ang.cos(), ang.sin());
                painter.line_segment(
                    [Pos2::new(cx, cy) + dir * r1, Pos2::new(cx, cy) + dir * r2],
                    thin,
                );
            }
        }
        IconKind::Moon => {
            // Hình trăng lưỡi liềm khép kín bằng đa giác 10 điểm mượt mà
            let pts = vec![
                Pos2::new(cx + w * 0.14, r.min.y + h * 0.10),
                Pos2::new(cx - w * 0.16, r.min.y + h * 0.22),
                Pos2::new(r.min.x + w * 0.16, cy),
                Pos2::new(cx - w * 0.16, r.max.y - h * 0.22),
                Pos2::new(cx + w * 0.14, r.max.y - h * 0.10),
                Pos2::new(cx + w * 0.34, r.max.y - h * 0.24),
                Pos2::new(cx + w * 0.08, r.max.y - h * 0.26),
                Pos2::new(cx - w * 0.04, cy),
                Pos2::new(cx + w * 0.08, r.min.y + h * 0.26),
                Pos2::new(cx + w * 0.34, r.min.y + h * 0.24),
                Pos2::new(cx + w * 0.14, r.min.y + h * 0.10),
            ];
            painter.add(Shape::line(pts, thin));
        }
    }
}
