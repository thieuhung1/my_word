use crate::document::{PAGE_BREAK_CHAR, find_matches};
use crate::editor::EditorState;
use crate::layout::{CachedLayout, PointerQuery, caret_page_index, ensure_layout_cache};
use crate::platform::{
    FileDialogResult, clear_autosave, has_autosave, load_document, read_autosave,
    spawn_open_dialog, spawn_save_dialog, write_autosave, write_document,
};
use crate::ui::{
    ThemePalette, render_confirm_modal, render_find_replace_bar, render_header_bar,
    render_inspector_sidebar, render_navigation_sidebar, render_page_canvas,
    render_recovery_banner, render_ribbon_bar, render_startup_modal, render_status_bar,
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RibbonTab {
    File,
    Home,
    Insert,
    Layout,
    View,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NavTab {
    Outline,
    Pages,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InspectorTab {
    Text,
    Page,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StartupNavTab {
    Recent,
    Templates,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineHeading {
    pub byte_offset: usize,
    pub title: String,
    pub level: u8,
}

pub const CARET_BLINK_MS: u64 = 550;
pub const AUTOSAVE_INTERVAL_SECS: u64 = 20;
pub const UNIKEY_BURST_WINDOW_MS: u64 = 45;
pub const STATUS_MESSAGE_TIMEOUT_SECS: u64 = 5;
pub const MIN_ZOOM: f32 = 0.5;
pub const MAX_ZOOM: f32 = 3.0;
pub const ZOOM_STEP: f32 = 0.1;
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
    pub pending_drop_path: Option<PathBuf>,
    pub show_recovery_banner: bool,
    pub show_startup_modal: bool,
    pub last_external_backspace: Option<Instant>,
    pub scroll_to_caret: bool,
    pub active_ribbon_tab: RibbonTab,
    pub focus_mode: bool,
    pub show_nav_panel: bool,
    pub show_inspector_panel: bool,
    pub nav_tab: NavTab,
    pub inspector_tab: InspectorTab,
    pub startup_nav_tab: StartupNavTab,
}

impl Default for MyWordApp {
    fn default() -> Self {
        let has_saved = has_autosave();
        Self {
            editor: EditorState::new_empty(),
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
            pending_drop_path: None,
            show_recovery_banner: has_saved,
            show_startup_modal: true,
            last_external_backspace: None,
            scroll_to_caret: false,
            active_ribbon_tab: RibbonTab::Home,
            focus_mode: false,
            show_nav_panel: true,
            show_inspector_panel: true,
            nav_tab: NavTab::Outline,
            inspector_tab: InspectorTab::Text,
            startup_nav_tab: StartupNavTab::Recent,
        }
    }
}

impl MyWordApp {
    pub fn extract_outline_headings(&self) -> Vec<OutlineHeading> {
        let mut headings = Vec::new();
        let mut offset = 0usize;
        for raw_line in self.editor.content.split('\n') {
            let trimmed = raw_line.trim().trim_matches(PAGE_BREAK_CHAR).trim();
            if !trimmed.is_empty() {
                let first_char_offset = offset + raw_line.find(trimmed).unwrap_or(0);
                let span_idx = self
                    .editor
                    .spans
                    .partition_point(|s| s.end <= first_char_offset);
                let style = self
                    .editor
                    .spans
                    .get(span_idx)
                    .map(|s| s.style)
                    .unwrap_or_default();
                let (level, title) = if let Some(rest) = trimmed.strip_prefix("### ") {
                    (3, rest.trim().to_string())
                } else if let Some(rest) = trimmed.strip_prefix("## ") {
                    (2, rest.trim().to_string())
                } else if let Some(rest) = trimmed.strip_prefix("# ") {
                    (1, rest.trim().to_string())
                } else if style.bold && style.font_size >= 20.0 {
                    (1, trimmed.to_string())
                } else if style.bold && style.font_size >= 17.0 {
                    (2, trimmed.to_string())
                } else if style.bold && style.font_size >= 15.0 {
                    (3, trimmed.to_string())
                } else {
                    (0, String::new())
                };
                if level > 0 && !title.is_empty() {
                    headings.push(OutlineHeading {
                        byte_offset: first_char_offset,
                        title,
                        level,
                    });
                }
            }
            offset += raw_line.len() + 1;
        }
        headings
    }

    pub fn jump_to_byte(&mut self, byte_offset: usize) {
        let mut safe = byte_offset.min(self.editor.content.len());
        while safe > 0 && !self.editor.content.is_char_boundary(safe) {
            safe -= 1;
        }
        self.editor.caret_byte_offset = safe;
        self.editor.selection_start = None;
        self.editor.sync_active_style_to_caret();
        self.scroll_to_caret = true;
        self.last_typing_time = Instant::now();
    }
    pub fn set_status(&mut self, msg: &str) {
        self.status_message = Some((msg.to_string(), Instant::now()));
    }

    pub fn set_zoom(&mut self, new_zoom: f32) {
        let clamped = (new_zoom * 10.0).round() / 10.0;
        let clamped = clamped.clamp(MIN_ZOOM, MAX_ZOOM);
        if (self.zoom - clamped).abs() > f32::EPSILON {
            self.zoom = clamped;
            self.cached_layout = None;
            self.set_status(&format!("Thu phóng: {:.0}%", self.zoom * 100.0));
        }
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

    pub fn reset_document_view_state(&mut self) {
        self.cached_layout = None;
        self.cached_matches.clear();
        self.active_match_idx = 0;
        self.scroll_to_caret = true;
    }

    pub fn cleanup_pending_file_ops(&mut self) {
        self.post_save_action = None;
        self.pending_drop_path = None;
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
                self.reset_document_view_state();
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
        self.editor = EditorState::new_empty();
        self.current_file_path = None;
        self.reset_document_view_state();
        clear_autosave();
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
            if self.write_to_path(&path) {
                self.execute_post_save_action(&ctx);
            } else {
                self.post_save_action = None;
            }
            return;
        }
        if self.file_dialog_rx.is_none() {
            self.file_dialog_rx = Some(spawn_save_dialog(ctx));
        }
    }

    pub fn write_to_path(&mut self, path: &Path) -> bool {
        match write_document(path, &self.editor) {
            Ok(()) => {
                self.current_file_path = Some(path.to_path_buf());
                self.editor.is_dirty = false;
                clear_autosave();
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("tệp");
                self.set_status(&format!("Đã lưu: {}", name));
                true
            }
            Err(e) => {
                self.set_status(&format!("Lỗi lưu tệp: {}", e));
                false
            }
        }
    }

    pub fn execute_post_save_action(&mut self, ctx: &egui::Context) {
        if let Some(action) = self.post_save_action.take() {
            match action {
                ConfirmAction::Quit => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                ConfirmAction::OpenNew => {
                    if let Some(drop_path) = self.pending_drop_path.take() {
                        self.load_from_path(&drop_path);
                    } else {
                        self.trigger_open_dialog(ctx.clone());
                    }
                }
                ConfirmAction::NewDocument => {
                    self.new_document();
                }
            }
        }
    }

    pub fn load_from_path(&mut self, path: &Path) {
        match load_document(path) {
            Ok((text, spans, aligns)) => {
                self.editor = EditorState::default();
                self.editor.content = text;
                self.editor.spans = spans;
                self.editor.para_alignments = aligns;
                self.current_file_path = Some(path.to_path_buf());
                self.editor.mark_content_changed();
                self.editor.is_dirty = false;
                self.editor.sync_active_style_to_caret();
                self.reset_document_view_state();
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
        match write_autosave(&self.editor) {
            Ok(()) => {
                self.last_autosave = Instant::now();
            }
            Err(e) => {
                self.set_status(&format!("Lưu tự động thất bại: {}", e));
            }
        }
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
                                if self.write_to_path(&p) {
                                    self.execute_post_save_action(ctx);
                                } else {
                                    self.cleanup_pending_file_ops();
                                }
                            }
                        }
                    } else {
                        self.cleanup_pending_file_ops();
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.file_dialog_rx = None;
                    self.cleanup_pending_file_ops();
                }
            }
        }
    }

    /// Fallback tính ranh giới đầu/cuối dòng trực tiếp từ nội dung văn bản khi CachedLayout chưa sẵn sàng.
    /// Đảm bảo offset trả về luôn nằm trên ranh giới ký tự UTF-8 (char boundary) hợp lệ.
    pub fn line_boundary_fallback(content: &str, caret: usize, to_end: bool) -> usize {
        let len = content.len();
        if len == 0 {
            return 0;
        }
        let safe_caret = caret.min(len);
        let safe_caret = if content.is_char_boundary(safe_caret) {
            safe_caret
        } else {
            let mut c = safe_caret;
            while c > 0 && !content.is_char_boundary(c) {
                c -= 1;
            }
            c
        };

        if !to_end {
            // Nhấn Home: Tìm ký tự xuống dòng '\n' hoặc PAGE_BREAK_CHAR gần nhất phía trước
            match content[..safe_caret].rfind(['\n', PAGE_BREAK_CHAR]) {
                Some(idx) => idx + 1,
                None => 0,
            }
        } else {
            // Nhấn End: Tìm ký tự xuống dòng '\n' hoặc PAGE_BREAK_CHAR gần nhất phía sau
            match content[safe_caret..].find(['\n', PAGE_BREAK_CHAR]) {
                Some(rel_idx) => {
                    let mut end = safe_caret + rel_idx;
                    // Bỏ qua '\r' nếu tệp có định dạng xuống dòng Windows CRLF
                    if end > 0 && content.as_bytes().get(end - 1) == Some(&b'\r') {
                        end -= 1;
                    }
                    end
                }
                None => len,
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
        } else {
            Self::line_boundary_fallback(
                &self.editor.content,
                self.editor.caret_byte_offset,
                to_end,
            )
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

    fn handle_dropped_files(&mut self, ctx: &egui::Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(file) = dropped.first()
            && let Some(path) = &file.path
        {
            if self.editor.is_dirty {
                self.pending_drop_path = Some(path.clone());
                self.pending_confirm = Some(ConfirmAction::OpenNew);
            } else {
                self.load_from_path(path);
            }
        }
    }

    fn process_events(&mut self, ctx: &egui::Context, ui_wants_kb: bool) {
        // Nếu đang mở hộp thoại xác nhận, chỉ nhận phím Escape để đóng hộp thoại
        if self.pending_confirm.is_some() {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.pending_confirm = None;
                self.post_save_action = None;
                self.pending_drop_path = None;
            }
            return;
        }

        // Hỗ trợ Ctrl + Cuộn chuột để Zoom
        let scroll_delta = ctx.input(|i| {
            if i.modifiers.command {
                i.smooth_scroll_delta.y
            } else {
                0.0
            }
        });
        if scroll_delta.abs() > 0.0 {
            let step = if scroll_delta > 0.0 {
                ZOOM_STEP
            } else {
                -ZOOM_STEP
            };
            self.set_zoom(self.zoom + step);
        }

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
                    if self.show_startup_modal {
                        self.show_startup_modal = false;
                        self.new_document();
                    } else {
                        self.trigger_new_document();
                    }
                    continue;
                }
                egui::Event::Key {
                    key: egui::Key::K,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command => {
                    self.find_bar_open = true;
                    self.focus_find_input = true;
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
                    key: egui::Key::Plus | egui::Key::Equals,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command => {
                    self.set_zoom(self.zoom + ZOOM_STEP);
                    continue;
                }
                egui::Event::Key {
                    key: egui::Key::Minus,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command => {
                    self.set_zoom(self.zoom - ZOOM_STEP);
                    continue;
                }
                egui::Event::Key {
                    key: egui::Key::Num0,
                    pressed: true,
                    modifiers,
                    ..
                } if modifiers.command => {
                    self.set_zoom(1.0);
                    continue;
                }
                egui::Event::Key {
                    key: egui::Key::Escape,
                    pressed: true,
                    ..
                } if self.show_startup_modal => {
                    self.show_startup_modal = false;
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

            if ui_wants_kb || self.show_startup_modal || self.pending_confirm.is_some() {
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
        let mut did_navigate_or_edit = true;

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
                    did_navigate_or_edit = false;
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
                _ => {
                    did_navigate_or_edit = false;
                }
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
                egui::Key::Tab => self.editor.insert_text("    "),
                _ => {
                    did_navigate_or_edit = false;
                }
            }
        }

        if did_navigate_or_edit {
            self.last_typing_time = Instant::now();
            self.scroll_to_caret = true;
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
        if dt > 0.0 && dt < 1.0 {
            self.fps = self.fps * 0.9 + (1.0 / dt) * 0.1;
        }

        if let Some((_, time)) = &self.status_message
            && time.elapsed() >= Duration::from_secs(STATUS_MESSAGE_TIMEOUT_SECS)
        {
            self.status_message = None;
        }

        self.maybe_autosave();
        self.poll_file_dialog(ctx);
        self.handle_dropped_files(ctx);

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

        let viewport_w = ctx.screen_rect().width();
        if !self.focus_mode {
            if self.show_nav_panel && viewport_w >= 860.0 {
                render_navigation_sidebar(ctx, self, &theme, total_pages, caret_page_idx);
            }
            if self.show_inspector_panel && viewport_w >= 1020.0 {
                render_inspector_sidebar(ctx, self, &theme);
            }
        }

        let elapsed_ms = self.last_typing_time.elapsed().as_millis() as u64;
        let show_caret = (elapsed_ms / CARET_BLINK_MS).is_multiple_of(2);
        let ime_output_rect = render_page_canvas(ctx, self, &theme, total_pages, show_caret);

        if !ui_wants_kb
            && !self.show_startup_modal
            && self.pending_confirm.is_none()
            && let Some((area_rect, cursor_rect)) = ime_output_rect
        {
            ctx.output_mut(|o| {
                o.ime = Some(egui::output::IMEOutput {
                    rect: area_rect,
                    cursor_rect,
                });
            });
        }

        render_startup_modal(ctx, self, &theme);
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
        let mut app = MyWordApp {
            show_startup_modal: false,
            ..Default::default()
        };
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

    #[test]
    fn test_startup_modal_blocks_typing_until_closed() {
        let mut app = MyWordApp::default();
        assert!(app.show_startup_modal);

        let ctx = egui::Context::default();
        ctx.input_mut(|i| {
            i.events.push(egui::Event::Text("hello".to_string()));
        });
        app.process_events(&ctx, false);
        // Khi modal đang mở, text không được đưa vào editor
        assert!(!app.editor.content.contains("hello"));

        // Đóng modal bằng Escape
        ctx.input_mut(|i| {
            i.events.push(egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            });
        });
        app.process_events(&ctx, false);
        assert!(!app.show_startup_modal);
    }

    #[test]
    fn test_line_boundary_fallback_home_end() {
        let content = "Dòng 1: Tiếng Việt\nDòng 2: Soạn thảo\nDòng 3: Kết thúc";
        // Vị trí bắt đầu Dòng 2
        let line2_start = content.find("Dòng 2").unwrap();
        let line2_mid = content.find("Soạn").unwrap();
        let line2_end = content.find("\nDòng 3").unwrap();

        // 1. Khi con trỏ ở giữa Dòng 2, Home phải nhảy về đầu Dòng 2 (không nhảy về 0)
        let home_idx = MyWordApp::line_boundary_fallback(content, line2_mid, false);
        assert_eq!(home_idx, line2_start);

        // 2. Khi con trỏ ở giữa Dòng 2, End phải nhảy về cuối Dòng 2 (không nhảy về content.len())
        let end_idx = MyWordApp::line_boundary_fallback(content, line2_mid, true);
        assert_eq!(end_idx, line2_end);

        // 3. Kiểm tra với app thực tế không có cache layout
        let mut app = MyWordApp::default();
        app.editor.content = content.to_string();
        app.cached_layout = None;
        app.editor.caret_byte_offset = line2_mid;

        app.move_to_line_boundary(false, false);
        assert_eq!(app.editor.caret_byte_offset, line2_start);
        assert!(app.editor.selection_start.is_none());

        app.move_to_line_boundary(true, true);
        assert_eq!(app.editor.caret_byte_offset, line2_end);
        assert_eq!(app.editor.selection_start, Some(line2_start));

        // 4. Kiểm tra CRLF
        let crlf_content = "Dòng A\r\nDòng B";
        let crlf_end = MyWordApp::line_boundary_fallback(crlf_content, 2, true);
        assert_eq!(&crlf_content[..crlf_end], "Dòng A");
    }

    #[test]
    fn test_cleanup_pending_file_ops_prevents_leaked_drop_path() {
        let mut app = MyWordApp {
            pending_drop_path: Some(PathBuf::from("C:\\test\\dropped.mwd")),
            post_save_action: Some(ConfirmAction::OpenNew),
            ..Default::default()
        };

        app.cleanup_pending_file_ops();

        assert!(app.pending_drop_path.is_none());
        assert!(app.post_save_action.is_none());
    }

    #[test]
    fn test_reset_document_view_state_clears_all_caches() {
        let mut app = MyWordApp {
            cached_matches: vec![(0, 5), (10, 15)],
            active_match_idx: 1,
            scroll_to_caret: false,
            ..Default::default()
        };

        app.reset_document_view_state();

        assert!(app.cached_layout.is_none());
        assert!(app.cached_matches.is_empty());
        assert_eq!(app.active_match_idx, 0);
        assert!(app.scroll_to_caret);
    }

    #[test]
    fn test_extract_outline_headings_and_jump_to_byte() {
        use crate::document::{StyleSpan, TextStyle};

        let mut app = MyWordApp::default();
        let h1 = TextStyle {
            bold: true,
            font_size: 22.0,
            ..Default::default()
        };
        let body = TextStyle {
            font_size: 14.0,
            ..Default::default()
        };

        let p1 = "01 Tầm nhìn & định hướng\n";
        let p2 = "Nội dung chi tiết tiếng Việt.\n";
        let p3 = "## Mục tiêu chiến lược\n";
        let p4 = "Đoạn cuối.";

        let s1_end = p1.len();
        let s2_end = s1_end + p2.len();
        let s3_end = s2_end + p3.len();
        let s4_end = s3_end + p4.len();

        app.editor.content = format!("{}{}{}{}", p1, p2, p3, p4);
        app.editor.spans = vec![
            StyleSpan {
                start: 0,
                end: s1_end,
                style: h1,
            },
            StyleSpan {
                start: s1_end,
                end: s4_end,
                style: body,
            },
        ];

        let headings = app.extract_outline_headings();
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[0].level, 1);
        assert_eq!(headings[0].title, "01 Tầm nhìn & định hướng");
        assert_eq!(headings[0].byte_offset, 0);
        assert_eq!(headings[1].level, 2);
        assert_eq!(headings[1].title, "Mục tiêu chiến lược");
        assert_eq!(headings[1].byte_offset, s2_end);

        // Kiểm tra jump_to_byte bảo vệ ranh giới ký tự UTF-8
        app.jump_to_byte(headings[1].byte_offset);
        assert_eq!(app.editor.caret_byte_offset, s2_end);
        assert!(app.scroll_to_caret);
    }

    #[test]
    fn test_ctrl_k_opens_find_bar_and_ctrl_n_from_startup() {
        let mut app = MyWordApp::default();
        assert!(app.show_startup_modal);

        let ctx = egui::Context::default();
        ctx.input_mut(|i| {
            i.events.push(egui::Event::Key {
                key: egui::Key::N,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            });
        });
        app.process_events(&ctx, false);
        assert!(!app.show_startup_modal);

        ctx.input_mut(|i| {
            i.events.push(egui::Event::Key {
                key: egui::Key::K,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::COMMAND,
            });
        });
        app.process_events(&ctx, false);
        assert!(app.find_bar_open);
        assert!(app.focus_find_input);
    }

    #[test]
    fn test_figma_ui_full_frame_render_no_panic() {
        let mut app = MyWordApp::default();
        let ctx = egui::Context::default();
        crate::platform::setup_fonts(&ctx);
        let theme = ThemePalette::from_mode(false);
        theme.apply(&ctx);

        let raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                Pos2::ZERO,
                egui::Vec2::new(1280.0, 860.0),
            )),
            ..Default::default()
        };

        // Frame 1: Render với Startup Modal, Header, Ribbon, Sidebars và Canvas
        let _ = ctx.run(raw_input.clone(), |ctx| {
            let _ = ensure_layout_cache(&mut app.cached_layout, &mut app.editor, app.zoom, ctx);
            render_header_bar(ctx, &mut app, &theme);
            render_ribbon_bar(ctx, &mut app, &theme);
            render_status_bar(ctx, &mut app, &theme, 0, 1, 0, 0);
            render_navigation_sidebar(ctx, &mut app, &theme, 1, 0);
            render_inspector_sidebar(ctx, &mut app, &theme);
            render_page_canvas(ctx, &mut app, &theme, 1, true);
            render_startup_modal(ctx, &mut app, &theme);
        });

        // Frame 2: Đổi sang Dark Mode và chuyển các tab phụ
        app.show_startup_modal = false;
        app.active_ribbon_tab = RibbonTab::Insert;
        app.nav_tab = NavTab::Pages;
        app.inspector_tab = InspectorTab::Page;
        let dark_theme = ThemePalette::from_mode(true);
        let _ = ctx.run(raw_input, |ctx| {
            let _ = ensure_layout_cache(&mut app.cached_layout, &mut app.editor, app.zoom, ctx);
            render_header_bar(ctx, &mut app, &dark_theme);
            render_ribbon_bar(ctx, &mut app, &dark_theme);
            render_status_bar(ctx, &mut app, &dark_theme, 0, 1, 10, 50);
            render_navigation_sidebar(ctx, &mut app, &dark_theme, 1, 0);
            render_inspector_sidebar(ctx, &mut app, &dark_theme);
            render_page_canvas(ctx, &mut app, &dark_theme, 1, true);
        });
    }
}
