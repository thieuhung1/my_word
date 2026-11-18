use std::path::PathBuf;

use crate::app::{InspectorTab, MyWordApp, NavTab, RibbonTab, StartupNavTab};
use crate::document::{PAGE_BREAK_CHAR, TextAlignment};
use crate::editor::{DocumentPresetStyle, InputMethodMode};
use crate::icons::{IconKind, draw_app_logo, icon_button, paint_inline_icon};
use crate::layout::{
    A4_HEIGHT_PT, A4_WIDTH_PT, CachedLayout, PAGE_MARGIN_PT, PointerQuery,
    byte_offset_from_char_index, char_index_from_byte_offset, font_id_for_style,
    paint_range_highlight,
};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Rounding, Shape, Stroke, Vec2};

pub struct ThemePalette {
    pub is_dark: bool,
    pub surface_bg: Color32,
    pub sidebar_bg: Color32,
    pub toolbar_bg: Color32,
    pub workspace_bg: Color32,
    pub border_color: Color32,
    pub text_main: Color32,
    pub text_muted: Color32,
    pub primary_accent: Color32,
    pub accent_tint: Color32,
    pub accent_border: Color32,
    pub input_bg: Color32,
    pub card_bg: Color32,
}

impl ThemePalette {
    pub fn from_mode(is_dark: bool) -> Self {
        if is_dark {
            Self {
                is_dark: true,
                surface_bg: Color32::from_rgb(24, 27, 34),
                sidebar_bg: Color32::from_rgb(20, 22, 28),
                toolbar_bg: Color32::from_rgb(24, 27, 34),
                workspace_bg: Color32::from_rgb(15, 17, 22),
                border_color: Color32::from_rgb(44, 49, 62),
                text_main: Color32::from_rgb(241, 245, 249),
                text_muted: Color32::from_rgb(148, 163, 184),
                primary_accent: Color32::from_rgb(99, 122, 245),
                accent_tint: Color32::from_rgb(36, 46, 98),
                accent_border: Color32::from_rgb(72, 92, 180),
                input_bg: Color32::from_rgb(32, 36, 46),
                card_bg: Color32::from_rgb(30, 34, 44),
            }
        } else {
            Self {
                is_dark: false,
                surface_bg: Color32::WHITE,
                sidebar_bg: Color32::from_rgb(250, 251, 253),
                toolbar_bg: Color32::WHITE,
                workspace_bg: Color32::from_rgb(240, 242, 245),
                border_color: Color32::from_rgb(229, 233, 240),
                text_main: Color32::from_rgb(35, 43, 60),
                text_muted: Color32::from_rgb(101, 112, 135),
                primary_accent: Color32::from_rgb(64, 89, 220),
                accent_tint: Color32::from_rgb(238, 241, 255),
                accent_border: Color32::from_rgb(201, 210, 255),
                input_bg: Color32::from_rgb(245, 246, 249),
                card_bg: Color32::from_rgb(248, 249, 251),
            }
        }
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = if self.is_dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.widgets.noninteractive.rounding = Rounding::same(6.0);
        visuals.widgets.inactive.rounding = Rounding::same(6.0);
        visuals.widgets.hovered.rounding = Rounding::same(6.0);
        visuals.widgets.active.rounding = Rounding::same(6.0);
        visuals.selection.bg_fill = self.accent_tint;
        visuals.selection.stroke = Stroke::new(1.0_f32, self.primary_accent);
        ctx.set_visuals(visuals);
    }
}

fn vertical_divider(ui: &mut egui::Ui, height: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(7.0, height), egui::Sense::hover());
    ui.painter().line_segment(
        [
            Pos2::new(rect.center().x, rect.min.y + 2.0),
            Pos2::new(rect.center().x, rect.max.y - 2.0),
        ],
        Stroke::new(1.0_f32, color),
    );
}

pub fn render_header_bar(ctx: &egui::Context, app: &mut MyWordApp, theme: &ThemePalette) {
    egui::TopBottomPanel::top("app_header")
        .frame(
            egui::Frame::none()
                .fill(theme.surface_bg)
                .inner_margin(egui::Margin::symmetric(16.0, 9.0))
                .stroke(Stroke::new(1.0_f32, theme.border_color)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Brand Monogram + Tên ứng dụng
                draw_app_logo(ui);
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("my word")
                        .size(16.5)
                        .color(theme.text_main)
                        .strong(),
                );
                ui.add_space(6.0);
                vertical_divider(ui, 24.0, theme.border_color);
                ui.add_space(4.0);

                // Tên tài liệu & trạng thái lưu
                let doc_name = app
                    .current_file_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("Kế hoạch thương hiệu 2026.docx");
                let dirty_mark = if app.editor.is_dirty { " *" } else { "" };
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.label(
                            egui::RichText::new(format!("{}{}", doc_name, dirty_mark))
                                .size(13.5)
                                .color(theme.text_main)
                                .strong(),
                        );
                        ui.label(egui::RichText::new("▾").size(11.0).color(theme.text_muted));
                    });
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let saved_col = if app.editor.is_dirty {
                            Color32::from_rgb(217, 119, 6)
                        } else {
                            Color32::from_rgb(22, 163, 74)
                        };
                        paint_inline_icon(ui, 13.0, IconKind::CloudCheck, saved_col);
                        let saved_text = if app.editor.is_dirty {
                            "Có thay đổi chưa lưu"
                        } else {
                            "Đã lưu mọi thay đổi"
                        };
                        ui.label(
                            egui::RichText::new(saved_text)
                                .size(10.5)
                                .color(theme.text_muted),
                        );
                    });
                });

                ui.add_space(16.0);

                // Thanh tìm kiếm nhanh kiểu Figma (Ctrl K)
                let avail_w = ui.available_width();
                if avail_w > 540.0 {
                    let pill_w = (avail_w - 460.0).clamp(180.0, 316.0);
                    let search_btn = egui::Button::new(
                        egui::RichText::new("🔍  Tìm kiếm công cụ và trợ giúp       Ctrl K")
                            .size(11.5)
                            .color(theme.text_muted),
                    )
                    .fill(theme.input_bg)
                    .stroke(Stroke::new(1.0_f32, theme.border_color))
                    .rounding(Rounding::same(7.0))
                    .min_size(Vec2::new(pill_w, 32.0));
                    if ui
                        .add(search_btn)
                        .on_hover_text("Mở thanh Tìm kiếm & Thay thế (Ctrl+F / Ctrl+K)")
                        .clicked()
                    {
                        app.find_bar_open = true;
                        app.focus_find_input = true;
                    }
                }

                // Khu vực nút chức năng bên phải
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    // Nút Chia sẻ kiểu Figma (#4059DC)
                    let (share_rect, share_resp) =
                        ui.allocate_exact_size(Vec2::new(88.0, 30.0), egui::Sense::click());
                    let share_bg = if share_resp.hovered() {
                        theme.primary_accent.linear_multiply(0.9)
                    } else {
                        theme.primary_accent
                    };
                    ui.painter()
                        .rect_filled(share_rect, Rounding::same(7.0), share_bg);
                    let share_icon_r = Rect::from_min_size(
                        Pos2::new(share_rect.min.x + 10.0, share_rect.center().y - 7.0),
                        Vec2::splat(14.0),
                    );
                    crate::icons::draw_icon(
                        ui.painter(),
                        share_icon_r,
                        IconKind::Share,
                        Color32::WHITE,
                    );
                    ui.painter().text(
                        Pos2::new(share_rect.min.x + 28.0, share_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        "Chia sẻ",
                        FontId::proportional(12.0),
                        Color32::WHITE,
                    );
                    if share_resp.on_hover_text("Chia sẻ tài liệu").clicked() {
                        app.set_status("Tính năng chia sẻ đang chuẩn bị");
                    }

                    // Nút Bình luận kiểu Figma
                    let (comment_rect, comment_resp) =
                        ui.allocate_exact_size(Vec2::new(96.0, 30.0), egui::Sense::click());
                    let comment_bg = if comment_resp.hovered() {
                        theme.card_bg
                    } else {
                        theme.surface_bg
                    };
                    ui.painter().rect(
                        comment_rect,
                        Rounding::same(7.0),
                        comment_bg,
                        Stroke::new(1.0_f32, theme.border_color),
                    );
                    let comment_icon_r = Rect::from_min_size(
                        Pos2::new(comment_rect.min.x + 10.0, comment_rect.center().y - 7.0),
                        Vec2::splat(14.0),
                    );
                    crate::icons::draw_icon(
                        ui.painter(),
                        comment_icon_r,
                        IconKind::Comment,
                        theme.text_main,
                    );
                    ui.painter().text(
                        Pos2::new(comment_rect.min.x + 28.0, comment_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        "Bình luận",
                        FontId::proportional(12.0),
                        theme.text_main,
                    );
                    if comment_resp
                        .on_hover_text("Xem và thêm bình luận")
                        .clicked()
                    {
                        app.set_status("Không có bình luận nào");
                    }

                    // Nút CTA Lưu chính
                    let save_cta = egui::Button::new(
                        egui::RichText::new("Lưu tệp")
                            .size(12.0)
                            .color(theme.text_main),
                    )
                    .fill(theme.surface_bg)
                    .stroke(Stroke::new(1.0_f32, theme.border_color))
                    .rounding(Rounding::same(7.0))
                    .min_size(Vec2::new(70.0, 30.0));
                    if ui
                        .add(save_cta)
                        .on_hover_text("Lưu tài liệu (Ctrl+S / Ctrl+Shift+S)")
                        .clicked()
                    {
                        app.trigger_save(ctx.clone(), false);
                    }

                    // Nút Mở tệp viền mảnh
                    let open_btn = egui::Button::new(
                        egui::RichText::new("Mở tệp")
                            .size(12.0)
                            .color(theme.text_main),
                    )
                    .fill(theme.surface_bg)
                    .stroke(Stroke::new(1.0_f32, theme.border_color))
                    .rounding(Rounding::same(7.0))
                    .min_size(Vec2::new(66.0, 30.0));
                    if ui
                        .add(open_btn)
                        .on_hover_text("Mở tệp .mwd / .doc / .txt / .md (Ctrl+O)")
                        .clicked()
                    {
                        app.trigger_open_dialog(ctx.clone());
                    }

                    let theme_icon = if theme.is_dark {
                        IconKind::Sun
                    } else {
                        IconKind::Moon
                    };
                    let theme_tip = if theme.is_dark {
                        "Chuyển sang nền sáng"
                    } else {
                        "Chuyển sang nền tối"
                    };
                    if icon_button(ui, theme_icon, theme_tip, false, theme.is_dark).clicked() {
                        app.dark_workspace = !app.dark_workspace;
                        ThemePalette::from_mode(app.dark_workspace).apply(ctx);
                    }

                    // Nút chọn chế độ Telex
                    let (mode_label, mode_bg, mode_border, mode_fg) = match app.editor.input_mode {
                        InputMethodMode::AutoTelex => (
                            "Telex: Tự động",
                            if theme.is_dark {
                                Color32::from_rgb(20, 48, 30)
                            } else {
                                Color32::from_rgb(240, 253, 244)
                            },
                            if theme.is_dark {
                                Color32::from_rgb(34, 197, 94)
                            } else {
                                Color32::from_rgb(187, 247, 208)
                            },
                            if theme.is_dark {
                                Color32::from_rgb(134, 239, 172)
                            } else {
                                Color32::from_rgb(21, 128, 61)
                            },
                        ),
                        InputMethodMode::SystemOnly => (
                            "Telex: Hệ thống",
                            theme.accent_tint,
                            theme.accent_border,
                            theme.primary_accent,
                        ),
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(format!("● {}", mode_label))
                                    .size(11.0)
                                    .color(mode_fg)
                                    .strong(),
                            )
                            .fill(mode_bg)
                            .stroke(Stroke::new(1.0_f32, mode_border))
                            .rounding(Rounding::same(14.0))
                            .min_size(Vec2::new(0.0, 26.0)),
                        )
                        .on_hover_text("Nhấn để chuyển giữa bộ gõ Telex tự động và thuần Hệ thống")
                        .clicked()
                    {
                        app.editor.input_mode = match app.editor.input_mode {
                            InputMethodMode::AutoTelex => InputMethodMode::SystemOnly,
                            InputMethodMode::SystemOnly => InputMethodMode::AutoTelex,
                        };
                    }
                });
            });
        });
}

fn ribbon_tab_button(ui: &mut egui::Ui, label: &str, selected: bool, theme: &ThemePalette) -> bool {
    let text_color = if selected {
        theme.primary_accent
    } else {
        theme.text_muted
    };
    let text = if selected {
        egui::RichText::new(label)
            .size(12.5)
            .strong()
            .color(text_color)
    } else {
        egui::RichText::new(label).size(12.5).color(text_color)
    };
    let resp = ui.add(
        egui::Button::new(text)
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .min_size(Vec2::new(56.0, 28.0)),
    );
    if selected {
        let r = resp.rect;
        ui.painter().rect_filled(
            Rect::from_min_max(
                Pos2::new(r.min.x + 6.0, r.max.y - 2.5),
                Pos2::new(r.max.x - 6.0, r.max.y),
            ),
            Rounding::same(1.5),
            theme.primary_accent,
        );
    }
    resp.clicked()
}

pub fn render_ribbon_bar(ctx: &egui::Context, app: &mut MyWordApp, theme: &ThemePalette) {
    let is_dark = theme.is_dark;
    egui::TopBottomPanel::top("ribbon_panel")
        .frame(
            egui::Frame::none()
                .fill(theme.toolbar_bg)
                .inner_margin(egui::Margin::symmetric(16.0, 0.0))
                .stroke(Stroke::new(1.0_f32, theme.border_color)),
        )
        .show(ctx, |ui| {
            // Hàng 1: Các tab điều hướng Ribbon (Tệp / Trang đầu / Chèn / Bố trí / Hiển thị)
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
                for (tab, label) in [
                    (RibbonTab::File, "Tệp"),
                    (RibbonTab::Home, "Trang đầu"),
                    (RibbonTab::Insert, "Chèn"),
                    (RibbonTab::Layout, "Bố trí"),
                    (RibbonTab::References, "Tham chiếu"),
                    (RibbonTab::Review, "Xem lại"),
                    (RibbonTab::View, "Hiển thị"),
                ] {
                    if ribbon_tab_button(ui, label, app.active_ribbon_tab == tab, theme) {
                        app.active_ribbon_tab = tab;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    if !app.focus_mode {
                        if icon_button(
                            ui,
                            IconKind::PanelRight,
                            "Ẩn / Hiện bảng Định dạng phải",
                            app.show_inspector_panel,
                            is_dark,
                        )
                        .clicked()
                        {
                            app.show_inspector_panel = !app.show_inspector_panel;
                        }
                        if icon_button(
                            ui,
                            IconKind::PanelLeft,
                            "Ẩn / Hiện bảng Điều hướng trái",
                            app.show_nav_panel,
                            is_dark,
                        )
                        .clicked()
                        {
                            app.show_nav_panel = !app.show_nav_panel;
                        }
                    }
                    ui.label(
                        egui::RichText::new("Đang chỉnh sửa")
                            .size(11.5)
                            .color(theme.text_muted),
                    );
                });
            });

            // Đường kẻ phân chia giữa Tab và Công cụ Ribbon
            let sep_rect = ui.available_rect_before_wrap();
            ui.painter().line_segment(
                [
                    Pos2::new(sep_rect.min.x - 16.0, sep_rect.min.y),
                    Pos2::new(sep_rect.max.x + 16.0, sep_rect.min.y),
                ],
                Stroke::new(1.0_f32, theme.border_color),
            );
            ui.add_space(6.0);

            // Hàng 2: Nội dung công cụ theo Tab đang chọn
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);
                ui.style_mut().spacing.interact_size.y = 26.0;

                match app.active_ribbon_tab {
                    RibbonTab::Home => render_home_ribbon_tools(ui, ctx, app, theme),
                    RibbonTab::File => render_file_ribbon_tools(ui, ctx, app, theme),
                    RibbonTab::Insert => render_insert_ribbon_tools(ui, app, theme),
                    RibbonTab::Layout => render_layout_ribbon_tools(ui, app, theme),
                    RibbonTab::References => render_references_ribbon_tools(ui, app, theme),
                    RibbonTab::Review => render_review_ribbon_tools(ui, app, theme),
                    RibbonTab::View => render_view_ribbon_tools(ui, ctx, app, theme),
                }

                // Nút Chế độ tập trung (Focus mode) luôn nằm bên phải
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let focus_bg = if app.focus_mode {
                        theme.accent_tint
                    } else {
                        theme.input_bg
                    };
                    let focus_stroke = if app.focus_mode {
                        Stroke::new(1.0_f32, theme.primary_accent)
                    } else {
                        Stroke::new(1.0_f32, theme.border_color)
                    };
                    let focus_fg = if app.focus_mode {
                        theme.primary_accent
                    } else {
                        theme.text_main
                    };
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(if app.focus_mode {
                                    "Thoát tập trung"
                                } else {
                                    "Chế độ tập trung"
                                })
                                .size(11.5)
                                .color(focus_fg)
                                .strong(),
                            )
                            .fill(focus_bg)
                            .stroke(focus_stroke)
                            .rounding(Rounding::same(7.0))
                            .min_size(Vec2::new(124.0, 34.0)),
                        )
                        .on_hover_text("Ẩn thanh điều hướng hai bên để tập trung soạn thảo")
                        .clicked()
                    {
                        app.focus_mode = !app.focus_mode;
                    }
                });
            });
            ui.add_space(6.0);
        });
}

