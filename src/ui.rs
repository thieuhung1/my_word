use crate::app::MyWordApp;
use crate::document::{PAGE_BREAK_CHAR, TextAlignment};
use crate::editor::InputMethodMode;
use crate::icons::{IconKind, draw_app_logo, icon_button};
use crate::layout::{
    A4_HEIGHT_PT, A4_WIDTH_PT, CachedLayout, PAGE_MARGIN_PT, PointerQuery,
    byte_offset_from_char_index, char_index_from_byte_offset, font_id_for_style,
    paint_range_highlight,
};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Rounding, Stroke, Vec2};

pub struct ThemePalette {
    pub is_dark: bool,
    pub surface_bg: Color32,
    pub toolbar_bg: Color32,
    pub workspace_bg: Color32,
    pub border_color: Color32,
    pub text_main: Color32,
    pub text_muted: Color32,
    pub primary_accent: Color32,
}

impl ThemePalette {
    pub fn from_mode(is_dark: bool) -> Self {
        if is_dark {
            Self {
                is_dark: true,
                surface_bg: Color32::from_rgb(26, 29, 36),
                toolbar_bg: Color32::from_rgb(22, 24, 30),
                workspace_bg: Color32::from_rgb(17, 19, 24),
                border_color: Color32::from_rgb(46, 51, 64),
                text_main: Color32::from_rgb(241, 245, 249),
                text_muted: Color32::from_rgb(148, 163, 184),
                primary_accent: Color32::from_rgb(37, 99, 235),
            }
        } else {
            Self {
                is_dark: false,
                surface_bg: Color32::WHITE,
                toolbar_bg: Color32::from_rgb(248, 250, 252),
                workspace_bg: Color32::from_rgb(239, 243, 248),
                border_color: Color32::from_rgb(226, 232, 240),
                text_main: Color32::from_rgb(30, 41, 59),
                text_muted: Color32::from_rgb(100, 116, 139),
                primary_accent: Color32::from_rgb(37, 99, 235),
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
        ctx.set_visuals(visuals);
    }
}

pub fn render_header_bar(ctx: &egui::Context, app: &mut MyWordApp, theme: &ThemePalette) {
    egui::TopBottomPanel::top("app_header")
        .frame(
            egui::Frame::none()
                .fill(theme.surface_bg)
                .inner_margin(egui::Margin::symmetric(16.0, 8.0))
                .stroke(Stroke::new(1.0_f32, theme.border_color)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                draw_app_logo(ui);
                ui.add_space(6.0);
                let doc_name = app
                    .current_file_path
                    .as_ref()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("Tài liệu 1");
                let dirty_mark = if app.editor.is_dirty { " *" } else { "" };
                ui.label(
                    egui::RichText::new(format!("{}{}", doc_name, dirty_mark))
                        .size(15.0)
                        .color(theme.text_main)
                        .strong(),
                );
                ui.label(egui::RichText::new("•").color(theme.text_muted));
                ui.label(
                    egui::RichText::new("Rust Word")
                        .size(12.0)
                        .color(theme.text_muted),
                );
                ui.add_space(8.0);

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
                        if theme.is_dark {
                            Color32::from_rgb(23, 37, 84)
                        } else {
                            Color32::from_rgb(239, 246, 255)
                        },
                        if theme.is_dark {
                            Color32::from_rgb(59, 130, 246)
                        } else {
                            Color32::from_rgb(191, 219, 254)
                        },
                        if theme.is_dark {
                            Color32::from_rgb(147, 197, 253)
                        } else {
                            Color32::from_rgb(29, 78, 216)
                        },
                    ),
                };
                if ui
                    .add(
                        egui::Button::new(
                            egui::RichText::new(format!("● {}", mode_label))
                                .size(11.5)
                                .color(mode_fg)
                                .strong(),
                        )
                        .fill(mode_bg)
                        .stroke(Stroke::new(1.0_f32, mode_border))
                        .rounding(Rounding::same(14.0)),
                    )
                    .on_hover_text("Nhấn để chuyển giữa bộ gõ Telex tự động và thuần Hệ thống")
                    .clicked()
                {
                    app.editor.input_mode = match app.editor.input_mode {
                        InputMethodMode::AutoTelex => InputMethodMode::SystemOnly,
                        InputMethodMode::SystemOnly => InputMethodMode::AutoTelex,
                    };
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                    if icon_button(
                        ui,
                        IconKind::Save,
                        "Lưu tài liệu (Ctrl+S / Ctrl+Shift+S)",
                        false,
                        theme.is_dark,
                    )
                    .clicked()
                    {
                        app.trigger_save(ctx.clone(), false);
                    }
                    if icon_button(
                        ui,
                        IconKind::FolderOpen,
                        "Mở tệp .mwd / .txt / .md (Ctrl+O)",
                        false,
                        theme.is_dark,
                    )
                    .clicked()
                    {
                        app.trigger_open_dialog(ctx.clone());
                    }
                });
            });
        });
}

