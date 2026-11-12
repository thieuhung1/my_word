use crate::document::{PAGE_BREAK_CHAR, find_matches};
use crate::editor::EditorState;
use crate::layout::{CachedLayout, PointerQuery, caret_page_index, ensure_layout_cache};
use crate::platform::{
    FileDialogResult, load_document, spawn_open_dialog, spawn_save_dialog, write_autosave,
    write_document,
};
use crate::ui::{
    ThemePalette, render_find_replace_bar, render_header_bar, render_page_canvas,
    render_ribbon_bar, render_status_bar,
};
use eframe::egui::{self, Pos2};
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

pub struct MyWordApp {
    pub editor: EditorState,
    pub start_time: Instant,
    pub last_frame_time: Instant,
    pub fps: f32,
    pub zoom: f32,
    pub show_margins: bool,
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
}

impl Default for MyWordApp {
    fn default() -> Self {
        Self {
            editor: EditorState::default(),
            start_time: Instant::now(),
            last_frame_time: Instant::now(),
            fps: 60.0,
            zoom: 1.0,
            show_margins: true,
            dark_workspace: false,
            status_message: Some((
                "Sẵn sàng soạn thảo tiếng Việt Telex".to_string(),
                Instant::now(),
            )),
            pending_pointer_query: None,
            last_caret_pos: None,
            last_line_height: 22.0,
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
        if !self.cached_matches.is_empty() && self.active_match_idx >= self.cached_matches.len() {
            self.active_match_idx = 0;
        }
    }

    pub fn trigger_open_dialog(&mut self, ctx: egui::Context) {
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

    fn write_to_path(&mut self, path: &Path) {
        match write_document(path, &self.editor) {
            Ok(()) => {
                self.current_file_path = Some(path.to_path_buf());
                self.editor.is_dirty = false;
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
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("tệp");
                self.set_status(&format!("Đã mở: {}", name));
            }
            Err(e) => self.set_status(&format!("Không đọc được tệp: {}", e)),
        }
    }

    fn maybe_autosave(&mut self) {
        if !self.editor.is_dirty || self.last_autosave.elapsed().as_secs() < 20 {
            return;
        }
        self.last_autosave = Instant::now();
        write_autosave(&self.editor);
    }

    fn poll_file_dialog(&mut self) {
        if let Some(rx) = &self.file_dialog_rx
            && let Ok(maybe_res) = rx.try_recv()
        {
            self.file_dialog_rx = None;
            if let Some(res) = maybe_res {
                match res {
                    FileDialogResult::Open(p) => self.load_from_path(&p),
                    FileDialogResult::Save(p) => self.write_to_path(&p),
                }
            }
        }
    }

    fn process_events(&mut self, ctx: &egui::Context, ui_wants_kb: bool) {
        let events = ctx.input(|i| i.events.clone());
        let mut saw_backspace_in_frame = false;

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
                    self.editor.handle_text_input(&t, !saw_backspace_in_frame);
                }
                egui::Event::Copy => self.copy_selection_to_clipboard(ctx),
                egui::Event::Cut => self.cut_selection_to_clipboard(ctx),
                egui::Event::Paste(pasted) => {
                    self.editor.clipboard_buffer = pasted.clone();
                    self.editor.insert_text(&pasted);
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if key == egui::Key::Backspace && !modifiers.command {
                        saw_backspace_in_frame = true;
                    }
                    self.handle_key_event(key, modifiers);
                }
                egui::Event::Ime(ime_event) => match ime_event {
                    egui::ImeEvent::Preedit(text) => {
                        self.editor.ime_preedit = if text.is_empty() { None } else { Some(text) };
                    }
                    egui::ImeEvent::Commit(text) => {
                        self.editor.ime_preedit = None;
                        self.editor.insert_text(&text);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    fn handle_key_event(&mut self, key: egui::Key, modifiers: egui::Modifiers) {
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
                _ => {}
            }
        } else {
            match key {
                egui::Key::Backspace => self.editor.backspace(),
                egui::Key::Delete => self.editor.delete_forward(),
                egui::Key::ArrowLeft => self.editor.move_left_with_selection(modifiers.shift),
                egui::Key::ArrowRight => self.editor.move_right_with_selection(modifiers.shift),
                egui::Key::ArrowUp => {
                    if let Some(pos) = self.last_caret_pos {
                        self.pending_keep_selection = modifiers.shift;
                        self.pending_pointer_query = Some(PointerQuery::MoveVertical(Pos2::new(
                            pos.x,
                            pos.y - self.last_line_height * 0.75,
                        )));
                    }
                }
                egui::Key::ArrowDown => {
                    if let Some(pos) = self.last_caret_pos {
                        self.pending_keep_selection = modifiers.shift;
                        self.pending_pointer_query = Some(PointerQuery::MoveVertical(Pos2::new(
                            pos.x,
                            pos.y + self.last_line_height * 1.25,
                        )));
                    }
                }
                egui::Key::Home => self.editor.move_to_doc_boundary(false, modifiers.shift),
                egui::Key::End => self.editor.move_to_doc_boundary(true, modifiers.shift),
                egui::Key::Enter => self.editor.insert_text("\n"),
                _ => {}
            }
        }
    }
}

impl eframe::App for MyWordApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        if dt > 0.0 {
            self.fps = self.fps * 0.9 + (1.0 / dt) * 0.1;
        }
        self.maybe_autosave();
        self.poll_file_dialog();

        let ui_wants_kb = ctx.wants_keyboard_input();
        self.process_events(ctx, ui_wants_kb);

        let theme = ThemePalette::from_mode(self.dark_workspace);
        render_header_bar(ctx, self, &theme);
        render_ribbon_bar(ctx, self, &theme);
        render_find_replace_bar(ctx, self, &theme);

        let (total_pages, caret_page_idx, word_count, char_count) = {
            let cache =
                ensure_layout_cache(&mut self.cached_layout, &mut self.editor, self.zoom, ctx);
            (
                cache.pages.len().max(1),
                caret_page_index(cache, self.editor.caret_byte_offset),
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

        let elapsed_ms = self.start_time.elapsed().as_millis() as u64;
        let show_caret = (elapsed_ms / 550).is_multiple_of(2);
        let ime_output_rect = render_page_canvas(ctx, self, &theme, total_pages, show_caret);

        if !ui_wants_kb && let Some((area_rect, cursor_rect)) = ime_output_rect {
            ctx.output_mut(|o| {
                o.ime = Some(egui::output::IMEOutput {
                    rect: area_rect,
                    cursor_rect,
                });
            });
        }

        let next_blink_ms = (550 - (elapsed_ms % 550)).max(16);
        ctx.request_repaint_after(Duration::from_millis(next_blink_ms));
    }
}