fn render_home_ribbon_tools(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app: &mut MyWordApp,
    theme: &ThemePalette,
) {
    let is_dark = theme.is_dark;

    // Nhóm 1: Hoàn tác / Làm lại
    ui.add_enabled_ui(!app.editor.undo_stack.is_empty(), |ui| {
        if icon_button(ui, IconKind::Undo, "Hoàn tác (Ctrl+Z)", false, is_dark).clicked()
            && app.editor.undo()
        {
            app.set_status("Đã hoàn tác");
        }
    });
    ui.add_enabled_ui(!app.editor.redo_stack.is_empty(), |ui| {
        if icon_button(ui, IconKind::Redo, "Làm lại (Ctrl+Y)", false, is_dark).clicked()
            && app.editor.redo()
        {
            app.set_status("Đã làm lại");
        }
    });
    vertical_divider(ui, 38.0, theme.border_color);

    // Nhóm 2: Bảng tạm
    let has_selection = app.editor.selected_range().is_some();
    ui.add_enabled_ui(has_selection, |ui| {
        if icon_button(ui, IconKind::Cut, "Cắt (Ctrl+X)", false, is_dark).clicked() {
            app.cut_selection_to_clipboard(ctx);
        }
        if icon_button(ui, IconKind::Copy, "Sao chép (Ctrl+C)", false, is_dark).clicked() {
            app.copy_selection_to_clipboard(ctx);
        }
    });
    ui.add_enabled_ui(!app.editor.clipboard_buffer.is_empty(), |ui| {
        if icon_button(ui, IconKind::Paste, "Dán (Ctrl+V)", false, is_dark).clicked() {
            let text = app.editor.clipboard_buffer.clone();
            app.editor.insert_text(&text);
        }
    });
    let fp_active = app.editor.format_painter_style.is_some();
    if icon_button(
        ui,
        IconKind::FormatPainter,
        "Sao chép định dạng (Format Painter)",
        fp_active,
        is_dark,
    )
    .clicked()
    {
        app.editor.toggle_format_painter();
        if app.editor.format_painter_style.is_some() {
            app.set_status("Đã chép định dạng — quét chuột lên chữ để áp dụng");
        } else {
            app.set_status("Đã hủy sao chép định dạng");
        }
    }
    vertical_divider(ui, 38.0, theme.border_color);

    // Nhóm 3: Phông chữ & Định dạng Rich Text
    if ui
        .add(
            egui::Button::new(egui::RichText::new("−").size(13.0))
                .fill(theme.input_bg)
                .rounding(5.0),
        )
        .on_hover_text("Giảm cỡ chữ")
        .clicked()
    {
        let next = (app.editor.active_style.font_size - 1.0).max(8.0);
        app.editor.apply_format(|s| s.font_size = next);
    }
    ui.label(
        egui::RichText::new(format!("{:.0} pt", app.editor.active_style.font_size))
            .size(12.0)
            .color(theme.text_main)
            .strong(),
    );
    if ui
        .add(
            egui::Button::new(egui::RichText::new("+").size(13.0))
                .fill(theme.input_bg)
                .rounding(5.0),
        )
        .on_hover_text("Tăng cỡ chữ")
        .clicked()
    {
        let next = (app.editor.active_style.font_size + 1.0).min(48.0);
        app.editor.apply_format(|s| s.font_size = next);
    }

    let style_toggle = |ui: &mut egui::Ui, label: egui::RichText, active: bool, tip: &str| {
        let mut btn = egui::Button::new(label)
            .min_size(Vec2::new(26.0, 26.0))
            .rounding(5.0);
        if active {
            btn = btn
                .fill(theme.accent_tint)
                .stroke(Stroke::new(1.0_f32, theme.accent_border));
        } else {
            btn = btn.fill(Color32::TRANSPARENT).stroke(Stroke::NONE);
        }
        ui.add(btn).on_hover_text(tip).clicked()
    };

    if style_toggle(
        ui,
        egui::RichText::new("B")
            .strong()
            .size(13.0)
            .color(if app.editor.active_style.bold {
                theme.primary_accent
            } else {
                theme.text_main
            }),
        app.editor.active_style.bold,
        "In đậm (Ctrl+B)",
    ) {
        let next = !app.editor.active_style.bold;
        app.editor.apply_format(|s| s.bold = next);
    }
    if style_toggle(
        ui,
        egui::RichText::new("I")
            .italics()
            .size(13.0)
            .color(if app.editor.active_style.italic {
                theme.primary_accent
            } else {
                theme.text_main
            }),
        app.editor.active_style.italic,
        "In nghiêng (Ctrl+I)",
    ) {
        let next = !app.editor.active_style.italic;
        app.editor.apply_format(|s| s.italic = next);
    }
    if style_toggle(
        ui,
        egui::RichText::new("U").underline().size(13.0).color(
            if app.editor.active_style.underline {
                theme.primary_accent
            } else {
                theme.text_main
            },
        ),
        app.editor.active_style.underline,
        "Gạch chân (Ctrl+U)",
    ) {
        let next = !app.editor.active_style.underline;
        app.editor.apply_format(|s| s.underline = next);
    }
    if style_toggle(
        ui,
        egui::RichText::new("S").strikethrough().size(13.0).color(
            if app.editor.active_style.strikethrough {
                theme.primary_accent
            } else {
                theme.text_main
            },
        ),
        app.editor.active_style.strikethrough,
        "Gạch ngang chữ",
    ) {
        let next = !app.editor.active_style.strikethrough;
        app.editor.apply_format(|s| s.strikethrough = next);
    }
    if icon_button(
        ui,
        IconKind::ClearFormatting,
        "Xóa toàn bộ định dạng về mặc định",
        false,
        is_dark,
    )
    .clicked()
    {
        app.editor.clear_formatting();
        app.set_status("Đã đưa chữ về định dạng mặc định");
    }

    for (rgb, tip) in [
        ([35, 43, 60], "Màu mặc định"),
        ([64, 89, 220], "Màu xanh thương hiệu"),
        ([220, 38, 38], "Màu đỏ"),
    ] {
        let selected = app.editor.active_style.color == rgb;
        let (c_rect, c_resp) = ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::click());
        ui.painter().circle_filled(
            c_rect.center(),
            5.5,
            Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
        );
        if selected {
            ui.painter().circle_stroke(
                c_rect.center(),
                7.5,
                Stroke::new(1.5_f32, theme.primary_accent),
            );
        }
        if c_resp.on_hover_text(tip).clicked() {
            app.editor.apply_format(|s| s.color = rgb);
        }
    }

    let hl_on = app.editor.active_style.highlight.is_some();
    if icon_button(ui, IconKind::Highlight, "Tô sáng Highlight", hl_on, is_dark).clicked() {
        let next_hl = if hl_on { None } else { Some([244, 215, 107]) };
        app.editor.apply_format(|s| s.highlight = next_hl);
    }
    vertical_divider(ui, 38.0, theme.border_color);

    // Nhóm 4: Đoạn văn (Căn lề & Danh sách)
    let cur_align = app.editor.current_alignment();
    for (align, icon, tip) in [
        (TextAlignment::Left, IconKind::AlignLeft, "Căn trái"),
        (TextAlignment::Center, IconKind::AlignCenter, "Căn giữa"),
        (TextAlignment::Right, IconKind::AlignRight, "Căn phải"),
        (
            TextAlignment::Justify,
            IconKind::AlignJustify,
            "Căn đều 2 bên",
        ),
    ] {
        if icon_button(ui, icon, tip, cur_align == align, is_dark).clicked() {
            app.editor.set_alignment(align);
        }
    }
    if icon_button(
        ui,
        IconKind::ListBullet,
        "Danh sách gạch đầu dòng (•)",
        false,
        is_dark,
    )
    .clicked()
    {
        app.editor.insert_bullet_item();
    }
    if icon_button(
        ui,
        IconKind::ListNumber,
        "Danh sách đánh số thứ tự (1, 2, 3)",
        false,
        is_dark,
    )
    .clicked()
    {
        app.editor.insert_numbered_item();
    }
    vertical_divider(ui, 38.0, theme.border_color);

    // Nhóm 5: Thẻ Kiểu văn bản kiểu Figma (Aa Thông thường, Aa Tiêu đề 1, Aa Tiêu đề 2...)
    for preset in [
        DocumentPresetStyle::Normal,
        DocumentPresetStyle::Heading1,
        DocumentPresetStyle::Heading2,
        DocumentPresetStyle::Heading3,
    ] {
        let is_active = is_preset_active(&app.editor.active_style, preset);
        let fill = if is_active {
            theme.accent_tint
        } else {
            theme.card_bg
        };
        let stroke = Stroke::new(
            1.0_f32,
            if is_active {
                theme.accent_border
            } else {
                theme.border_color
            },
        );
        let (card_rect, resp) = ui.allocate_exact_size(Vec2::new(80.0, 42.0), egui::Sense::click());
        ui.painter()
            .rect(card_rect, Rounding::same(6.0), fill, stroke);
        let aa_col = if is_active {
            theme.primary_accent
        } else {
            theme.text_main
        };
        let label_col = if is_active {
            theme.primary_accent
        } else {
            theme.text_muted
        };
        ui.painter().text(
            Pos2::new(card_rect.min.x + 8.0, card_rect.min.y + 6.0),
            egui::Align2::LEFT_TOP,
            "Aa",
            FontId::proportional(15.0),
            aa_col,
        );
        ui.painter().text(
            Pos2::new(card_rect.min.x + 8.0, card_rect.max.y - 6.0),
            egui::Align2::LEFT_BOTTOM,
            preset.label(),
            FontId::proportional(9.5),
            label_col,
        );
        if resp
            .on_hover_text(format!("Áp dụng kiểu {}", preset.label()))
            .clicked()
        {
            app.editor.apply_preset_style(preset);
            app.set_status(&format!("Đã áp dụng: {}", preset.label()));
        }
    }
    vertical_divider(ui, 38.0, theme.border_color);

    // Nhóm 6: Chèn nhanh
    if icon_button(
        ui,
        IconKind::PageBreak,
        "Chèn ngắt trang (Ctrl+Enter)",
        false,
        is_dark,
    )
    .clicked()
    {
        app.editor.insert_text(&PAGE_BREAK_CHAR.to_string());
        app.set_status("Đã chèn ngắt trang");
    }
    if icon_button(
        ui,
        IconKind::Search,
        "Tìm & Thay thế (Ctrl+F / Ctrl+H)",
        app.find_bar_open,
        is_dark,
    )
    .clicked()
    {
        app.find_bar_open = !app.find_bar_open;
        if app.find_bar_open {
            app.focus_find_input = true;
        }
    }
}

fn render_file_ribbon_tools(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app: &mut MyWordApp,
    theme: &ThemePalette,
) {
    let pill_btn = |ui: &mut egui::Ui, text: &str, primary: bool| {
        let mut btn = egui::Button::new(egui::RichText::new(text).size(12.0).color(if primary {
            Color32::WHITE
        } else {
            theme.text_main
        }))
        .rounding(Rounding::same(6.0))
        .min_size(Vec2::new(0.0, 32.0));
        if primary {
            btn = btn.fill(theme.primary_accent);
        } else {
            btn = btn
                .fill(theme.card_bg)
                .stroke(Stroke::new(1.0_f32, theme.border_color));
        }
        ui.add(btn).clicked()
    };

    if pill_btn(ui, "Tài liệu mới (Ctrl+N)", false) {
        app.trigger_new_document();
    }
    if pill_btn(ui, "Mở tệp... (Ctrl+O)", false) {
        app.trigger_open_dialog(ctx.clone());
    }
    if pill_btn(ui, "Lưu tệp (Ctrl+S)", true) {
        app.trigger_save(ctx.clone(), false);
    }
    if pill_btn(ui, "Lưu thành... (Ctrl+Shift+S)", false) {
        app.trigger_save(ctx.clone(), true);
    }
    if pill_btn(ui, "Màn hình chào mừng", false) {
        app.show_startup_modal = true;
    }
}

fn render_insert_ribbon_tools(ui: &mut egui::Ui, app: &mut MyWordApp, theme: &ThemePalette) {
    let pill_btn = |ui: &mut egui::Ui, text: &str| {
        ui.add(
            egui::Button::new(egui::RichText::new(text).size(12.0).color(theme.text_main))
                .fill(theme.card_bg)
                .stroke(Stroke::new(1.0_f32, theme.border_color))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(0.0, 32.0)),
        )
        .clicked()
    };

    if pill_btn(ui, "Ngắt trang (Ctrl+Enter)") {
        app.editor.insert_text(&PAGE_BREAK_CHAR.to_string());
        app.set_status("Đã chèn ngắt trang");
    }
    if pill_btn(ui, "Gạch đầu dòng (•)") {
        app.editor.insert_bullet_item();
    }
    if pill_btn(ui, "Danh sách số (1. 2. 3.)") {
        app.editor.insert_numbered_item();
    }
    if pill_btn(ui, "Đường phân cách") {
        app.editor
            .insert_text("\n────────────────────────────────────\n");
    }
}