pub fn render_ribbon_bar(ctx: &egui::Context, app: &mut MyWordApp, theme: &ThemePalette) {
    let is_dark = theme.is_dark;
    egui::TopBottomPanel::top("ribbon_panel")
        .frame(
            egui::Frame::none()
                .fill(theme.toolbar_bg)
                .inner_margin(egui::Margin::symmetric(16.0, 6.0))
                .stroke(Stroke::new(1.0_f32, theme.border_color)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(3.5, 4.0);
                ui.style_mut().spacing.interact_size.y = 26.0;

                ui.add_enabled_ui(!app.editor.undo_stack.is_empty(), |ui| {
                    if icon_button(ui, IconKind::Undo, "Hoàn tác (Ctrl+Z)", false, is_dark)
                        .clicked()
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
                ui.separator();

                let has_selection = app.editor.selected_range().is_some();
                ui.add_enabled_ui(has_selection, |ui| {
                    if icon_button(ui, IconKind::Cut, "Cắt (Ctrl+X)", false, is_dark).clicked() {
                        app.cut_selection_to_clipboard(ctx);
                    }
                    if icon_button(ui, IconKind::Copy, "Sao chép (Ctrl+C)", false, is_dark)
                        .clicked()
                    {
                        app.copy_selection_to_clipboard(ctx);
                    }
                });
                ui.add_enabled_ui(!app.editor.clipboard_buffer.is_empty(), |ui| {
                    if icon_button(ui, IconKind::Paste, "Dán (Ctrl+V)", false, is_dark).clicked() {
                        let text = app.editor.clipboard_buffer.clone();
                        app.editor.insert_text(&text);
                    }
                });
                ui.separator();

                if ui
                    .add(egui::Button::new(egui::RichText::new("−").size(13.0)).rounding(5.0))
                    .on_hover_text("Giảm cỡ chữ")
                    .clicked()
                {
                    let next = (app.editor.active_style.font_size - 1.0).max(8.0);
                    app.editor.apply_format(|s| s.font_size = next);
                }
                ui.label(
                    egui::RichText::new(format!("{:.0} pt", app.editor.active_style.font_size))
                        .size(12.5)
                        .color(theme.text_main)
                        .strong(),
                );
                if ui
                    .add(egui::Button::new(egui::RichText::new("+").size(13.0)).rounding(5.0))
                    .on_hover_text("Tăng cỡ chữ")
                    .clicked()
                {
                    let next = (app.editor.active_style.font_size + 1.0).min(48.0);
                    app.editor.apply_format(|s| s.font_size = next);
                }

                let style_toggle =
                    |ui: &mut egui::Ui, label: egui::RichText, active: bool, tip: &str| {
                        let mut btn = egui::Button::new(label)
                            .min_size(Vec2::new(25.0, 26.0))
                            .rounding(6.0);
                        if active {
                            btn = btn
                                .fill(if is_dark {
                                    Color32::from_rgb(30, 58, 138)
                                } else {
                                    Color32::from_rgb(238, 242, 255)
                                })
                                .stroke(Stroke::new(1.0_f32, theme.primary_accent));
                        }
                        ui.add(btn).on_hover_text(tip).clicked()
                    };

                if style_toggle(
                    ui,
                    egui::RichText::new("B").strong().size(13.0),
                    app.editor.active_style.bold,
                    "In đậm (Ctrl+B)",
                ) {
                    let next = !app.editor.active_style.bold;
                    app.editor.apply_format(|s| s.bold = next);
                }
                if style_toggle(
                    ui,
                    egui::RichText::new("I").italics().size(13.0),
                    app.editor.active_style.italic,
                    "In nghiêng (Ctrl+I)",
                ) {
                    let next = !app.editor.active_style.italic;
                    app.editor.apply_format(|s| s.italic = next);
                }
                if style_toggle(
                    ui,
                    egui::RichText::new("U").underline().size(13.0),
                    app.editor.active_style.underline,
                    "Gạch chân (Ctrl+U)",
                ) {
                    let next = !app.editor.active_style.underline;
                    app.editor.apply_format(|s| s.underline = next);
                }
                if style_toggle(
                    ui,
                    egui::RichText::new("S").strikethrough().size(13.0),
                    app.editor.active_style.strikethrough,
                    "Gạch ngang chữ",
                ) {
                    let next = !app.editor.active_style.strikethrough;
                    app.editor.apply_format(|s| s.strikethrough = next);
                }

                for (rgb, tip) in [
                    ([30, 41, 59], "Màu mặc định"),
                    ([37, 99, 235], "Màu xanh"),
                    ([220, 38, 38], "Màu đỏ"),
                ] {
                    let selected = app.editor.active_style.color == rgb;
                    let (c_rect, c_resp) =
                        ui.allocate_exact_size(Vec2::new(16.0, 16.0), egui::Sense::click());
                    ui.painter().circle_filled(
                        c_rect.center(),
                        6.0,
                        Color32::from_rgb(rgb[0], rgb[1], rgb[2]),
                    );
                    if selected {
                        ui.painter().circle_stroke(
                            c_rect.center(),
                            8.0,
                            Stroke::new(1.5_f32, theme.primary_accent),
                        );
                    }
                    if c_resp.on_hover_text(tip).clicked() {
                        app.editor.apply_format(|s| s.color = rgb);
                    }
                }

                let hl_on = app.editor.active_style.highlight.is_some();
                if icon_button(ui, IconKind::Highlight, "Tô sáng Highlight", hl_on, is_dark)
                    .clicked()
                {
                    let next_hl = if hl_on { None } else { Some([254, 240, 138]) };
                    app.editor.apply_format(|s| s.highlight = next_hl);
                }
                ui.separator();

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
                ui.separator();

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
                if icon_button(
                    ui,
                    IconKind::Margins,
                    "Góc căn lề A4",
                    app.show_margins,
                    is_dark,
                )
                .clicked()
                {
                    app.show_margins = !app.show_margins;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let zoom_pct = format!("{:.0}%", app.zoom * 100.0);
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new(zoom_pct)
                                    .size(12.0)
                                    .color(theme.text_main),
                            )
                            .rounding(6.0),
                        )
                        .on_hover_text("Đặt lại 100%")
                        .clicked()
                    {
                        app.zoom = 1.0;
                    }
                    ui.add(egui::Slider::new(&mut app.zoom, 0.6_f32..=1.5_f32).show_value(false));
                });
            });
        });
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
                    if ui.button("Mở Thay thế ▾").clicked() {
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
                        "⚠️ Phát hiện bản sao lưu tự động chưa được lưu từ phiên làm việc trước.",
                    )
                    .size(13.0)
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
    } else if do_discard {
        app.pending_confirm = None;
        app.editor.is_dirty = false;
        match action {
            crate::app::ConfirmAction::Quit => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            crate::app::ConfirmAction::OpenNew => {
                app.trigger_open_dialog(ctx.clone());
            }
            crate::app::ConfirmAction::NewDocument => {
                app.new_document();
            }
        }
    } else if do_save {
        app.pending_confirm = None;
        if let Some(path) = app.current_file_path.clone() {
            app.write_to_path(&path);
            match action {
                crate::app::ConfirmAction::Quit => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                crate::app::ConfirmAction::OpenNew => {
                    app.trigger_open_dialog(ctx.clone());
                }
                crate::app::ConfirmAction::NewDocument => {
                    app.new_document();
                }
            }
        } else {
            app.post_save_action = Some(action);
            app.file_dialog_rx = Some(crate::platform::spawn_save_dialog(ctx.clone()));
        }
    }
}

