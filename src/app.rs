use crate::document::{PAGE_BREAK_CHAR, find_matches};
use crate::editor::EditorState;
use crate::layout::{CachedLayout, PointerQuery, caret_page_index, ensure_layout_cache};
use crate::platform::{
    FileDialogResult, clear_autosave, has_autosave, load_document, read_autosave,
    spawn_open_dialog, spawn_save_dialog, write_autosave, write_document,
};
use crate::ui::{
    ThemePalette, render_confirm_modal, render_find_replace_bar, render_header_bar,
    render_page_canvas, render_recovery_banner, render_ribbon_bar, render_status_bar,
};
use eframe::egui::{self, Pos2};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ConfirmAction {
    Quit,
    OpenNew,
    NewDocument,
}

pub const CARET_BLINK_MS: u64 = 550;
pub const AUTOSAVE_INTERVAL_SECS: u64 = 20;
pub const UNIKEY_BURST_WINDOW_MS: u64 = 45;
const DEFAULT_LINE_HEIGHT_PT: f32 = 22.0;

pub struct MyWordApp {
    pub editor: EditorState,
    pub last_typing_time: Instant,
    pub last_frame_time: Instant,
    pub fps: f32,
    pub zoom: f32,
    pub show_margins: bool,
    pub show_ruler: bool,
    pub dark_workspace: bool,
    pub status_message: Option<(String, Instant)>,
    pub pending_pointer_query: Option<PointerQuery>,
    pub last_caret_pos: Option<Pos2>,
    pub last_line_height: f32,
    pub pending_keep_selection: bool,
    pub current_file_path: Option<PathBuf>,
    pub last_autosave: Instant,
    pub file_dialog_rx: Option<Receiver<Option<FileDialogResult>>>,
    pub find_bar_open: bool,
    pub replace_bar_open: bool,
    pub find_query: String,
    pub replace_query: String,
    pub find_case_sensitive: bool,
    pub active_match_idx: usize,
    pub focus_find_input: bool,
    pub cached_layout: Option<CachedLayout>,
    pub cached_matches: Vec<(usize, usize)>,
    pub pending_confirm: Option<ConfirmAction>,
    pub post_save_action: Option<ConfirmAction>,
    pub show_recovery_banner: bool,
    pub last_external_backspace: Option<Instant>,
    pub scroll_to_caret: bool,
}

impl Default for MyWordApp {
    fn default() -> Self {
        let has_saved = has_autosave();
        Self {
            editor: EditorState::default(),
            last_typing_time: Instant::now(),
            last_frame_time: Instant::now(),
            fps: 60.0,
            zoom: 1.0,
            show_margins: true,
            show_ruler: true,
            dark_workspace: false,
            status_message: Some((
                "Sẵn sàng soạn thảo tiếng Việt Telex".to_string(),
                Instant::now(),
            )),
            pending_pointer_query: None,
            last_caret_pos: None,
            last_line_height: DEFAULT_LINE_HEIGHT_PT,
            pending_keep_selection: false,
            current_file_path: None,
            last_autosave: Instant::now(),
            file_dialog_rx: None,
            find_bar_open: false,
            replace_bar_open: false,
            find_query: String::new(),
            replace_query: String::new(),
            find_case_sensitive: false,
            active_match_idx: 0,
            focus_find_input: false,
            cached_layout: None,
            cached_matches: Vec::new(),
            pending_confirm: None,
            post_save_action: None,
            show_recovery_banner: has_saved,
            last_external_backspace: None,
            scroll_to_caret: false,
        }
    }
}