fn render_layout_ribbon_tools(ui: &mut egui::Ui, app: &mut MyWordApp, theme: &ThemePalette) {
    let toggle_btn = |ui: &mut egui::Ui, text: &str, active: bool| {
        ui.add(
            egui::Button::new(egui::RichText::new(text).size(12.0).color(if active {
                theme.primary_accent
            } else {
                theme.text_main
            }))
            .fill(if active {
                theme.accent_tint
            } else {
                theme.card_bg
            })
            .stroke(Stroke::new(
                1.0_f32,
                if active {
                    theme.accent_border
                } else {
                    theme.border_color
                },
            ))
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(0.0, 32.0)),
        )
        .clicked()
    };

    if toggle_btn(ui, "Thước kẻ A4", app.show_ruler) {
        app.show_ruler = !app.show_ruler;
    }
    if toggle_btn(ui, "Góc căn lề trang", app.show_margins) {
        app.show_margins = !app.show_margins;
    }
    vertical_divider(ui, 32.0, theme.border_color);
    let cur = app.editor.current_alignment();
    for (align, label) in [
        (TextAlignment::Left, "Căn trái"),
        (TextAlignment::Center, "Căn giữa"),
        (TextAlignment::Right, "Căn phải"),
        (TextAlignment::Justify, "Căn đều"),
    ] {
        if toggle_btn(ui, label, cur == align) {
            app.editor.set_alignment(align);
        }
    }
}

fn render_view_ribbon_tools(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    app: &mut MyWordApp,
    theme: &ThemePalette,
) {
    let toggle_btn = |ui: &mut egui::Ui, text: &str, active: bool| {
        ui.add(
            egui::Button::new(egui::RichText::new(text).size(12.0).color(if active {
                theme.primary_accent
            } else {
                theme.text_main
            }))
            .fill(if active {
                theme.accent_tint
            } else {
                theme.card_bg
            })
            .stroke(Stroke::new(
                1.0_f32,
                if active {
                    theme.accent_border
                } else {
                    theme.border_color
                },
            ))
            .rounding(Rounding::same(6.0))
            .min_size(Vec2::new(0.0, 32.0)),
        )
        .clicked()
    };

    if toggle_btn(ui, "Bảng Điều hướng", app.show_nav_panel && !app.focus_mode) {
        app.focus_mode = false;
        app.show_nav_panel = !app.show_nav_panel;
    }
    if toggle_btn(
        ui,
        "Bảng Định dạng",
        app.show_inspector_panel && !app.focus_mode,
    ) {
        app.focus_mode = false;
        app.show_inspector_panel = !app.show_inspector_panel;
    }
    if toggle_btn(ui, "Giao diện Tối", app.dark_workspace) {
        app.dark_workspace = !app.dark_workspace;
        ThemePalette::from_mode(app.dark_workspace).apply(ctx);
    }
    vertical_divider(ui, 32.0, theme.border_color);
    for (z, label) in [(0.75, "75%"), (1.0, "100%"), (1.25, "125%"), (1.5, "150%")] {
        if toggle_btn(ui, label, (app.zoom - z).abs() < 0.05) {
            app.set_zoom(z);
        }
    }
}

fn render_references_ribbon_tools(ui: &mut egui::Ui, app: &mut MyWordApp, theme: &ThemePalette) {
    let pill_btn = |ui: &mut egui::Ui, text: &str| {
        ui.add(
            egui::Button::new(egui::RichText::new(text).size(12.0).color(theme.text_main))
                .fill(theme.card_bg)
                .stroke(Stroke::new(1.0_f32, theme.border_color))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(0.0, 32.0)),
        )
        .clicked()
    };

    if pill_btn(ui, "📑 Bảng Mục lục") {
        app.show_nav_panel = true;
        app.nav_tab = NavTab::Outline;
        app.set_status("Đã mở bảng mục lục điều hướng");
    }
    if pill_btn(ui, "Chèn Chú thích") {
        app.editor.insert_text(" [1]");
        app.set_status("Đã chèn ký hiệu chú thích");
    }
    if pill_btn(ui, "Thêm Trích dẫn") {
        app.editor.insert_text(" (Nguồn: Studio North, 2026)");
        app.set_status("Đã chèn trích dẫn tài liệu");
    }
    if pill_btn(ui, "Quản lý Nguồn") {
        app.set_status("Quản lý danh mục nguồn tài liệu tham khảo");
    }
}

fn render_review_ribbon_tools(ui: &mut egui::Ui, app: &mut MyWordApp, theme: &ThemePalette) {
    let pill_btn = |ui: &mut egui::Ui, text: &str| {
        ui.add(
            egui::Button::new(egui::RichText::new(text).size(12.0).color(theme.text_main))
                .fill(theme.card_bg)
                .stroke(Stroke::new(1.0_f32, theme.border_color))
                .rounding(Rounding::same(6.0))
                .min_size(Vec2::new(0.0, 32.0)),
        )
        .clicked()
    };

    if pill_btn(ui, "🔍 Kiểm tra chính tả") {
        app.set_status("Không phát hiện lỗi chính tả tiếng Việt");
    }
    if pill_btn(ui, "📊 Danh sách trang") {
        app.show_nav_panel = true;
        app.nav_tab = NavTab::Pages;
        app.set_status("Đã mở danh sách trang tài liệu");
    }
    if pill_btn(ui, "💬 Thêm bình luận") {
        app.set_status("Đã tạo ghi chú / bình luận mới tại con trỏ");
    }
    if pill_btn(ui, "📝 Theo dõi thay đổi") {
        app.set_status("Chế độ theo dõi lịch sử chỉnh sửa đang bật");
    }
}

fn is_preset_active(style: &crate::document::TextStyle, preset: DocumentPresetStyle) -> bool {
    match preset {
        DocumentPresetStyle::Normal => (style.font_size - 14.0).abs() < 0.5 && !style.bold,
        DocumentPresetStyle::Heading1 => (style.font_size - 22.0).abs() < 0.5 && style.bold,
        DocumentPresetStyle::Heading2 => (style.font_size - 18.0).abs() < 0.5 && style.bold,
        DocumentPresetStyle::Heading3 => (style.font_size - 16.0).abs() < 0.5 && style.bold,
        DocumentPresetStyle::Quote => style.italic,
    }
}

pub fn render_find_replace_bar(ctx: &egui::Context, app: &mut MyWordApp, theme: &ThemePalette) {
    if !app.find_bar_open {
        return;
    }
    app.refresh_find_cache();

    egui::TopBottomPanel::top("find_replace_panel")
        .frame(
            egui::Frame::none()
                .fill(theme.surface_bg)
                .inner_margin(egui::Margin::symmetric(16.0, 6.0))
                .stroke(Stroke::new(1.0_f32, theme.border_color)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Tìm:")
                        .size(12.5)
                        .color(theme.text_main),
                );
                let find_resp = ui.add(
                    egui::TextEdit::singleline(&mut app.find_query)
                        .desired_width(180.0)
                        .hint_text("Nhập từ khóa..."),
                );
                if find_resp.changed() {
                    app.editor.find_dirty = true;
                }
                if (find_resp.lost_focus() || find_resp.has_focus())
                    && ctx.input(|i| i.key_pressed(egui::Key::Enter))
                    && !app.cached_matches.is_empty()
                {
                    app.active_match_idx = (app.active_match_idx + 1) % app.cached_matches.len();
                    let (s, e) = app.cached_matches[app.active_match_idx];
                    app.editor.selection_start = Some(s);
                    app.editor.caret_byte_offset = e;
                    app.scroll_to_caret = true;
                }
                if app.focus_find_input {
                    find_resp.request_focus();
                    app.focus_find_input = false;
                }

                let case_btn = if app.find_case_sensitive {
                    let fill_col = if theme.is_dark {
                        Color32::from_rgb(45, 55, 75)
                    } else {
                        Color32::from_rgb(238, 242, 255)
                    };
                    let text_col = if theme.is_dark {
                        Color32::from_rgb(165, 180, 252)
                    } else {
                        theme.primary_accent
                    };
                    egui::Button::new(egui::RichText::new("Aa").size(12.0).color(text_col))
                        .rounding(5.0)
                        .fill(fill_col)
                        .stroke(Stroke::new(1.0_f32, theme.primary_accent))
                } else {
                    egui::Button::new(egui::RichText::new("Aa").size(12.0)).rounding(5.0)
                };
                if ui
                    .add(case_btn)
                    .on_hover_text("Phân biệt hoa / thường")
                    .clicked()
                {
                    app.find_case_sensitive = !app.find_case_sensitive;
                    app.editor.find_dirty = true;
                }

                let count_str = if app.cached_matches.is_empty() {
                    if app.find_query.is_empty() {
                        "0 kết quả".to_string()
                    } else {
                        "Không thấy".to_string()
                    }
                } else {
                    format!("{}/{}", app.active_match_idx + 1, app.cached_matches.len())
                };
                ui.label(
                    egui::RichText::new(count_str)
                        .size(12.0)
                        .color(theme.text_muted),
                );

                if ui.button("Trước").clicked() && !app.cached_matches.is_empty() {
                    app.active_match_idx = if app.active_match_idx == 0 {
                        app.cached_matches.len() - 1
                    } else {
                        app.active_match_idx - 1
                    };
                    let (s, e) = app.cached_matches[app.active_match_idx];
                    app.editor.selection_start = Some(s);
                    app.editor.caret_byte_offset = e;
                }
                if ui.button("Sau").clicked() && !app.cached_matches.is_empty() {
                    app.active_match_idx = (app.active_match_idx + 1) % app.cached_matches.len();
                    let (s, e) = app.cached_matches[app.active_match_idx];
                    app.editor.selection_start = Some(s);
                    app.editor.caret_byte_offset = e;
                }

                ui.separator();
                if !app.replace_bar_open {
                    if ui.button("Thay thế...").clicked() {
                        app.replace_bar_open = true;
                    }
                } else {
                    ui.label(
                        egui::RichText::new("Thay:")
                            .size(12.5)
                            .color(theme.text_main),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut app.replace_query)
                            .desired_width(160.0)
                            .hint_text("Chuỗi mới..."),
                    );
                    if ui.button("Thay thế").clicked()
                        && app.editor.replace_current_match(
                            &app.find_query,
                            &app.replace_query,
                            app.find_case_sensitive,
                            app.active_match_idx,
                        )
                    {
                        app.refresh_find_cache();
                        app.set_status("Đã thay thế 1 kết quả");
                    }
                    if ui.button("Thay tất cả").clicked() {
                        let count = app.editor.replace_all_matches(
                            &app.find_query,
                            &app.replace_query,
                            app.find_case_sensitive,
                        );
                        app.refresh_find_cache();
                        if count > 0 {
                            app.set_status(&format!("Đã thay thế {} kết quả", count));
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Đóng (Esc)").clicked() {
                        app.find_bar_open = false;
                        app.replace_bar_open = false;
                    }
                });
            });
        });
}

pub fn render_recovery_banner(ctx: &egui::Context, app: &mut MyWordApp, theme: &ThemePalette) {
    if !app.show_recovery_banner {
        return;
    }
    egui::TopBottomPanel::top("recovery_banner")
        .frame(
            egui::Frame::none()
                .fill(if theme.is_dark {
                    Color32::from_rgb(55, 40, 10)
                } else {
                    Color32::from_rgb(254, 243, 199)
                })
                .inner_margin(egui::Margin::symmetric(16.0, 8.0))
                .stroke(Stroke::new(
                    1.0_f32,
                    if theme.is_dark {
                        Color32::from_rgb(180, 130, 30)
                    } else {
                        Color32::from_rgb(245, 158, 11)
                    },
                )),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(
                        "Phát hiện bản sao lưu tự động chưa được lưu từ phiên làm việc trước.",
                    )
                    .size(12.5)
                    .color(if theme.is_dark {
                        Color32::from_rgb(253, 230, 138)
                    } else {
                        Color32::from_rgb(146, 64, 14)
                    })
                    .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Bỏ qua").clicked() {
                        app.discard_autosave();
                    }
                    if ui
                        .button(egui::RichText::new("Khôi phục").strong())
                        .clicked()
                    {
                        app.recover_autosave();
                    }
                });
            });
        });
}

pub fn render_confirm_modal(ctx: &egui::Context, app: &mut MyWordApp, theme: &ThemePalette) {
    let Some(action) = app.pending_confirm else {
        return;
    };

    let mut close_modal = false;
    let mut do_save = false;
    let mut do_discard = false;

    egui::Window::new("Xác nhận lưu tài liệu")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(theme.surface_bg)
                .stroke(Stroke::new(1.0_f32, theme.border_color))
                .inner_margin(egui::Margin::same(16.0)),
        )
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new("Tài liệu hiện tại có thay đổi chưa được lưu.")
                        .size(14.5)
                        .strong()
                        .color(theme.text_main),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Bạn có muốn lưu các thay đổi trước khi tiếp tục không?")
                        .size(13.0)
                        .color(theme.text_muted),
                );
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Hủy").clicked() {
                            close_modal = true;
                        }
                        if ui.button("Không lưu").clicked() {
                            do_discard = true;
                        }
                        if ui
                            .button(egui::RichText::new("Lưu").color(Color32::WHITE).strong())
                            .clicked()
                        {
                            do_save = true;
                        }
                    });
                });
            });
        });

    if close_modal {
        app.pending_confirm = None;
        app.cleanup_pending_file_ops();
    } else if do_discard {
        app.pending_confirm = None;
        app.editor.is_dirty = false;
        match action {
            crate::app::ConfirmAction::Quit => {
                app.cleanup_pending_file_ops();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            crate::app::ConfirmAction::OpenNew => {
                if let Some(drop_path) = app.pending_drop_path.take() {
                    app.load_from_path(&drop_path);
                } else {
                    app.trigger_open_dialog(ctx.clone());
                }
                app.cleanup_pending_file_ops();
            }
            crate::app::ConfirmAction::NewDocument => {
                app.cleanup_pending_file_ops();
                app.new_document();
            }
        }
    } else if do_save {
        app.pending_confirm = None;
        if let Some(path) = app.current_file_path.clone() {
            if app.write_to_path(&path) {
                app.post_save_action = Some(action);
                app.execute_post_save_action(ctx);
            } else {
                app.cleanup_pending_file_ops();
            }
        } else {
            app.post_save_action = Some(action);
            app.file_dialog_rx = Some(crate::platform::spawn_save_dialog(ctx.clone()));
        }
    }
}