pub fn render_status_bar(
    ctx: &egui::Context,
    app: &MyWordApp,
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
                        .size(12.0)
                        .color(theme.text_main)
                        .strong(),
                );
                ui.separator();
                ui.label(
                    egui::RichText::new(format!("{} từ  •  {} ký tự", word_count, char_count))
                        .size(12.0)
                        .color(theme.text_muted),
                );
                ui.separator();
                ui.label(
                    egui::RichText::new("UTF-8 NFC")
                        .size(12.0)
                        .color(theme.text_muted),
                );

                if app.editor.unikey_backspace_count > 0 {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!(
                            "Backspace: {}",
                            app.editor.unikey_backspace_count
                        ))
                        .size(12.0)
                        .color(theme.primary_accent),
                    );
                }
                if let Some((msg, created_at)) = &app.status_message
                    && created_at.elapsed().as_secs() < 4
                {
                    ui.separator();
                    ui.label(
                        egui::RichText::new(msg)
                            .color(theme.primary_accent)
                            .strong()
                            .size(12.0),
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("{:.0} FPS", app.fps))
                            .size(11.5)
                            .color(theme.text_muted),
                    );
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!(
                            "A4 (210 × 297 mm)  •  {:.0}%",
                            app.zoom * 100.0
                        ))
                        .size(11.5)
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
    }
    if let Some(c) = ptr_state.caret_target {
        app.editor.caret_byte_offset = c;
        app.last_typing_time = std::time::Instant::now();
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
        Stroke::new(1.0_f32, Color32::from_rgb(220, 225, 232)),
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