impl MyWordApp {
    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), Instant::now()));
    }

    pub fn copy_selection_to_clipboard(&mut self, ctx: &egui::Context) {
        if let Some(text) = self.editor.copy_selection() {
            ctx.output_mut(|o| o.copied_text = text.clone());
            self.editor.clipboard_buffer = text;
            self.set_status("Đã sao chép vào Clipboard");
        }
    }

    pub fn cut_selection_to_clipboard(&mut self, ctx: &egui::Context) {
        if let Some(text) = self.editor.copy_selection() {
            self.editor.save_undo();
            ctx.output_mut(|o| o.copied_text = text.clone());
            self.editor.clipboard_buffer = text;
            self.editor.delete_selection();
            self.set_status("Đã cắt vào Clipboard");
        }
    }

    pub fn refresh_find_cache(&mut self) {
        if self.editor.find_dirty {
            self.cached_matches = if self.find_query.is_empty() {
                Vec::new()
            } else {
                find_matches(
                    &self.editor.content,
                    &self.find_query,
                    self.find_case_sensitive,
                )
            };
            self.editor.find_dirty = false;
        }
        if self.cached_matches.is_empty() || self.active_match_idx >= self.cached_matches.len() {
            self.active_match_idx = 0;
        }
    }

    pub fn recover_autosave(&mut self) {
        match read_autosave() {
            Ok((text, spans, aligns)) => {
                self.editor.save_undo();
                self.editor.content = text;
                self.editor.spans = spans;
                self.editor.para_alignments = aligns;
                self.editor.caret_byte_offset = 0;
                self.editor.selection_start = None;
                self.editor.mark_content_changed();
                self.editor.is_dirty = true;
                self.editor.sync_active_style_to_caret();
                self.cached_layout = None;
                self.show_recovery_banner = false;
                self.set_status("Đã khôi phục phiên soạn thảo chưa lưu");
            }
            Err(e) => {
                self.show_recovery_banner = false;
                self.set_status(&format!("Không đọc được bản sao lưu: {}", e));
            }
        }
    }

    pub fn discard_autosave(&mut self) {
        clear_autosave();
        self.show_recovery_banner = false;
        self.set_status("Đã bỏ qua bản sao lưu");
    }

    pub fn new_document(&mut self) {
        self.editor.save_undo();
        self.editor.content.clear();
        self.editor.spans.clear();
        self.editor.para_alignments.clear();
        self.editor.caret_byte_offset = 0;
        self.editor.selection_start = None;
        self.current_file_path = None;
        self.editor.is_dirty = false;
        self.editor.mark_content_changed();
        self.cached_layout = None;
        self.editor.ime_preedit = None;
        self.active_match_idx = 0;
        self.set_status("Tài liệu mới");
    }

    pub fn trigger_new_document(&mut self) {
        if self.editor.is_dirty {
            self.pending_confirm = Some(ConfirmAction::NewDocument);
        } else {
            self.new_document();
        }
    }

    pub fn trigger_open_dialog(&mut self, ctx: egui::Context) {
        if self.editor.is_dirty {
            self.pending_confirm = Some(ConfirmAction::OpenNew);
            return;
        }
        if self.file_dialog_rx.is_none() {
            self.file_dialog_rx = Some(spawn_open_dialog(ctx));
        }
    }

    pub fn trigger_save(&mut self, ctx: egui::Context, force_save_as: bool) {
        if !force_save_as && let Some(path) = self.current_file_path.clone() {
            self.write_to_path(&path);
            return;
        }
        if self.file_dialog_rx.is_none() {
            self.file_dialog_rx = Some(spawn_save_dialog(ctx));
        }
    }

    pub fn write_to_path(&mut self, path: &Path) {
        match write_document(path, &self.editor) {
            Ok(()) => {
                self.current_file_path = Some(path.to_path_buf());
                self.editor.is_dirty = false;
                clear_autosave();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("tệp");
                self.set_status(&format!("Đã lưu: {}", name));
            }
            Err(e) => self.set_status(&format!("Lỗi lưu tệp: {}", e)),
        }
    }

    fn load_from_path(&mut self, path: &Path) {
        match load_document(path) {
            Ok((text, spans, aligns)) => {
                self.editor.save_undo();
                self.editor.content = text;
                self.editor.spans = spans;
                self.editor.para_alignments = aligns;
                self.editor.caret_byte_offset = 0;
                self.editor.selection_start = None;
                self.current_file_path = Some(path.to_path_buf());
                self.editor.mark_content_changed();
                self.editor.is_dirty = false;
                self.editor.sync_active_style_to_caret();
                self.cached_layout = None;
                self.editor.ime_preedit = None;
                self.active_match_idx = 0;
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("tệp");
                self.set_status(&format!("Đã mở: {}", name));
            }
            Err(e) => self.set_status(&format!("Không đọc được tệp: {}", e)),
        }
    }

    fn maybe_autosave(&mut self) {
        if self.show_recovery_banner
            || !self.editor.is_dirty
            || self.last_autosave.elapsed().as_secs() < AUTOSAVE_INTERVAL_SECS
        {
            return;
        }
        self.last_autosave = Instant::now();
        write_autosave(&self.editor);
    }

    fn poll_file_dialog(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.file_dialog_rx {
            match rx.try_recv() {
                Ok(maybe_res) => {
                    self.file_dialog_rx = None;
                    if let Some(res) = maybe_res {
                        match res {
                            FileDialogResult::Open(p) => self.load_from_path(&p),
                            FileDialogResult::Save(p) => {
                                self.write_to_path(&p);
                                if let Some(action) = self.post_save_action.take() {
                                    match action {
                                        ConfirmAction::Quit => {
                                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                        }
                                        ConfirmAction::OpenNew => {
                                            self.trigger_open_dialog(ctx.clone());
                                        }
                                        ConfirmAction::NewDocument => {
                                            self.new_document();
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        self.post_save_action = None;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.file_dialog_rx = None;
                    self.post_save_action = None;
                }
            }
        }
    }

    pub fn move_to_line_boundary(&mut self, to_end: bool, keep_selection: bool) {
        if keep_selection && self.editor.selection_start.is_none() {
            self.editor.selection_start = Some(self.editor.caret_byte_offset);
        } else if !keep_selection {
            self.editor.selection_start = None;
        }

        let target_byte = if let Some(cache) = &self.cached_layout {
            let (b_start, b_end) =
                cache.line_boundary_at(&self.editor.content, self.editor.caret_byte_offset);
            if to_end { b_end } else { b_start }
        } else if to_end {
            self.editor.content.len()
        } else {
            0
        };

        self.editor.caret_byte_offset = target_byte;
        self.editor.sync_active_style_to_caret();
        self.scroll_to_caret = true;
    }

    pub fn move_caret_vertical(&mut self, is_up: bool, keep_selection: bool) {
        if keep_selection && self.editor.selection_start.is_none() {
            self.editor.selection_start = Some(self.editor.caret_byte_offset);
        } else if !keep_selection {
            self.editor.selection_start = None;
        }

        if let Some(cache) = &self.cached_layout
            && let Some(target) = cache.navigate_vertical_line(
                &self.editor.content,
                self.editor.caret_byte_offset,
                is_up,
            )
        {
            self.editor.caret_byte_offset = target;
            self.editor.sync_active_style_to_caret();
            self.scroll_to_caret = true;
            return;
        }

        if let Some(pos) = self.last_caret_pos {
            self.pending_keep_selection = keep_selection;
            let offset_y = if is_up {
                -self.last_line_height * 0.75
            } else {
                self.last_line_height * 1.25
            };
            self.pending_pointer_query = Some(PointerQuery::MoveVertical(Pos2::new(
                pos.x,
                pos.y + offset_y,
            )));
            self.scroll_to_caret = true;
        }
    }

    fn process_events(&mut self, ctx: &egui::Context, ui_wants_kb: bool) {
        let events = ctx.input(|i| i.events.clone());

        let mut saw_backspace_in_frame = false;
        for ev in &events {
            if let egui::Event::Key {
                key: egui::Key::Backspace,
                pressed: true,
                modifiers,
                ..
            } = ev
                && !modifiers.command
            {
                saw_backspace_in_frame = true;
                self.last_external_backspace = Some(Instant::now());
                break;
            }
        }

        let is_recent_backspace = saw_backspace_in_frame
            || self
                .last_external_backspace
                .is_some_and(|t| t.elapsed() < Duration::from_millis(UNIKEY_BURST_WINDOW_MS));

        for event in events {
            match event {
                egui::Event::Key {
                    key: egui::Key::F,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command => {
                    self.find_bar_open = true;
                    self.focus_find_input = true;
                    continue;
                }
                egui::Event::Key {
                    key: egui::Key::H,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command => {
                    self.find_bar_open = true;
                    self.replace_bar_open = true;
                    self.focus_find_input = true;
                    continue;
                }
                egui::Event::Key {
                    key: egui::Key::O,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command => {
                    self.trigger_open_dialog(ctx.clone());
                    continue;
                }
                egui::Event::Key {
                    key: egui::Key::N,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command => {
                    self.trigger_new_document();
                    continue;
                }
                egui::Event::Key {
                    key: egui::Key::S,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command => {
                    self.trigger_save(ctx.clone(), modifiers.shift);
                    continue;
                }
                egui::Event::Key {
                    key: egui::Key::Escape,
                    pressed: true,
                    ..
                } if self.find_bar_open => {
                    self.find_bar_open = false;
                    self.replace_bar_open = false;
                    continue;
                }
                _ => {}
            }

            if ui_wants_kb {
                continue;
            }

            match event {
                egui::Event::Text(t) => {
                    self.editor.ime_preedit = None;
                    self.last_typing_time = Instant::now();
                    self.editor.handle_text_input(&t, !is_recent_backspace);
                    self.scroll_to_caret = true;
                }
                egui::Event::Copy => self.copy_selection_to_clipboard(ctx),
                egui::Event::Cut => {
                    self.last_typing_time = Instant::now();
                    self.cut_selection_to_clipboard(ctx);
                    self.scroll_to_caret = true;
                }
                egui::Event::Paste(pasted) => {
                    self.last_typing_time = Instant::now();
                    let clean = pasted.replace("\r\n", "\n").replace('\r', "\n");
                    self.editor.clipboard_buffer = clean.clone();
                    self.editor.insert_text(&clean);
                    self.scroll_to_caret = true;
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    self.last_typing_time = Instant::now();
                    if self.editor.ime_preedit.is_none() {
                        self.handle_key_event(key, modifiers);
                    }
                }
                egui::Event::Ime(ime_event) => match ime_event {
                    egui::ImeEvent::Preedit(text) => {
                        self.last_typing_time = Instant::now();
                        self.editor.ime_preedit = if text.is_empty() { None } else { Some(text) };
                        self.scroll_to_caret = true;
                    }
                    egui::ImeEvent::Commit(text) => {
                        self.last_typing_time = Instant::now();
                        self.editor.ime_preedit = None;
                        self.editor.insert_text(&text);
                        self.scroll_to_caret = true;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn handle_key_event(&mut self, key: egui::Key, modifiers: egui::Modifiers) {
        self.scroll_to_caret = true;
        if modifiers.command {
            match key {
                egui::Key::Z => {
                    let changed = if modifiers.shift {
                        self.editor.redo()
                    } else {
                        self.editor.undo()
                    };
                    if changed {
                        self.set_status(if modifiers.shift {
                            "Đã làm lại"
                        } else {
                            "Đã hoàn tác"
                        });
                    }
                }
                egui::Key::Y => {
                    if self.editor.redo() {
                        self.set_status("Đã làm lại");
                    }
                }
                egui::Key::A => {
                    self.editor.select_all();
                    self.set_status("Đã chọn toàn bộ văn bản");
                }
                egui::Key::B => {
                    let next = !self.editor.active_style.bold;
                    self.editor.apply_format(|s| s.bold = next);
                }
                egui::Key::I => {
                    let next = !self.editor.active_style.italic;
                    self.editor.apply_format(|s| s.italic = next);
                }
                egui::Key::U => {
                    let next = !self.editor.active_style.underline;
                    self.editor.apply_format(|s| s.underline = next);
                }
                egui::Key::Enter => {
                    self.editor.insert_text(&PAGE_BREAK_CHAR.to_string());
                    self.set_status("Đã chèn ngắt trang");
                }
                egui::Key::Backspace => self.editor.delete_word_backward(),
                egui::Key::Delete => self.editor.delete_word_forward(),
                egui::Key::ArrowLeft => {
                    self.editor.move_word_left_with_selection(modifiers.shift);
                }
                egui::Key::ArrowRight => {
                    self.editor.move_word_right_with_selection(modifiers.shift);
                }
                egui::Key::Home => {
                    self.editor.move_to_doc_boundary(false, modifiers.shift);
                }
                egui::Key::End => {
                    self.editor.move_to_doc_boundary(true, modifiers.shift);
                }
                _ => {}
            }
        } else {
            match key {
                egui::Key::Backspace => self.editor.backspace(),
                egui::Key::Delete => self.editor.delete_forward(),
                egui::Key::ArrowLeft => self.editor.move_left_with_selection(modifiers.shift),
                egui::Key::ArrowRight => self.editor.move_right_with_selection(modifiers.shift),
                egui::Key::ArrowUp => self.move_caret_vertical(true, modifiers.shift),
                egui::Key::ArrowDown => self.move_caret_vertical(false, modifiers.shift),
                egui::Key::Home => self.move_to_line_boundary(false, modifiers.shift),
                egui::Key::End => self.move_to_line_boundary(true, modifiers.shift),
                egui::Key::Enter => self.editor.insert_text("\n"),
                _ => {}
            }
        }
    }
}

impl eframe::App for MyWordApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) && self.editor.is_dirty {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending_confirm = Some(ConfirmAction::Quit);
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        if dt > 0.0 {
            self.fps = self.fps * 0.9 + (1.0 / dt) * 0.1;
        }
        self.maybe_autosave();
        self.poll_file_dialog(ctx);

        let ui_wants_kb = ctx.wants_keyboard_input();
        self.process_events(ctx, ui_wants_kb);

        let theme = ThemePalette::from_mode(self.dark_workspace);
        render_header_bar(ctx, self, &theme);
        render_ribbon_bar(ctx, self, &theme);
        render_recovery_banner(ctx, self, &theme);
        render_find_replace_bar(ctx, self, &theme);

        let (total_pages, caret_page_idx, word_count, char_count) = {
            let cache =
                ensure_layout_cache(&mut self.cached_layout, &mut self.editor, self.zoom, ctx);
            (
                cache.pages.len().max(1),
                caret_page_index(cache, &self.editor.content, self.editor.caret_byte_offset),
                cache.word_count,
                cache.char_count,
            )
        };

        render_status_bar(
            ctx,
            self,
            &theme,
            caret_page_idx,
            total_pages,
            word_count,
            char_count,
        );

        let elapsed_ms = self.last_typing_time.elapsed().as_millis() as u64;
        let show_caret = (elapsed_ms / CARET_BLINK_MS).is_multiple_of(2);
        let ime_output_rect = render_page_canvas(ctx, self, &theme, total_pages, show_caret);

        if !ui_wants_kb && let Some((area_rect, cursor_rect)) = ime_output_rect {
            ctx.output_mut(|o| {
                o.ime = Some(egui::output::IMEOutput {
                    rect: area_rect,
                    cursor_rect,
                });
            });
        }

        render_confirm_modal(ctx, self, &theme);

        let next_blink_ms = (CARET_BLINK_MS - (elapsed_ms % CARET_BLINK_MS)).max(16);
        ctx.request_repaint_after(Duration::from_millis(next_blink_ms));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_confirmation_and_new_document() {
        let mut app = MyWordApp::default();
        app.editor.is_dirty = true;
        app.trigger_new_document();
        assert_eq!(app.pending_confirm, Some(ConfirmAction::NewDocument));

        let ctx = egui::Context::default();
        app.trigger_open_dialog(ctx);
        assert_eq!(app.pending_confirm, Some(ConfirmAction::OpenNew));

        app.editor.is_dirty = false;
        app.pending_confirm = None;
        app.editor.content = "Nội dung cũ".to_string();
        app.trigger_new_document();
        assert_eq!(app.pending_confirm, None);
        assert!(app.editor.content.is_empty());
    }

    #[test]
    fn test_typing_resets_last_typing_time_and_clears_ime_preedit() {
        let mut app = MyWordApp::default();
        app.editor.ime_preedit = Some("tam".to_string());
        let before = Instant::now() - Duration::from_secs(5);
        app.last_typing_time = before;

        let ctx = egui::Context::default();
        ctx.input_mut(|i| {
            i.events.push(egui::Event::Text("t".to_string()));
        });
        app.process_events(&ctx, false);

        assert!(app.editor.ime_preedit.is_none());
        assert!(app.last_typing_time > before);
        assert!(app.last_typing_time.elapsed().as_millis() < 550);
    }
}