pub fn build_brand_plan_sample() -> crate::editor::EditorState {
    use crate::document::{StyleSpan, TextStyle};

    let meta = TextStyle {
        bold: true,
        font_size: 10.5,
        color: [64, 89, 220],
        ..Default::default()
    };
    let title = TextStyle {
        bold: true,
        font_size: 32.0,
        color: [35, 43, 60],
        ..Default::default()
    };
    let subtitle = TextStyle {
        font_size: 14.0,
        color: [101, 112, 135],
        ..Default::default()
    };
    let meta_gray = TextStyle {
        font_size: 10.0,
        color: [101, 112, 135],
        ..Default::default()
    };
    let h1 = TextStyle {
        bold: true,
        font_size: 20.0,
        color: [35, 43, 60],
        ..Default::default()
    };
    let h2 = TextStyle {
        bold: true,
        font_size: 14.5,
        color: [35, 43, 60],
        ..Default::default()
    };
    let quote = TextStyle {
        bold: true,
        italic: true,
        font_size: 13.5,
        color: [64, 89, 220],
        ..Default::default()
    };
    let body = TextStyle {
        font_size: 12.5,
        color: [35, 43, 60],
        ..Default::default()
    };
    let bullet = TextStyle {
        font_size: 12.0,
        color: [55, 65, 81],
        ..Default::default()
    };

    let sections: [(&str, TextStyle); 20] = [
        (
            "STUDIO NORTH  /  CHIẾN LƯỢC THƯƠNG HIỆU                                             2026\n",
            meta,
        ),
        (
            "──────────────────────────────────────────────────────────────────────────\n\n",
            meta_gray,
        ),
        ("Kế hoạch\nthương hiệu 2026\n\n", title),
        (
            "Xây dựng bản sắc. Kết nối giá trị. Tạo dấu ấn.\n\n",
            subtitle,
        ),
        (
            "NHÓM MARKETING    •    18 tháng 11, 2026                 [ BẢN NHÁP · NỘI BỘ ]\n",
            meta_gray,
        ),
        (
            "──────────────────────────────────────────────────────────────────────────\n\n",
            meta_gray,
        ),
        ("01  Tầm nhìn & định hướng\n", h1),
        (
            "Một thương hiệu tốt không chỉ được nhìn thấy, mà còn được ghi nhớ. Năm 2026, chúng ta tập trung xây dựng trải nghiệm nhất quán, chân thật và có ý nghĩa tại mọi điểm chạm với khách hàng.\n\n",
            body,
        ),
        (
            "▌ ĐỊNH HƯỚNG CỦA CHÚNG TA\n  \"Biến những ý tưởng khác biệt thành giá trị bền vững.\"\n\n",
            quote,
        ),
        ("Ba giá trị cốt lõi\n", h2),
        (
            "• 01. Chân thật: Nhất quán trong lời nói và hành động trong mọi hoàn cảnh.\n",
            bullet,
        ),
        (
            "• 02. Sáng tạo: Luôn tìm một góc nhìn mới cho mỗi bài toán sản phẩm.\n",
            bullet,
        ),
        (
            "• 03. Đồng hành: Lấy con người làm trung tâm phát triển bền vững.\n\n",
            bullet,
        ),
        ("02  Mục tiêu chiến lược\n", h1),
        (
            "Tăng mức độ nhận diện và xây dựng sự tin tưởng dài hạn thông qua những trải nghiệm thương hiệu có chiều sâu.\n\n",
            body,
        ),
        ("03  Đối tượng mục tiêu\n", h1),
        (
            "Đội ngũ sáng tạo, chuyên gia văn phòng và cá nhân đòi hỏi sự chỉn chu, tốc độ và tính bảo mật cao.\n\n",
            body,
        ),
        ("04  Kế hoạch triển khai\n", h1),
        (
            "• Q1: Chuẩn hóa bộ nhận diện và công cụ soạn thảo tiếng Việt Telex mượt mà.\n• Q2: Mở rộng tính năng xuất bản Word (.doc/.docx) và mẫu tài liệu chuyên nghiệp.\n• Q3-Q4: Tối ưu hiệu năng, bảo mật dữ liệu và sao lưu thông minh.\n\n",
            bullet,
        ),
        ("05  Đo lường hiệu quả\n", h1),
    ];

    let mut content = String::new();
    let mut spans = Vec::new();
    for (text, style) in sections {
        let start = content.len();
        content.push_str(text);
        let end = content.len();
        spans.push(StyleSpan { start, end, style });
    }
    let tail_start = content.len();
    content.push_str("Đánh giá định kỳ thông qua sự hài lòng người dùng và năng suất thực tế mỗi ngày.\n\n──────────────────────────────────────────────────────────────────────────\nSTUDIO NORTH — BRAND PLAN                                                         01");
    spans.push(StyleSpan {
        start: tail_start,
        end: content.len(),
        style: body,
    });

    let para_count = content.split(['\n', PAGE_BREAK_CHAR]).count().max(1);
    let mut ed = crate::editor::EditorState::new_empty();
    ed.content = content;
    ed.spans = spans;
    ed.para_alignments = vec![TextAlignment::Left; para_count];
    ed.is_dirty = false;
    ed.mark_content_changed();
    ed
}

pub fn build_report_sample() -> crate::editor::EditorState {
    use crate::document::{StyleSpan, TextStyle};

    let title = TextStyle {
        bold: true,
        font_size: 26.0,
        color: [35, 43, 60],
        ..Default::default()
    };
    let h1 = TextStyle {
        bold: true,
        font_size: 18.0,
        color: [64, 89, 220],
        ..Default::default()
    };
    let body = TextStyle {
        font_size: 13.0,
        color: [35, 43, 60],
        ..Default::default()
    };

    let text = "BÁO CÁO TIẾN ĐỘ THÁNG 11\n\n1. Tổng quan tình hình thực hiện\nTrong tháng qua, toàn bộ các mục tiêu cốt lõi đã hoàn thành đúng hạn.\n\n2. Các kết quả nổi bật\n• Tối ưu hóa hiệu năng ứng dụng soạn thảo đạt 60 FPS.\n• Hỗ trợ gõ tiếng Việt Telex chuẩn xác, không kẹt phím.\n• Tự động sao lưu an toàn khi có sự cố.\n\n3. Kế hoạch tháng tiếp theo\nTiếp tục hoàn thiện các mẫu văn bản văn phòng và mở rộng khả năng in ấn.";
    let mut ed = crate::editor::EditorState::new_empty();
    ed.content = text.to_string();
    ed.spans = vec![
        StyleSpan {
            start: 0,
            end: 24,
            style: title,
        },
        StyleSpan {
            start: 26,
            end: 58,
            style: h1,
        },
        StyleSpan {
            start: 59,
            end: 129,
            style: body,
        },
        StyleSpan {
            start: 131,
            end: 154,
            style: h1,
        },
        StyleSpan {
            start: 155,
            end: 304,
            style: body,
        },
        StyleSpan {
            start: 306,
            end: 334,
            style: h1,
        },
        StyleSpan {
            start: 335,
            end: ed.content.len(),
            style: body,
        },
    ];
    let para_count = ed.content.split(['\n', PAGE_BREAK_CHAR]).count().max(1);
    ed.para_alignments = vec![TextAlignment::Left; para_count];
    ed.is_dirty = false;
    ed.mark_content_changed();
    ed
}

pub fn build_proposal_sample() -> crate::editor::EditorState {
    use crate::document::{StyleSpan, TextStyle};

    let title = TextStyle {
        bold: true,
        font_size: 26.0,
        color: [62, 140, 119],
        ..Default::default()
    };
    let h1 = TextStyle {
        bold: true,
        font_size: 18.0,
        color: [35, 43, 60],
        ..Default::default()
    };
    let body = TextStyle {
        font_size: 13.0,
        color: [35, 43, 60],
        ..Default::default()
    };

    let text = "ĐỀ XUẤT DỰ ÁN MỚI\n\n1. Mục tiêu dự án\nChuyển đổi các ý tưởng sáng tạo thành giải pháp công nghệ phục vụ người dùng Việt Nam.\n\n2. Phạm vi triển khai\n• Giai đoạn Nghiên cứu & Định hình sản phẩm.\n• Giai đoạn Phát triển MVP và thử nghiệm người dùng.\n• Giai đoạn Tối ưu hóa và mở rộng quy mô.\n\n3. Dự kiến ngân sách & nhân sự\nPhân bổ hợp lý nguồn lực để đảm bảo tiến độ và chất lượng cao nhất.";
    let mut ed = crate::editor::EditorState::new_empty();
    ed.content = text.to_string();
    ed.spans = vec![
        StyleSpan {
            start: 0,
            end: 17,
            style: title,
        },
        StyleSpan {
            start: 19,
            end: 36,
            style: h1,
        },
        StyleSpan {
            start: 37,
            end: 125,
            style: body,
        },
        StyleSpan {
            start: 127,
            end: 148,
            style: h1,
        },
        StyleSpan {
            start: 149,
            end: 298,
            style: body,
        },
        StyleSpan {
            start: 300,
            end: 332,
            style: h1,
        },
        StyleSpan {
            start: 333,
            end: ed.content.len(),
            style: body,
        },
    ];
    let para_count = ed.content.split(['\n', PAGE_BREAK_CHAR]).count().max(1);
    ed.para_alignments = vec![TextAlignment::Left; para_count];
    ed.is_dirty = false;
    ed.mark_content_changed();
    ed
}

pub fn render_startup_modal(ctx: &egui::Context, app: &mut MyWordApp, theme: &ThemePalette) {
    if !app.show_startup_modal {
        return;
    }

    let mut action_open = false;
    let mut action_new = false;
    let mut action_open_selected = false;
    let mut action_report = false;
    let mut action_proposal = false;
    let mut action_recover = false;
    let mut action_close = false;

    // Lớp phủ mờ nền phía sau modal (#17213B với opacity ~38% theo Figma)
    let screen = ctx.screen_rect();
    egui::Area::new(egui::Id::new("startup_backdrop"))
        .fixed_pos(screen.min)
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(screen.size(), egui::Sense::click());
            ui.painter().rect_filled(
                rect,
                Rounding::ZERO,
                Color32::from_rgba_unmultiplied(23, 33, 59, 100),
            );
        });

    let modal_w = (screen.width() - 48.0).clamp(740.0, 1120.0);
    let modal_h = (screen.height() - 48.0).clamp(520.0, 752.0);

    egui::Window::new("startup_welcome_modal")
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Foreground)
        .fixed_size(Vec2::new(modal_w, modal_h))
        .frame(
            egui::Frame::none()
                .fill(theme.surface_bg)
                .rounding(Rounding::same(16.0))
                .stroke(Stroke::new(1.0_f32, theme.border_color))
                .shadow(egui::epaint::Shadow {
                    offset: Vec2::new(0.0, 20.0),
                    blur: 70.0,
                    spread: 0.0,
                    color: Color32::from_rgba_unmultiplied(20, 31, 61, 56),
                }),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Cột điều hướng trái (#F7F8FC, rộng 232px theo Figma)
                let left_w = 232.0;
                egui::Frame::none()
                    .fill(if theme.is_dark {
                        theme.sidebar_bg
                    } else {
                        Color32::from_rgb(247, 248, 252)
                    })
                    .rounding(Rounding {
                        nw: 16.0,
                        sw: 16.0,
                        ne: 0.0,
                        se: 0.0,
                    })
                    .inner_margin(egui::Margin::symmetric(20.0, 24.0))
                    .show(ui, |ui| {
                        ui.set_width(left_w - 40.0);
                        ui.set_height(modal_h - 48.0);
                        ui.vertical(|ui| {
                            // Logo và tên ứng dụng
                            ui.horizontal(|ui| {
                                draw_app_logo(ui);
                                ui.add_space(8.0);
                                ui.label(
                                    egui::RichText::new("my word")
                                        .size(19.0)
                                        .strong()
                                        .color(theme.text_main),
                                );
                            });
                            ui.add_space(26.0);
                            ui.label(
                                egui::RichText::new("KHÔNG GIAN CỦA BẠN")
                                    .size(9.5)
                                    .strong()
                                    .color(theme.text_muted),
                            );
                            ui.add_space(10.0);

                            // Các mục không gian
                            for (tab, icon, label, badge) in [
                                (StartupNavTab::Recent, IconKind::Clock, "Gần đây", Some("4")),
                                (StartupNavTab::Pinned, IconKind::Pin, "Đã ghim", None),
                                (
                                    StartupNavTab::Local,
                                    IconKind::FolderOpen,
                                    "Máy tính này",
                                    None,
                                ),
                                (StartupNavTab::Templates, IconKind::Grid, "Kho mẫu", None),
                            ] {
                                let selected = app.startup_nav_tab == tab;
                                let bg = if selected {
                                    if theme.is_dark {
                                        theme.accent_tint
                                    } else {
                                        Color32::from_rgb(233, 237, 255)
                                    }
                                } else {
                                    Color32::TRANSPARENT
                                };
                                let fg = if selected {
                                    theme.primary_accent
                                } else {
                                    theme.text_main
                                };
                                let (row_r, resp) = ui.allocate_exact_size(
                                    Vec2::new(left_w - 40.0, 36.0),
                                    egui::Sense::click(),
                                );
                                ui.painter().rect_filled(row_r, Rounding::same(7.0), bg);
                                let icon_r = Rect::from_min_size(
                                    Pos2::new(row_r.min.x + 12.0, row_r.center().y - 8.0),
                                    Vec2::splat(16.0),
                                );
                                crate::icons::draw_icon(ui.painter(), icon_r, icon, fg);
                                ui.painter().text(
                                    Pos2::new(row_r.min.x + 38.0, row_r.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    label,
                                    FontId::proportional(12.5),
                                    fg,
                                );
                                if let Some(badge_text) = badge {
                                    ui.painter().text(
                                        Pos2::new(row_r.max.x - 14.0, row_r.center().y),
                                        egui::Align2::RIGHT_CENTER,
                                        badge_text,
                                        FontId::proportional(11.5),
                                        fg,
                                    );
                                }
                                if resp.clicked() {
                                    if tab == StartupNavTab::Local {
                                        action_open = true;
                                    } else {
                                        app.startup_nav_tab = tab;
                                    }
                                }
                                ui.add_space(4.0);
                            }

                            // Footer bên trái
                            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                                ui.label(
                                    egui::RichText::new("My Word 1.0")
                                        .size(10.0)
                                        .color(theme.text_muted),
                                );
                                ui.add_space(6.0);
                                let (set_r, set_resp) = ui.allocate_exact_size(
                                    Vec2::new(left_w - 40.0, 32.0),
                                    egui::Sense::click(),
                                );
                                if set_resp.hovered() {
                                    ui.painter().rect_filled(
                                        set_r,
                                        Rounding::same(6.0),
                                        theme.input_bg,
                                    );
                                }
                                let set_icon_r = Rect::from_min_size(
                                    Pos2::new(set_r.min.x + 10.0, set_r.center().y - 8.0),
                                    Vec2::splat(16.0),
                                );
                                crate::icons::draw_icon(
                                    ui.painter(),
                                    set_icon_r,
                                    IconKind::Settings,
                                    theme.text_muted,
                                );
                                ui.painter().text(
                                    Pos2::new(set_r.min.x + 36.0, set_r.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    "Cài đặt ứng dụng",
                                    FontId::proportional(11.5),
                                    theme.text_muted,
                                );
                                ui.add_space(8.0);
                                ui.separator();
                            });
                        });
                    });

                // Nội dung bên phải (File selection content)
                egui::Frame::none()
                    .fill(theme.surface_bg)
                    .rounding(Rounding {
                        nw: 0.0,
                        sw: 0.0,
                        ne: 16.0,
                        se: 16.0,
                    })
                    .inner_margin(egui::Margin::symmetric(28.0, 24.0))
                    .show(ui, |ui| {
                        let right_w = modal_w - left_w - 56.0;
                        ui.set_width(right_w);
                        ui.set_height(modal_h - 48.0);

                        // Tiêu đề chào mừng & Nút đóng
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new("Chào mừng đến với My Word")
                                        .size(23.0)
                                        .strong()
                                        .color(theme.text_main),
                                );
                                ui.add_space(3.0);
                                ui.label(
                                    egui::RichText::new(
                                        "Tiếp tục ý tưởng của bạn hoặc bắt đầu một trang viết mới.",
                                    )
                                    .size(12.5)
                                    .color(theme.text_muted),
                                );
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                                if icon_button(
                                    ui,
                                    IconKind::Close,
                                    "Đóng (Esc)",
                                    false,
                                    theme.is_dark,
                                )
                                .clicked()
                                {
                                    action_close = true;
                                }
                            });
                        });

                        ui.add_space(16.0);

                        // Header Bắt đầu mới + Xem tất cả mẫu
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Bắt đầu mới")
                                    .size(13.5)
                                    .strong()
                                    .color(theme.text_main),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let view_all = egui::Button::new(
                                        egui::RichText::new("Xem tất cả mẫu  ›")
                                            .size(11.5)
                                            .color(theme.primary_accent),
                                    )
                                    .fill(Color32::TRANSPARENT);
                                    if ui.add(view_all).clicked() {
                                        app.startup_nav_tab = StartupNavTab::Templates;
                                    }
                                },
                            );
                        });
                        ui.add_space(8.0);

                        // 3 thẻ Bắt đầu mới theo đúng thiết kế Figma
                        let card_gap = 12.0;
                        let card_w = (right_w - card_gap * 2.0) / 3.0;
                        let card_h = 106.0;
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = card_gap;

                            // Thẻ 1: Tài liệu trống (Highlight xanh)
                            let (c1, r1) = ui.allocate_exact_size(
                                Vec2::new(card_w, card_h),
                                egui::Sense::click(),
                            );
                            ui.painter().rect(
                                c1,
                                Rounding::same(9.0),
                                theme.accent_tint,
                                Stroke::new(1.0_f32, theme.accent_border),
                            );
                            let badge = Rect::from_min_size(
                                Pos2::new(c1.min.x + 14.0, c1.min.y + 14.0),
                                Vec2::new(36.0, 42.0),
                            );
                            ui.painter().rect(
                                badge,
                                Rounding::same(5.0),
                                theme.surface_bg,
                                Stroke::new(1.0_f32, theme.accent_border),
                            );
                            crate::icons::draw_icon(
                                ui.painter(),
                                badge.shrink(10.0),
                                IconKind::Plus,
                                theme.primary_accent,
                            );
                            ui.painter().text(
                                Pos2::new(c1.min.x + 60.0, c1.min.y + 16.0),
                                egui::Align2::LEFT_TOP,
                                "Tài liệu trống",
                                FontId::proportional(13.0),
                                theme.text_main,
                            );
                            ui.painter().text(
                                Pos2::new(c1.min.x + 60.0, c1.min.y + 36.0),
                                egui::Align2::LEFT_TOP,
                                "Viết theo cách của bạn",
                                FontId::proportional(10.0),
                                theme.text_muted,
                            );
                            ui.painter().text(
                                Pos2::new(c1.min.x + 14.0, c1.max.y - 12.0),
                                egui::Align2::LEFT_BOTTOM,
                                "Ctrl + N",
                                FontId::proportional(10.0),
                                theme.primary_accent,
                            );
                            if r1.clicked() {
                                action_new = true;
                            }

                            // Thẻ 2: Báo cáo
                            let (c2, r2) = ui.allocate_exact_size(
                                Vec2::new(card_w, card_h),
                                egui::Sense::click(),
                            );
                            let border2 = if r2.hovered() {
                                theme.primary_accent
                            } else {
                                theme.border_color
                            };
                            ui.painter().rect(
                                c2,
                                Rounding::same(9.0),
                                theme.surface_bg,
                                Stroke::new(1.0_f32, border2),
                            );
                            let thumb2 = Rect::from_min_size(
                                Pos2::new(c2.min.x + 14.0, c2.min.y + 14.0),
                                Vec2::new(36.0, 42.0),
                            );
                            ui.painter().rect(
                                thumb2,
                                Rounding::same(4.0),
                                theme.surface_bg,
                                Stroke::new(1.0_f32, theme.border_color),
                            );
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    Pos2::new(thumb2.min.x + 5.0, thumb2.min.y + 6.0),
                                    Vec2::new(26.0, 4.0),
                                ),
                                Rounding::same(1.0),
                                theme.primary_accent,
                            );
                            for dy in [16.0, 22.0, 28.0] {
                                ui.painter().rect_filled(
                                    Rect::from_min_size(
                                        Pos2::new(thumb2.min.x + 5.0, thumb2.min.y + dy),
                                        Vec2::new(if dy > 25.0 { 16.0 } else { 24.0 }, 2.0),
                                    ),
                                    Rounding::ZERO,
                                    theme.border_color,
                                );
                            }
                            ui.painter().text(
                                Pos2::new(c2.min.x + 60.0, c2.min.y + 16.0),
                                egui::Align2::LEFT_TOP,
                                "Báo cáo",
                                FontId::proportional(13.0),
                                theme.text_main,
                            );
                            ui.painter().text(
                                Pos2::new(c2.min.x + 60.0, c2.min.y + 36.0),
                                egui::Align2::LEFT_TOP,
                                "Rõ ràng & chuyên nghiệp",
                                FontId::proportional(10.0),
                                theme.text_muted,
                            );
                            ui.painter().text(
                                Pos2::new(c2.min.x + 14.0, c2.max.y - 12.0),
                                egui::Align2::LEFT_BOTTOM,
                                "Dùng mẫu này",
                                FontId::proportional(10.0),
                                theme.text_muted,
                            );
                            if r2.clicked() {
                                action_report = true;
                            }

                            // Thẻ 3: Đề xuất dự án
                            let (c3, r3) = ui.allocate_exact_size(
                                Vec2::new(card_w, card_h),
                                egui::Sense::click(),
                            );
                            let border3 = if r3.hovered() {
                                theme.primary_accent
                            } else {
                                theme.border_color
                            };
                            ui.painter().rect(
                                c3,
                                Rounding::same(9.0),
                                theme.surface_bg,
                                Stroke::new(1.0_f32, border3),
                            );
                            let thumb3 = Rect::from_min_size(
                                Pos2::new(c3.min.x + 14.0, c3.min.y + 14.0),
                                Vec2::new(36.0, 42.0),
                            );
                            ui.painter().rect(
                                thumb3,
                                Rounding::same(4.0),
                                theme.surface_bg,
                                Stroke::new(1.0_f32, theme.border_color),
                            );
                            ui.painter().rect_filled(
                                Rect::from_min_size(
                                    Pos2::new(thumb3.min.x + 5.0, thumb3.min.y + 6.0),
                                    Vec2::new(26.0, 4.0),
                                ),
                                Rounding::same(1.0),
                                Color32::from_rgb(62, 140, 119),
                            );
                            for dy in [16.0, 22.0, 28.0] {
                                ui.painter().rect_filled(
                                    Rect::from_min_size(
                                        Pos2::new(thumb3.min.x + 5.0, thumb3.min.y + dy),
                                        Vec2::new(if dy > 25.0 { 16.0 } else { 24.0 }, 2.0),
                                    ),
                                    Rounding::ZERO,
                                    theme.border_color,
                                );
                            }
                            ui.painter().text(
                                Pos2::new(c3.min.x + 60.0, c3.min.y + 16.0),
                                egui::Align2::LEFT_TOP,
                                "Đề xuất dự án",
                                FontId::proportional(13.0),
                                theme.text_main,
                            );
                            ui.painter().text(
                                Pos2::new(c3.min.x + 60.0, c3.min.y + 36.0),
                                egui::Align2::LEFT_TOP,
                                "Từ ý tưởng đến hành động",
                                FontId::proportional(10.0),
                                theme.text_muted,
                            );
                            ui.painter().text(
                                Pos2::new(c3.min.x + 14.0, c3.max.y - 12.0),
                                egui::Align2::LEFT_BOTTOM,
                                "Dùng mẫu này",
                                FontId::proportional(10.0),
                                theme.text_muted,
                            );
                            if r3.clicked() {
                                action_proposal = true;
                            }
                        });

                        ui.add_space(16.0);

                        // Header Tài liệu gần đây + Ô tìm kiếm
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Tài liệu gần đây")
                                    .size(13.5)
                                    .strong()
                                    .color(theme.text_main),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut app.startup_search_query)
                                            .hint_text("Tìm tên tài liệu...")
                                            .desired_width(200.0),
                                    );
                                },
                            );
                        });
                        ui.add_space(6.0);

                        // Tiêu đề cột bảng tài liệu gần đây
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.label(
                                egui::RichText::new("TÊN TÀI LIỆU")
                                    .size(9.5)
                                    .strong()
                                    .color(theme.text_muted),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.set_width(120.0);
                                    ui.label(
                                        egui::RichText::new("KÍCH THƯỚC")
                                            .size(9.5)
                                            .strong()
                                            .color(theme.text_muted),
                                    );
                                },
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.set_width(140.0);
                                    ui.label(
                                        egui::RichText::new("ĐÃ SỬA ĐỔI")
                                            .size(9.5)
                                            .strong()
                                            .color(theme.text_muted),
                                    );
                                },
                            );
                        });
                        ui.separator();
                        ui.add_space(4.0);

                        // Danh sách 4 tài liệu gần đây từ Figma
                        let recent_items = [
                            (
                                "Kế hoạch thương hiệu 2026.docx",
                                "Tài liệu / Studio North",
                                "Vừa xong",
                                "248 KB",
                            ),
                            (
                                "Báo cáo tiến độ tháng 11.docx",
                                "Tài liệu / Công việc",
                                "Hôm qua, 16:30",
                                "186 KB",
                            ),
                            (
                                "Biên bản họp nhóm.docx",
                                "Tài liệu / Cuộc họp",
                                "16 thg 11, 2026",
                                "72 KB",
                            ),
                            (
                                "Ý tưởng cho dự án mới.docx",
                                "Tài liệu / Cá nhân",
                                "14 thg 11, 2026",
                                "54 KB",
                            ),
                        ];

                        let q = app.startup_search_query.to_lowercase();
                        for (idx, (doc_title, doc_sub, modified, size)) in
                            recent_items.iter().enumerate()
                        {
                            if !q.is_empty() && !doc_title.to_lowercase().contains(&q) {
                                continue;
                            }
                            let is_selected = app.selected_recent_idx == idx;
                            let (row_r, resp) = ui.allocate_exact_size(
                                Vec2::new(right_w, 48.0),
                                egui::Sense::click(),
                            );
                            let bg_color = if is_selected {
                                if theme.is_dark {
                                    theme.accent_tint
                                } else {
                                    Color32::from_rgb(238, 241, 255)
                                }
                            } else if resp.hovered() {
                                theme.card_bg
                            } else {
                                theme.surface_bg
                            };
                            let border_color = if is_selected {
                                if theme.is_dark {
                                    theme.primary_accent
                                } else {
                                    Color32::from_rgb(204, 213, 255)
                                }
                            } else {
                                theme.border_color
                            };
                            ui.painter().rect(
                                row_r,
                                Rounding::same(6.0),
                                bg_color,
                                Stroke::new(1.0_f32, border_color),
                            );

                            let doc_icon_r = Rect::from_min_size(
                                Pos2::new(row_r.min.x + 12.0, row_r.center().y - 10.0),
                                Vec2::splat(20.0),
                            );
                            crate::icons::draw_icon(
                                ui.painter(),
                                doc_icon_r,
                                IconKind::PageDoc,
                                if is_selected {
                                    theme.primary_accent
                                } else {
                                    theme.text_muted
                                },
                            );

                            ui.painter().text(
                                Pos2::new(row_r.min.x + 42.0, row_r.min.y + 10.0),
                                egui::Align2::LEFT_TOP,
                                *doc_title,
                                FontId::proportional(12.5),
                                theme.text_main,
                            );
                            ui.painter().text(
                                Pos2::new(row_r.min.x + 42.0, row_r.min.y + 28.0),
                                egui::Align2::LEFT_TOP,
                                *doc_sub,
                                FontId::proportional(10.0),
                                theme.text_muted,
                            );

                            ui.painter().text(
                                Pos2::new(row_r.max.x - 220.0, row_r.center().y),
                                egui::Align2::LEFT_CENTER,
                                *modified,
                                FontId::proportional(11.0),
                                theme.text_muted,
                            );
                            ui.painter().text(
                                Pos2::new(row_r.max.x - 60.0, row_r.center().y),
                                egui::Align2::LEFT_CENTER,
                                *size,
                                FontId::proportional(11.0),
                                theme.text_muted,
                            );

                            if is_selected {
                                let check_r = Rect::from_min_size(
                                    Pos2::new(row_r.max.x - 28.0, row_r.center().y - 7.0),
                                    Vec2::splat(14.0),
                                );
                                crate::icons::draw_icon(
                                    ui.painter(),
                                    check_r,
                                    IconKind::Check,
                                    theme.primary_accent,
                                );
                            } else if resp.hovered() {
                                let arr_r = Rect::from_min_size(
                                    Pos2::new(row_r.max.x - 26.0, row_r.center().y - 6.0),
                                    Vec2::splat(12.0),
                                );
                                crate::icons::draw_icon(
                                    ui.painter(),
                                    arr_r,
                                    IconKind::ChevronRight,
                                    theme.text_muted,
                                );
                            }

                            if resp.clicked() {
                                app.selected_recent_idx = idx;
                            }
                            if resp.double_clicked() {
                                app.selected_recent_idx = idx;
                                action_open_selected = true;
                            }
                            ui.add_space(4.0);
                        }

                        // Banner khôi phục bản sao lưu (nếu có)
                        if app.show_recovery_banner {
                            let (rec_r, rec_resp) = ui.allocate_exact_size(
                                Vec2::new(right_w, 38.0),
                                egui::Sense::click(),
                            );
                            ui.painter().rect(
                                rec_r,
                                Rounding::same(6.0),
                                theme.accent_tint,
                                Stroke::new(1.0_f32, theme.accent_border),
                            );
                            ui.painter().text(
                                Pos2::new(rec_r.min.x + 12.0, rec_r.center().y),
                                egui::Align2::LEFT_CENTER,
                                "Khôi phục tài liệu chưa lưu từ phiên làm việc trước",
                                FontId::proportional(11.5),
                                theme.text_main,
                            );
                            ui.painter().text(
                                Pos2::new(rec_r.max.x - 14.0, rec_r.center().y),
                                egui::Align2::RIGHT_CENTER,
                                "Khôi phục",
                                FontId::proportional(11.5),
                                theme.primary_accent,
                            );
                            if rec_resp.clicked() {
                                action_recover = true;
                            }
                            ui.add_space(4.0);
                        }

                        // Footer Modal chuẩn Figma
                        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    let browse_btn = egui::Button::new(
                                        egui::RichText::new("📁  Duyệt máy tính")
                                            .size(12.5)
                                            .color(theme.text_main),
                                    )
                                    .fill(theme.surface_bg)
                                    .stroke(Stroke::new(1.0_f32, theme.border_color))
                                    .rounding(Rounding::same(7.0))
                                    .min_size(Vec2::new(138.0, 36.0));
                                    if ui.add(browse_btn).clicked() {
                                        action_open = true;
                                    }
                                    ui.add_space(2.0);
                                    ui.label(
                                        egui::RichText::new("Hỗ trợ .docx, .doc, .odt, .txt, .mwd")
                                            .size(9.0)
                                            .color(theme.text_muted),
                                    );
                                });

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let open_cta = egui::Button::new(
                                            egui::RichText::new("📁  Mở tài liệu")
                                                .size(12.5)
                                                .color(Color32::WHITE)
                                                .strong(),
                                        )
                                        .fill(theme.primary_accent)
                                        .rounding(Rounding::same(7.0))
                                        .min_size(Vec2::new(140.0, 36.0));
                                        if ui.add(open_cta).clicked() {
                                            action_open_selected = true;
                                        }

                                        let cancel_btn = egui::Button::new(
                                            egui::RichText::new("Hủy")
                                                .size(12.5)
                                                .color(theme.text_main),
                                        )
                                        .fill(theme.surface_bg)
                                        .stroke(Stroke::new(1.0_f32, theme.border_color))
                                        .rounding(Rounding::same(7.0))
                                        .min_size(Vec2::new(72.0, 36.0));
                                        if ui.add(cancel_btn).clicked() {
                                            action_close = true;
                                        }
                                    },
                                );
                            });
                            ui.add_space(8.0);
                            ui.separator();
                        });
                    });
            });
        });

    if action_open {
        app.show_startup_modal = false;
        app.trigger_open_dialog(ctx.clone());
    } else if action_new {
        app.show_startup_modal = false;
        app.new_document();
    } else if action_open_selected {
        app.show_startup_modal = false;
        match app.selected_recent_idx {
            0 => {
                app.editor = build_brand_plan_sample();
                app.current_file_path = Some(PathBuf::from("Kế hoạch thương hiệu 2026.docx"));
                app.set_status("Đã mở: Kế hoạch thương hiệu 2026.docx");
            }
            1 => {
                app.editor = build_report_sample();
                app.current_file_path = Some(PathBuf::from("Báo cáo tiến độ tháng 11.docx"));
                app.set_status("Đã mở: Báo cáo tiến độ tháng 11.docx");
            }
            2 => {
                app.editor = build_brand_plan_sample();
                app.current_file_path = Some(PathBuf::from("Biên bản họp nhóm.docx"));
                app.set_status("Đã mở: Biên bản họp nhóm.docx");
            }
            _ => {
                app.editor = build_proposal_sample();
                app.current_file_path = Some(PathBuf::from("Ý tưởng cho dự án mới.docx"));
                app.set_status("Đã mở: Ý tưởng cho dự án mới.docx");
            }
        }
        app.reset_document_view_state();
    } else if action_report {
        app.show_startup_modal = false;
        app.editor = build_report_sample();
        app.current_file_path = Some(PathBuf::from("Báo cáo mới.docx"));
        app.reset_document_view_state();
        app.set_status("Đã nạp mẫu Báo cáo");
    } else if action_proposal {
        app.show_startup_modal = false;
        app.editor = build_proposal_sample();
        app.current_file_path = Some(PathBuf::from("Đề xuất dự án mới.docx"));
        app.reset_document_view_state();
        app.set_status("Đã nạp mẫu Đề xuất dự án");
    } else if action_recover {
        app.show_startup_modal = false;
        app.recover_autosave();
    } else if action_close {
        app.show_startup_modal = false;
    }
}

pub fn render_navigation_sidebar(
    ctx: &egui::Context,
    app: &mut MyWordApp,
    theme: &ThemePalette,
    total_pages: usize,
    caret_page_idx: usize,
) {
    egui::SidePanel::left("nav_sidebar")
        .resizable(false)
        .exact_width(228.0)
        .frame(
            egui::Frame::none()
                .fill(theme.sidebar_bg)
                .inner_margin(egui::Margin::same(14.0))
                .stroke(Stroke::new(1.0_f32, theme.border_color)),
        )
        .show(ctx, |ui| {
            // Header panel
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Điều hướng")
                        .size(13.5)
                        .strong()
                        .color(theme.text_main),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(
                        ui,
                        IconKind::ChevronLeft,
                        "Thu gọn bảng điều hướng",
                        false,
                        theme.is_dark,
                    )
                    .clicked()
                    {
                        app.show_nav_panel = false;
                    }
                });
            });
            ui.add_space(8.0);

            // Ô tìm nhanh trong tài liệu
            let find_resp = ui.add(
                egui::TextEdit::singleline(&mut app.find_query)
                    .desired_width(f32::INFINITY)
                    .hint_text("Tìm trong tài liệu..."),
            );
            if find_resp.changed() {
                app.editor.find_dirty = true;
                if !app.find_query.is_empty() {
                    app.find_bar_open = true;
                }
            }
            ui.add_space(10.0);

            // Tabs: Mục lục / Trang
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                for (tab, label) in [(NavTab::Outline, "Mục lục"), (NavTab::Pages, "Trang")] {
                    let selected = app.nav_tab == tab;
                    let col = if selected {
                        theme.primary_accent
                    } else {
                        theme.text_muted
                    };
                    let resp = ui.add(
                        egui::Button::new(if selected {
                            egui::RichText::new(label).size(12.0).strong().color(col)
                        } else {
                            egui::RichText::new(label).size(12.0).color(col)
                        })
                        .fill(Color32::TRANSPARENT)
                        .stroke(Stroke::NONE)
                        .min_size(Vec2::new(44.0, 24.0)),
                    );
                    if selected {
                        let r = resp.rect;
                        ui.painter().rect_filled(
                            Rect::from_min_max(
                                Pos2::new(r.min.x, r.max.y - 2.0),
                                Pos2::new(r.max.x, r.max.y),
                            ),
                            Rounding::same(1.0),
                            theme.primary_accent,
                        );
                    }
                    if resp.clicked() {
                        app.nav_tab = tab;
                    }
                }
            });
            ui.separator();
            ui.add_space(6.0);

            // Nội dung danh sách
            let avail_h = (ui.available_height() - 96.0).max(120.0);
            egui::ScrollArea::vertical()
                .max_height(avail_h)
                .show(ui, |ui| match app.nav_tab {
                    NavTab::Outline => {
                        let headings = app.extract_outline_headings();
                        if headings.is_empty() {
                            ui.add_space(8.0);
                            ui.label(
                                egui::RichText::new(
                                    "Chưa có tiêu đề.\nHãy dùng Tiêu đề 1 hoặc Tiêu đề 2 trên thanh công cụ để tạo mục lục tự động.",
                                )
                                .size(11.0)
                                .color(theme.text_muted),
                            );
                        } else {
                            let active_idx = headings
                                .iter()
                                .rposition(|h| h.byte_offset <= app.editor.caret_byte_offset);
                            let mut h1_count = 0usize;
                            let mut jump_target: Option<usize> = None;

                            for (idx, h) in headings.iter().enumerate() {
                                let is_active = active_idx == Some(idx);
                                if h.level == 1 {
                                    h1_count += 1;
                                }
                                let indent = match h.level {
                                    1 => 0.0,
                                    2 => 10.0,
                                    _ => 20.0,
                                };
                                let label = if h.level == 1 {
                                    format!("{:02}  {}", h1_count, h.title)
                                } else {
                                    h.title.clone()
                                };

                                let (row_r, resp) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), 30.0),
                                    egui::Sense::click(),
                                );
                                if is_active {
                                    ui.painter().rect_filled(
                                        row_r,
                                        Rounding::same(6.0),
                                        theme.accent_tint,
                                    );
                                    ui.painter().rect_filled(
                                        Rect::from_min_size(
                                            Pos2::new(row_r.min.x, row_r.min.y + 6.0),
                                            Vec2::new(3.0, row_r.height() - 12.0),
                                        ),
                                        Rounding::same(1.5),
                                        theme.primary_accent,
                                    );
                                } else if resp.hovered() {
                                    ui.painter().rect_filled(
                                        row_r,
                                        Rounding::same(6.0),
                                        theme.input_bg,
                                    );
                                }
                                let text_col = if is_active {
                                    theme.primary_accent
                                } else if h.level == 1 {
                                    theme.text_main
                                } else {
                                    theme.text_muted
                                };
                                ui.painter().text(
                                    Pos2::new(row_r.min.x + 10.0 + indent, row_r.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    label,
                                    FontId::proportional(if h.level == 1 { 11.5 } else { 11.0 }),
                                    text_col,
                                );
                                if resp.clicked() {
                                    jump_target = Some(h.byte_offset);
                                }
                            }
                            if let Some(off) = jump_target {
                                app.jump_to_byte(off);
                            }
                        }
                    }
                    NavTab::Pages => {
                        let mut jump_target: Option<usize> = None;
                        for page_idx in 0..total_pages {
                            let is_active = page_idx == caret_page_idx;
                            let (row_r, resp) = ui.allocate_exact_size(
                                Vec2::new(ui.available_width(), 32.0),
                                egui::Sense::click(),
                            );
                            if is_active {
                                ui.painter().rect_filled(
                                    row_r,
                                    Rounding::same(6.0),
                                    theme.accent_tint,
                                );
                                ui.painter().rect_filled(
                                    Rect::from_min_size(
                                        Pos2::new(row_r.min.x, row_r.min.y + 6.0),
                                        Vec2::new(3.0, row_r.height() - 12.0),
                                    ),
                                    Rounding::same(1.5),
                                    theme.primary_accent,
                                );
                            } else if resp.hovered() {
                                ui.painter().rect_filled(
                                    row_r,
                                    Rounding::same(6.0),
                                    theme.input_bg,
                                );
                            }
                            ui.painter().text(
                                Pos2::new(row_r.min.x + 12.0, row_r.center().y),
                                egui::Align2::LEFT_CENTER,
                                format!("Trang {}", page_idx + 1),
                                FontId::proportional(12.0),
                                if is_active {
                                    theme.primary_accent
                                } else {
                                    theme.text_main
                                },
                            );
                            if resp.clicked()
                                && let Some(cache) = &app.cached_layout
                                && let Some(placed_list) = cache.pages.get(page_idx)
                                && let Some(first_para) = placed_list.first()
                                && let Some(para_data) = cache.paras.get(first_para.para_data_idx)
                            {
                                let (b, _) = first_para.byte_range(para_data, &app.editor.content);
                                jump_target = Some(b);
                            }
                        }
                        if let Some(off) = jump_target {
                            app.jump_to_byte(off);
                        }
                    }
                });

            // Thẻ mẹo ở đáy sidebar (Mọi ý tưởng, một nơi.)
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                egui::Frame::none()
                    .fill(if theme.is_dark {
                        theme.card_bg
                    } else {
                        Color32::from_rgb(240, 243, 249)
                    })
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        paint_inline_icon(ui, 16.0, IconKind::Book, theme.primary_accent);
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Mọi ý tưởng, một nơi.")
                                .size(11.5)
                                .strong()
                                .color(theme.text_main),
                        );
                        ui.label(
                            egui::RichText::new("Dùng tiêu đề để sắp xếp tài liệu.")
                                .size(10.0)
                                .color(theme.text_muted),
                        );
                    });
            });
        });
}

pub fn render_inspector_sidebar(ctx: &egui::Context, app: &mut MyWordApp, theme: &ThemePalette) {
    egui::SidePanel::right("inspector_sidebar")
        .resizable(false)
        .exact_width(256.0)
        .frame(
            egui::Frame::none()
                .fill(theme.surface_bg)
                .inner_margin(egui::Margin::same(16.0))
                .stroke(Stroke::new(1.0_f32, theme.border_color)),
        )
        .show(ctx, |ui| {
            // Header
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Định dạng")
                        .size(13.5)
                        .strong()
                        .color(theme.text_main),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(
                        ui,
                        IconKind::Close,
                        "Đóng bảng Định dạng",
                        false,
                        theme.is_dark,
                    )
                    .clicked()
                    {
                        app.show_inspector_panel = false;
                    }
                });
            });
            ui.add_space(8.0);

            // Segmented Tabs: Văn bản / Trang
            let seg_w = ui.available_width();
            let (seg_r, _) = ui.allocate_exact_size(Vec2::new(seg_w, 34.0), egui::Sense::hover());
            ui.painter()
                .rect_filled(seg_r, Rounding::same(7.0), theme.input_bg);
            let half_w = (seg_w - 6.0) * 0.5;
            for (idx, (tab, label)) in [
                (InspectorTab::Text, "Văn bản"),
                (InspectorTab::Page, "Trang"),
            ]
            .iter()
            .enumerate()
            {
                let tab_r = Rect::from_min_size(
                    Pos2::new(seg_r.min.x + 3.0 + (idx as f32) * half_w, seg_r.min.y + 3.0),
                    Vec2::new(half_w, 28.0),
                );
                let selected = app.inspector_tab == *tab;
                if selected {
                    ui.painter()
                        .rect_filled(tab_r, Rounding::same(5.0), theme.surface_bg);
                }
                ui.painter().text(
                    tab_r.center(),
                    egui::Align2::CENTER_CENTER,
                    *label,
                    FontId::proportional(11.5),
                    if selected {
                        theme.primary_accent
                    } else {
                        theme.text_muted
                    },
                );
                let resp = ui.interact(tab_r, ui.id().with(idx), egui::Sense::click());
                if resp.clicked() {
                    app.inspector_tab = *tab;
                }
            }
            ui.add_space(14.0);

            match app.inspector_tab {
                InspectorTab::Text => {
                    ui.label(
                        egui::RichText::new("KIỂU VĂN BẢN")
                            .size(10.0)
                            .strong()
                            .color(theme.text_muted),
                    );
                    ui.add_space(6.0);
                    let cur_label = [
                        DocumentPresetStyle::Normal,
                        DocumentPresetStyle::Heading1,
                        DocumentPresetStyle::Heading2,
                        DocumentPresetStyle::Heading3,
                        DocumentPresetStyle::Quote,
                    ]
                    .into_iter()
                    .find(|p| is_preset_active(&app.editor.active_style, *p))
                    .map(|p| p.label())
                    .unwrap_or("Tùy chỉnh");

                    egui::ComboBox::from_id_source("inspector_style_combo")
                        .width(ui.available_width())
                        .selected_text(cur_label)
                        .show_ui(ui, |ui| {
                            for preset in [
                                DocumentPresetStyle::Normal,
                                DocumentPresetStyle::Heading1,
                                DocumentPresetStyle::Heading2,
                                DocumentPresetStyle::Heading3,
                                DocumentPresetStyle::Quote,
                            ] {
                                if ui
                                    .selectable_label(
                                        is_preset_active(&app.editor.active_style, preset),
                                        preset.label(),
                                    )
                                    .clicked()
                                {
                                    app.editor.apply_preset_style(preset);
                                }
                            }
                        });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label(
                        egui::RichText::new("Phông chữ")
                            .size(12.0)
                            .strong()
                            .color(theme.text_main),
                    );
                    ui.add_space(6.0);
                    egui::Frame::none()
                        .fill(theme.card_bg)
                        .stroke(Stroke::new(1.0_f32, theme.border_color))
                        .rounding(Rounding::same(6.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 7.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(
                                egui::RichText::new("Segoe UI (Hệ thống)")
                                    .size(11.5)
                                    .color(theme.text_main),
                            );
                        });
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Cỡ chữ:")
                                .size(11.0)
                                .color(theme.text_muted),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("+").clicked() {
                                let next = (app.editor.active_style.font_size + 1.0).min(48.0);
                                app.editor.apply_format(|s| s.font_size = next);
                            }
                            ui.label(
                                egui::RichText::new(format!(
                                    "{:.0} pt",
                                    app.editor.active_style.font_size
                                ))
                                .size(12.0)
                                .strong()
                                .color(theme.text_main),
                            );
                            if ui.button("−").clicked() {
                                let next = (app.editor.active_style.font_size - 1.0).max(8.0);
                                app.editor.apply_format(|s| s.font_size = next);
                            }
                        });
                    });
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Màu chữ:")
                                .size(11.0)
                                .color(theme.text_muted),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            for rgb in [[22, 163, 74], [220, 38, 38], [64, 89, 220], [35, 43, 60]] {
                                let sel = app.editor.active_style.color == rgb;
                                let (cr, cresp) = ui.allocate_exact_size(
                                    Vec2::new(18.0, 18.0),
                                    egui::Sense::click(),
                                );
                                ui.painter().rect_filled(
                                    cr.shrink(2.0),
                                    Rounding::same(3.0),
                                    Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                                );
                                if sel {
                                    ui.painter().rect_stroke(
                                        cr,
                                        Rounding::same(4.0),
                                        Stroke::new(1.5_f32, theme.primary_accent),
                                    );
                                }
                                if cresp.clicked() {
                                    app.editor.apply_format(|s| s.color = rgb);
                                }
                            }
                        });
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label(
                        egui::RichText::new("Đoạn văn")
                            .size(12.0)
                            .strong()
                            .color(theme.text_main),
                    );
                    ui.add_space(8.0);
                    let cur_align = app.editor.current_alignment();
                    let align_w = ui.available_width();
                    let (al_r, _) =
                        ui.allocate_exact_size(Vec2::new(align_w, 34.0), egui::Sense::hover());
                    ui.painter()
                        .rect_filled(al_r, Rounding::same(6.0), theme.input_bg);
                    let seg_cell_w = (align_w - 6.0) / 4.0;
                    for (idx, (align, icon)) in [
                        (TextAlignment::Left, IconKind::AlignLeft),
                        (TextAlignment::Center, IconKind::AlignCenter),
                        (TextAlignment::Right, IconKind::AlignRight),
                        (TextAlignment::Justify, IconKind::AlignJustify),
                    ]
                    .iter()
                    .enumerate()
                    {
                        let cell_r = Rect::from_min_size(
                            Pos2::new(
                                al_r.min.x + 3.0 + (idx as f32) * seg_cell_w,
                                al_r.min.y + 3.0,
                            ),
                            Vec2::new(seg_cell_w, 28.0),
                        );
                        let active = cur_align == *align;
                        if active {
                            ui.painter()
                                .rect_filled(cell_r, Rounding::same(4.0), theme.surface_bg);
                        }
                        let icon_r = Rect::from_center_size(cell_r.center(), Vec2::splat(16.0));
                        crate::icons::draw_icon(
                            ui.painter(),
                            icon_r,
                            *icon,
                            if active {
                                theme.primary_accent
                            } else {
                                theme.text_muted
                            },
                        );
                        if ui
                            .interact(
                                cell_r,
                                ui.id().with(("insp_align", idx)),
                                egui::Sense::click(),
                            )
                            .clicked()
                        {
                            app.editor.set_alignment(*align);
                        }
                    }

                    // Thẻ tóm tắt khổ giấy A4 ở đáy
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                        egui::Frame::none()
                            .fill(theme.card_bg)
                            .stroke(Stroke::new(1.0_f32, theme.border_color))
                            .rounding(Rounding::same(7.0))
                            .inner_margin(egui::Margin::same(12.0))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    paint_inline_icon(
                                        ui,
                                        20.0,
                                        IconKind::PageDoc,
                                        theme.primary_accent,
                                    );
                                    ui.add_space(6.0);
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new("A4 · Hướng dọc")
                                                .size(12.0)
                                                .strong()
                                                .color(theme.text_main),
                                        );
                                        ui.label(
                                            egui::RichText::new("21 × 29,7 cm")
                                                .size(10.5)
                                                .color(theme.text_muted),
                                        );
                                        if ui
                                            .link(
                                                egui::RichText::new("Thiết lập trang")
                                                    .size(10.5)
                                                    .color(theme.primary_accent),
                                            )
                                            .clicked()
                                        {
                                            app.inspector_tab = InspectorTab::Page;
                                        }
                                    });
                                });
                            });
                    });
                }
                InspectorTab::Page => {
                    ui.label(
                        egui::RichText::new("CÀI ĐẶT TRANG GIẤY")
                            .size(10.0)
                            .strong()
                            .color(theme.text_muted),
                    );
                    ui.add_space(8.0);
                    ui.checkbox(&mut app.show_ruler, "Hiển thị thước kẻ A4");
                    ui.add_space(4.0);
                    ui.checkbox(&mut app.show_margins, "Hiển thị góc căn lề");
                    ui.add_space(4.0);
                    ui.checkbox(&mut app.focus_mode, "Chế độ tập trung");
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label(
                        egui::RichText::new("Thông số chuẩn")
                            .size(12.0)
                            .strong()
                            .color(theme.text_main),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Khổ giấy: A4 (595 × 842 pt)\nLề trang: 72 pt (2,54 cm)",
                        )
                        .size(11.0)
                        .color(theme.text_muted),
                    );
                }
            }
        });
}

pub fn render_status_bar(
    ctx: &egui::Context,
    app: &mut MyWordApp,
    theme: &ThemePalette,
    caret_page_idx: usize,
    total_pages: usize,
    word_count: usize,
    char_count: usize,
) {
    egui::TopBottomPanel::bottom("status_bar")
        .frame(
            egui::Frame::none()
                .fill(theme.surface_bg)
                .inner_margin(egui::Margin::symmetric(16.0, 6.0))
                .stroke(Stroke::new(1.0_f32, theme.border_color)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("Trang {} / {}", caret_page_idx + 1, total_pages))
                        .size(11.5)
                        .color(theme.text_muted),
                );
                vertical_divider(ui, 14.0, theme.border_color);
                ui.label(
                    egui::RichText::new(format!("{} từ  •  {} ký tự", word_count, char_count))
                        .size(11.5)
                        .color(theme.text_muted),
                );
                vertical_divider(ui, 14.0, theme.border_color);
                paint_inline_icon(ui, 14.0, IconKind::Check, Color32::from_rgb(22, 163, 74));
                ui.label(
                    egui::RichText::new("UTF-8 NFC  •  Tiếng Việt")
                        .size(11.5)
                        .color(theme.text_muted),
                );

                if app.editor.unikey_backspace_count > 0 {
                    vertical_divider(ui, 14.0, theme.border_color);
                    ui.label(
                        egui::RichText::new(format!(
                            "Backspace: {}",
                            app.editor.unikey_backspace_count
                        ))
                        .size(11.5)
                        .color(theme.primary_accent),
                    );
                }
                if let Some((msg, created_at)) = &app.status_message
                    && created_at.elapsed().as_secs() < 4
                {
                    vertical_divider(ui, 14.0, theme.border_color);
                    ui.label(
                        egui::RichText::new(msg)
                            .color(theme.primary_accent)
                            .strong()
                            .size(11.5),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let zoom_pct = format!("{:.0}%", app.zoom * 100.0);
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(zoom_pct)
                                    .size(11.5)
                                    .color(theme.text_muted),
                            )
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::NONE),
                        )
                        .on_hover_text("Đặt lại cỡ 100%")
                        .clicked()
                    {
                        app.set_zoom(1.0);
                    }
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("+").size(12.0))
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE)
                                .min_size(Vec2::new(18.0, 18.0)),
                        )
                        .on_hover_text("Phóng to")
                        .clicked()
                    {
                        app.set_zoom(app.zoom + 0.1);
                    }
                    let mut z = app.zoom;
                    if ui
                        .add_sized(
                            [96.0, 16.0],
                            egui::Slider::new(&mut z, 0.5..=2.0)
                                .show_value(false)
                                .trailing_fill(true),
                        )
                        .changed()
                    {
                        app.set_zoom(z);
                    }
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("−").size(12.0))
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::NONE)
                                .min_size(Vec2::new(18.0, 18.0)),
                        )
                        .on_hover_text("Thu nhỏ")
                        .clicked()
                    {
                        app.set_zoom(app.zoom - 0.1);
                    }
                    vertical_divider(ui, 14.0, theme.border_color);
                    if icon_button(
                        ui,
                        IconKind::Ruler,
                        "Bật / Tắt thước kẻ ngang A4",
                        app.show_ruler,
                        theme.is_dark,
                    )
                    .clicked()
                    {
                        app.show_ruler = !app.show_ruler;
                    }
                    if icon_button(
                        ui,
                        IconKind::Margins,
                        "Bật / Tắt góc căn lề A4",
                        app.show_margins,
                        theme.is_dark,
                    )
                    .clicked()
                    {
                        app.show_margins = !app.show_margins;
                    }
                    if icon_button(
                        ui,
                        IconKind::Focus,
                        "Chế độ tập trung",
                        app.focus_mode,
                        theme.is_dark,
                    )
                    .clicked()
                    {
                        app.focus_mode = !app.focus_mode;
                    }
                    ui.label(
                        egui::RichText::new(format!("{:.0} FPS", app.fps))
                            .size(11.0)
                            .color(theme.text_muted),
                    );
                });
            });
        });
}

pub fn render_page_canvas(
    ctx: &egui::Context,
    app: &mut MyWordApp,
    theme: &ThemePalette,
    total_pages: usize,
    show_caret: bool,
) -> Option<(Rect, Rect)> {
    let page_width = A4_WIDTH_PT * app.zoom;
    let page_height = A4_HEIGHT_PT * app.zoom;
    let margin = PAGE_MARGIN_PT * app.zoom;
    let base_line_height = (app.editor.active_style.font_size * 1.6) * app.zoom;
    app.last_line_height = base_line_height;

    let mut ime_output_rect: Option<(Rect, Rect)> = None;
    let selected_range = app.editor.selected_range();
    let pointer_query = app.pending_pointer_query.take();
    let keep_sel_vertical = app.pending_keep_selection;
    app.pending_keep_selection = false;
    let mut ptr_state = PointerResolution {
        next_caret_pos: app.last_caret_pos,
        ..Default::default()
    };

    let doc_text_color = Color32::from_rgb(30, 41, 59);
    let show_find_hl =
        app.find_bar_open && !app.editor.find_dirty && !app.cached_matches.is_empty();
    let zoom = app.zoom;

    if let Some(cache) = app.cached_layout.as_ref() {
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme.workspace_bg))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(24.0);

                        for (page_idx, placed_list) in cache.pages.iter().enumerate() {
                            let (rect, response) = ui.allocate_exact_size(
                                Vec2::new(page_width, page_height),
                                egui::Sense::click_and_drag(),
                            );

                            let (primary_just_pressed, press_origin) = ctx
                                .input(|i| (i.pointer.primary_pressed(), i.pointer.press_origin()));
                            if response.double_clicked() {
                                if let Some(pos) = response.interact_pointer_pos() {
                                    app.pending_pointer_query =
                                        Some(PointerQuery::DoubleClick(pos));
                                    ctx.memory_mut(|m| m.request_focus(response.id));
                                }
                            } else if primary_just_pressed && response.hovered() {
                                if let Some(pos) =
                                    press_origin.or_else(|| response.interact_pointer_pos())
                                {
                                    app.pending_pointer_query = Some(PointerQuery::Click(pos));
                                    ctx.memory_mut(|m| m.request_focus(response.id));
                                }
                            } else if response.clicked() {
                                if let Some(pos) = response.interact_pointer_pos() {
                                    app.pending_pointer_query = Some(PointerQuery::Click(pos));
                                    ctx.memory_mut(|m| m.request_focus(response.id));
                                }
                            } else if response.dragged()
                                && let Some(pos) = response.interact_pointer_pos()
                                && let Some(orig) = press_origin
                                && orig.distance(pos) > 3.0
                            {
                                app.pending_pointer_query = Some(PointerQuery::Drag(pos));
                            }

                            let painter = ui.painter();
                            if page_idx == 0 && app.show_ruler {
                                paint_ruler(
                                    painter,
                                    rect,
                                    cache,
                                    theme,
                                    app.last_caret_pos.map(|p| p.x),
                                );
                            }
                            paint_page_chrome(
                                painter,
                                rect,
                                cache,
                                app.show_margins,
                                page_idx,
                                total_pages,
                            );

                            let content_origin =
                                Pos2::new(rect.min.x + margin, rect.min.y + margin);

                            for placed in placed_list {
                                let p = &cache.paras[placed.para_data_idx];
                                let p_text = &app.editor.content[p.start..p.end];
                                let (placed_b_start, placed_b_end) =
                                    placed.byte_range(p, &app.editor.content);

                                let (row_min_y, row_max_y, chunk_h) = if !p.galley.rows.is_empty()
                                    && placed.row_start < placed.row_end
                                {
                                    let min_y = p.galley.rows[placed.row_start].rect.min.y;
                                    let max_y = p.galley.rows[placed.row_end - 1].rect.max.y;
                                    (min_y, max_y, (max_y - min_y).max(base_line_height))
                                } else {
                                    (0.0, p.height, p.height)
                                };

                                let cursor_y = content_origin.y + placed.y_in_page;
                                let draw_x = match p.alignment {
                                    TextAlignment::Left | TextAlignment::Justify => {
                                        content_origin.x
                                    }
                                    TextAlignment::Center => {
                                        content_origin.x + cache.usable_width * 0.5
                                    }
                                    TextAlignment::Right => content_origin.x + cache.usable_width,
                                };
                                let draw_origin = Pos2::new(draw_x, cursor_y - row_min_y);
                                let clip_rect = Rect::from_min_max(
                                    Pos2::new(rect.min.x, cursor_y - 1.0),
                                    Pos2::new(rect.max.x, cursor_y + chunk_h + 1.0),
                                );
                                let page_painter = painter.with_clip_rect(clip_rect);

                                if let Some(q) = pointer_query {
                                    let target_pos = pointer_pos(q);
                                    if !ptr_state.resolved
                                        && target_pos.y >= cursor_y
                                        && target_pos.y < cursor_y + chunk_h + p.gap
                                    {
                                        let rel = Vec2::new(
                                            target_pos.x - draw_x,
                                            (target_pos.y - cursor_y + row_min_y)
                                                .clamp(row_min_y, row_max_y),
                                        );
                                        let cursor = p.galley.cursor_from_pos(rel);
                                        let target_byte = p.start
                                            + byte_offset_from_char_index(
                                                p_text,
                                                cursor.ccursor.index,
                                            );
                                        ptr_state.apply_hit(
                                            q,
                                            target_byte,
                                            keep_sel_vertical,
                                            app.editor.selection_start,
                                            app.editor.caret_byte_offset,
                                        );
                                        ptr_state.resolved = true;
                                    }
                                }

                                if show_find_hl {
                                    for (m_idx, &(mf, mt)) in app.cached_matches.iter().enumerate()
                                    {
                                        if mf < p.end && mt > p.start {
                                            let lf = mf.saturating_sub(p.start).min(p_text.len());
                                            let lt = (mt - p.start).min(p_text.len());
                                            let fill = if m_idx == app.active_match_idx {
                                                Color32::from_rgba_unmultiplied(251, 146, 60, 180)
                                            } else {
                                                Color32::from_rgba_unmultiplied(253, 224, 71, 150)
                                            };
                                            paint_range_highlight(
                                                &page_painter,
                                                &p.galley,
                                                p_text,
                                                lf,
                                                lt,
                                                draw_origin,
                                                fill,
                                                placed.row_start,
                                                placed.row_end,
                                            );
                                        }
                                    }
                                }

                                if let Some((sf, st)) = selected_range
                                    && sf < p.end
                                    && st > p.start
                                {
                                    let lf = sf.saturating_sub(p.start).min(p_text.len());
                                    let lt = (st - p.start).min(p_text.len());
                                    paint_range_highlight(
                                        &page_painter,
                                        &p.galley,
                                        p_text,
                                        lf,
                                        lt,
                                        draw_origin,
                                        Color32::from_rgba_unmultiplied(191, 219, 254, 185),
                                        placed.row_start,
                                        placed.row_end,
                                    );
                                }

                                page_painter.galley(draw_origin, p.galley.clone(), doc_text_color);

                                if p.followed_by_page_break
                                    && (p.galley.rows.is_empty()
                                        || placed.row_end >= p.galley.rows.len())
                                    && app.show_margins
                                {
                                    let pb_y = cursor_y + chunk_h + 2.0 * zoom;
                                    painter.line_segment(
                                        [
                                            Pos2::new(content_origin.x, pb_y),
                                            Pos2::new(content_origin.x + cache.usable_width, pb_y),
                                        ],
                                        Stroke::new(1.0_f32, Color32::from_rgb(147, 197, 253)),
                                    );
                                    painter.text(
                                        Pos2::new(
                                            content_origin.x + cache.usable_width * 0.5,
                                            pb_y,
                                        ),
                                        egui::Align2::CENTER_CENTER,
                                        " Ngắt trang ",
                                        FontId::proportional(10.0 * zoom),
                                        theme.primary_accent,
                                    );
                                }

                                let is_last_chunk = placed.row_end >= p.galley.rows.len();
                                let in_this_chunk = if is_last_chunk {
                                    app.editor.caret_byte_offset >= placed_b_start
                                        && app.editor.caret_byte_offset <= placed_b_end
                                } else {
                                    app.editor.caret_byte_offset >= placed_b_start
                                        && app.editor.caret_byte_offset < placed_b_end
                                };

                                if in_this_chunk {
                                    let total_chars = p_text.chars().count();
                                    let c_idx = char_index_from_byte_offset(
                                        p_text,
                                        app.editor.caret_byte_offset.saturating_sub(p.start),
                                    );

                                    // Tính tọa độ con trỏ: nếu ở cuối đoạn (bên phải), lấy trực tiếp mép phải glyph cuối
                                    let (rel_x, rel_y, line_h) = if c_idx >= total_chars
                                        && total_chars > 0
                                    {
                                        if let Some(last_row) = p.galley.rows.last() {
                                            let right_x = last_row
                                                .glyphs
                                                .last()
                                                .map(|g| g.pos.x + g.size.x)
                                                .unwrap_or(last_row.rect.max.x);
                                            (right_x, last_row.rect.min.y, last_row.rect.height())
                                        } else {
                                            (0.0, 0.0, base_line_height)
                                        }
                                    } else {
                                        let local_rect = p.galley.pos_from_cursor(
                                            &p.galley.from_ccursor(egui::text::CCursor::new(c_idx)),
                                        );
                                        (local_rect.min.x, local_rect.min.y, local_rect.height())
                                    };

                                    let caret_x = draw_x + rel_x;
                                    let caret_y = cursor_y - row_min_y + rel_y;
                                    let c_h = line_h.max(base_line_height * 0.85);

                                    ptr_state.next_caret_pos = Some(Pos2::new(caret_x, caret_y));

                                    if app.scroll_to_caret {
                                        ui.scroll_to_rect(
                                            Rect::from_min_size(
                                                Pos2::new(caret_x, caret_y),
                                                Vec2::new(2.0, c_h),
                                            ),
                                            None,
                                        );
                                        app.scroll_to_caret = false;
                                    }

                                    if let Some(preedit) = &app.editor.ime_preedit {
                                        let p_font =
                                            font_id_for_style(&app.editor.active_style, zoom);
                                        let preedit_galley = painter.layout_no_wrap(
                                            preedit.clone(),
                                            p_font,
                                            theme.primary_accent,
                                        );
                                        let preedit_w = preedit_galley.size().x;
                                        let after_x = caret_x + preedit_w;

                                        // IME candidate window neo ở sau chuỗi preedit (bên phải con trỏ)
                                        ime_output_rect = Some((
                                            rect,
                                            Rect::from_min_size(
                                                Pos2::new(after_x, caret_y),
                                                Vec2::new(2.0, c_h),
                                            ),
                                        ));

                                        // 1. Chuỗi preedit tạm vẽ bắt đầu từ caret_x
                                        painter.galley(
                                            Pos2::new(caret_x, caret_y),
                                            preedit_galley,
                                            theme.primary_accent,
                                        );

                                        // 2. Gạch chân dưới chuỗi preedit
                                        painter.line_segment(
                                            [
                                                Pos2::new(caret_x, caret_y + c_h),
                                                Pos2::new(after_x, caret_y + c_h),
                                            ],
                                            Stroke::new(1.5_f32, theme.primary_accent),
                                        );

                                        // 3. Con trỏ luôn hiện ở BÊN PHẢI (after_x) khi đang gõ, không nhấp nháy tắt
                                        if selected_range.is_none() {
                                            painter.line_segment(
                                                [
                                                    Pos2::new(after_x, caret_y + 1.0),
                                                    Pos2::new(after_x, caret_y + c_h),
                                                ],
                                                Stroke::new(2.0_f32, theme.primary_accent),
                                            );
                                        }
                                    } else {
                                        ime_output_rect = Some((
                                            rect,
                                            Rect::from_min_size(
                                                Pos2::new(caret_x, caret_y),
                                                Vec2::new(2.0, c_h),
                                            ),
                                        ));

                                        if show_caret && selected_range.is_none() {
                                            // Không có preedit: con trỏ bình thường (nhấp nháy sau 550ms không gõ)
                                            painter.line_segment(
                                                [
                                                    Pos2::new(caret_x, caret_y + 1.0),
                                                    Pos2::new(caret_x, caret_y + c_h),
                                                ],
                                                Stroke::new(2.0_f32, theme.primary_accent),
                                            );
                                        }
                                    }
                                }
                            }

                            if let Some(q) = pointer_query
                                && !ptr_state.resolved
                                && rect.contains(pointer_pos(q))
                                && let (Some(first_placed), Some(last_placed)) =
                                    (placed_list.first(), placed_list.last())
                            {
                                let pos = pointer_pos(q);
                                let target_b = if pos.y < content_origin.y {
                                    cache.paras[first_placed.para_data_idx].start
                                } else {
                                    cache.paras[last_placed.para_data_idx].end
                                };
                                ptr_state.apply_hit(
                                    q,
                                    target_b,
                                    keep_sel_vertical,
                                    app.editor.selection_start,
                                    app.editor.caret_byte_offset,
                                );
                                ptr_state.resolved = true;
                            }

                            ui.add_space(20.0);
                        }

                        ui.add_space(24.0);
                    });
                });
            });
    }

    if let Some(sel) = ptr_state.sel_target {
        app.editor.selection_start = sel;
        if app.editor.apply_format_painter_if_active() {
            app.set_status("Đã áp dụng định dạng sao chép");
        }
    }
    if let Some(c) = ptr_state.caret_target {
        app.editor.caret_byte_offset = c;
        app.last_typing_time = std::time::Instant::now();
        if ptr_state.sel_target.is_some() && app.editor.apply_format_painter_if_active() {
            app.set_status("Đã áp dụng định dạng sao chép");
        }
    }
    if ptr_state.sync_style {
        app.editor.sync_active_style_to_caret();
    }
    if let Some(word_offset) = ptr_state.word_select_target {
        app.editor.select_word_at(word_offset);
        app.last_typing_time = std::time::Instant::now();
    }
    app.last_caret_pos = ptr_state.next_caret_pos;

    ime_output_rect
}

#[derive(Default)]
struct PointerResolution {
    resolved: bool,
    next_caret_pos: Option<Pos2>,
    caret_target: Option<usize>,
    sel_target: Option<Option<usize>>,
    word_select_target: Option<usize>,
    sync_style: bool,
}

impl PointerResolution {
    fn apply_hit(
        &mut self,
        q: PointerQuery,
        target_byte: usize,
        keep_sel_vertical: bool,
        cur_selection_start: Option<usize>,
        cur_caret_byte_offset: usize,
    ) {
        match q {
            PointerQuery::Click(_) => {
                self.caret_target = Some(target_byte);
                self.sel_target = Some(None);
                self.sync_style = true;
            }
            PointerQuery::Drag(_) => {
                if cur_selection_start.is_none() {
                    self.sel_target = Some(Some(cur_caret_byte_offset));
                }
                self.caret_target = Some(target_byte);
            }
            PointerQuery::DoubleClick(_) => self.word_select_target = Some(target_byte),
            PointerQuery::MoveVertical(_) => {
                if keep_sel_vertical && cur_selection_start.is_none() {
                    self.sel_target = Some(Some(cur_caret_byte_offset));
                } else if !keep_sel_vertical {
                    self.sel_target = Some(None);
                }
                self.caret_target = Some(target_byte);
                self.sync_style = true;
            }
        }
    }
}

fn paint_page_chrome(
    painter: &egui::Painter,
    rect: Rect,
    cache: &CachedLayout,
    show_margins: bool,
    page_idx: usize,
    total_pages: usize,
) {
    let zoom = cache.zoom;
    let margin = PAGE_MARGIN_PT * zoom;
    let shadow1 = rect
        .translate(Vec2::new(0.0, 4.0 * zoom))
        .expand(6.0 * zoom);
    painter.rect_filled(shadow1, 14.0 * zoom, Color32::from_black_alpha(10));
    let shadow2 = rect
        .translate(Vec2::new(0.0, 10.0 * zoom))
        .expand(2.0 * zoom);
    painter.rect_filled(shadow2, 10.0 * zoom, Color32::from_black_alpha(16));
    let shadow3 = rect.translate(Vec2::new(0.0, 1.0 * zoom));
    painter.rect_filled(shadow3, 4.0 * zoom, Color32::from_black_alpha(22));

    painter.rect_filled(rect, 4.0, Color32::WHITE);
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0_f32, Color32::from_rgb(229, 233, 240)),
    );

    if show_margins {
        let origin = Pos2::new(rect.min.x + margin, rect.min.y + margin);
        let crop_len = 11.0 * zoom;
        let pad = 5.0 * zoom;
        let crop_stroke = Stroke::new(1.0_f32, Color32::from_rgb(203, 213, 225));
        let tl = Pos2::new(origin.x - pad, origin.y - pad);
        let tr = Pos2::new(origin.x + cache.usable_width + pad, origin.y - pad);
        let bl = Pos2::new(origin.x - pad, origin.y + cache.usable_height + pad);
        let br = Pos2::new(
            origin.x + cache.usable_width + pad,
            origin.y + cache.usable_height + pad,
        );

        painter.line_segment([Pos2::new(tl.x - crop_len, tl.y), tl], crop_stroke);
        painter.line_segment([Pos2::new(tl.x, tl.y - crop_len), tl], crop_stroke);
        painter.line_segment([Pos2::new(tr.x + crop_len, tr.y), tr], crop_stroke);
        painter.line_segment([Pos2::new(tr.x, tr.y - crop_len), tr], crop_stroke);
        painter.line_segment([Pos2::new(bl.x - crop_len, bl.y), bl], crop_stroke);
        painter.line_segment([Pos2::new(bl.x, bl.y + crop_len), bl], crop_stroke);
        painter.line_segment([Pos2::new(br.x + crop_len, br.y), br], crop_stroke);
        painter.line_segment([Pos2::new(br.x, br.y + crop_len), br], crop_stroke);
    }

    if total_pages > 1 {
        painter.text(
            Pos2::new(rect.center().x, rect.max.y - margin * 0.45),
            egui::Align2::CENTER_CENTER,
            format!("— {} —", page_idx + 1),
            FontId::proportional(11.0 * zoom),
            Color32::from_rgb(148, 163, 184),
        );
    }
}

#[inline]
fn pointer_pos(q: PointerQuery) -> Pos2 {
    match q {
        PointerQuery::Click(pt)
        | PointerQuery::Drag(pt)
        | PointerQuery::DoubleClick(pt)
        | PointerQuery::MoveVertical(pt) => pt,
    }
}

fn paint_ruler(
    painter: &egui::Painter,
    page_rect: Rect,
    cache: &CachedLayout,
    theme: &ThemePalette,
    caret_x: Option<f32>,
) {
    let zoom = cache.zoom;
    let margin = PAGE_MARGIN_PT * zoom;
    let ruler_h = (16.0 * zoom).max(14.0);
    let ruler_rect = Rect::from_min_max(
        Pos2::new(page_rect.min.x, page_rect.min.y - ruler_h - 4.0),
        Pos2::new(page_rect.max.x, page_rect.min.y - 4.0),
    );

    let bg_usable = if theme.is_dark {
        Color32::from_rgb(38, 42, 54)
    } else {
        Color32::from_rgb(248, 249, 252)
    };
    let bg_margin = if theme.is_dark {
        Color32::from_rgb(26, 29, 38)
    } else {
        Color32::from_rgb(236, 239, 245)
    };
    let tick_color = if theme.is_dark {
        Color32::from_rgb(100, 116, 139)
    } else {
        Color32::from_rgb(138, 148, 166)
    };

    // 1. Nền vùng lề trái và lề phải
    painter.rect_filled(ruler_rect, Rounding::same(2.0), bg_margin);
    // 2. Nền vùng viết khả dụng ở giữa
    let usable_rect = Rect::from_min_max(
        Pos2::new(ruler_rect.min.x + margin, ruler_rect.min.y),
        Pos2::new(ruler_rect.max.x - margin, ruler_rect.max.y),
    );
    painter.rect_filled(usable_rect, Rounding::ZERO, bg_usable);
    painter.rect_stroke(
        ruler_rect,
        Rounding::same(2.0),
        Stroke::new(1.0_f32, theme.border_color),
    );

    // 3. Vạch chia (mỗi 18 pt = 0.25 inch, mỗi 72 pt = 1 inch)
    let quarter_inch = 18.0 * zoom;
    let num_ticks = (cache.usable_width / quarter_inch).floor() as usize;

    for i in 0..=num_ticks {
        let x = usable_rect.min.x + (i as f32) * quarter_inch;
        if x > usable_rect.max.x {
            break;
        }
        let is_major = i % 4 == 0;
        let is_half = i % 2 == 0;
        let tick_len = if is_major {
            ruler_h * 0.55
        } else if is_half {
            ruler_h * 0.35
        } else {
            ruler_h * 0.20
        };

        painter.line_segment(
            [
                Pos2::new(x, ruler_rect.max.y - tick_len),
                Pos2::new(x, ruler_rect.max.y),
            ],
            Stroke::new(1.0_f32, tick_color),
        );

        if is_major && i > 0 {
            let inch_num = i / 4;
            painter.text(
                Pos2::new(x, ruler_rect.min.y + 1.0),
                egui::Align2::CENTER_TOP,
                format!("{}", inch_num),
                FontId::proportional((8.0 * zoom).max(7.0)),
                tick_color,
            );
        }
    }

    // 4. Con trượt lề hình tam giác tại vị trí lề trái và phải
    let mark_l = usable_rect.min.x;
    let mark_r = usable_rect.max.x;
    for &mx in &[mark_l, mark_r] {
        let tri = vec![
            Pos2::new(mx - 3.5, ruler_rect.max.y),
            Pos2::new(mx + 3.5, ruler_rect.max.y),
            Pos2::new(mx, ruler_rect.max.y - 5.0),
        ];
        painter.add(Shape::convex_polygon(
            tri,
            theme.primary_accent,
            Stroke::NONE,
        ));
    }

    // 5. Vạch chỉ vị trí Caret con trỏ hiện tại chiếu lên thước
    if let Some(cx) = caret_x
        && cx >= usable_rect.min.x
        && cx <= usable_rect.max.x
    {
        painter.line_segment(
            [
                Pos2::new(cx, ruler_rect.min.y),
                Pos2::new(cx, ruler_rect.max.y),
            ],
            Stroke::new(1.2_f32, theme.primary_accent),
        );
    }
}
