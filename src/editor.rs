use crate::document::{
    DocumentSnapshot, PAGE_BREAK_CHAR, StyleSpan, TextAlignment, TextStyle, apply_style_on_range,
    find_matches, shift_spans_on_delete, shift_spans_on_insert,
};
use crate::telex;
use std::collections::VecDeque;
use std::time::Instant;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

const MAX_UNDO_SNAPSHOTS: usize = 50;
const UNDO_COALESCE_MS: u128 = 800;

/// Ép offset byte về ranh giới ký tự UTF-8 hợp lệ (<= `s.len()`) để tránh panic khi cắt chuỗi
pub fn clamp_to_char_boundary(s: &str, offset: usize) -> usize {
    let mut clamped = offset.min(s.len());
    while clamped > 0 && !s.is_char_boundary(clamped) {
        clamped -= 1;
    }
    clamped
}

/// Chế độ gõ tiếng Việt
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputMethodMode {
    AutoTelex,
    SystemOnly,
}

/// Kiểu phong cách văn bản mẫu chuẩn Word / WPS Office
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DocumentPresetStyle {
    Normal,
    Heading1,
    Heading2,
    Heading3,
    Quote,
}

impl DocumentPresetStyle {
    pub fn label(self) -> &'static str {
        match self {
            DocumentPresetStyle::Normal => "Văn bản chính",
            DocumentPresetStyle::Heading1 => "Tiêu đề 1",
            DocumentPresetStyle::Heading2 => "Tiêu đề 2",
            DocumentPresetStyle::Heading3 => "Tiêu đề 3",
            DocumentPresetStyle::Quote => "Trích dẫn",
        }
    }

    pub fn to_text_style(self) -> TextStyle {
        match self {
            DocumentPresetStyle::Normal => TextStyle::default(),
            DocumentPresetStyle::Heading1 => TextStyle {
                font_size: 22.0,
                bold: true,
                color: [30, 41, 59],
                ..Default::default()
            },
            DocumentPresetStyle::Heading2 => TextStyle {
                font_size: 18.0,
                bold: true,
                color: [37, 99, 235],
                ..Default::default()
            },
            DocumentPresetStyle::Heading3 => TextStyle {
                font_size: 16.0,
                bold: true,
                italic: true,
                color: [51, 65, 85],
                ..Default::default()
            },
            DocumentPresetStyle::Quote => TextStyle {
                font_size: 14.0,
                italic: true,
                color: [100, 116, 139],
                ..Default::default()
            },
        }
    }
}

/// Trạng thái cốt lõi của trình soạn thảo (nội dung, định dạng, con trỏ, Undo/Redo)
pub struct EditorState {
    pub content: String,
    pub spans: Vec<StyleSpan>,
    pub para_alignments: Vec<TextAlignment>,
    pub active_style: TextStyle,
    pub caret_byte_offset: usize,
    pub selection_start: Option<usize>,
    pub undo_stack: VecDeque<DocumentSnapshot>,
    pub redo_stack: Vec<DocumentSnapshot>,
    pub clipboard_buffer: String,
    pub ime_preedit: Option<String>,
    pub format_painter_style: Option<TextStyle>,
    pub unikey_backspace_count: usize,
    pub input_mode: InputMethodMode,
    pub is_dirty: bool,
    pub layout_dirty: bool,
    pub find_dirty: bool,
    last_typing_undo: Instant,
    can_coalesce_undo: bool,
}

impl Default for EditorState {
    fn default() -> Self {
        let initial_text: String = "Cộng hòa Xã hội Chủ nghĩa Việt Nam\nĐộc lập - Tự do - Hạnh phúc\n\nĐây là Rust Word Processor — hỗ trợ Rich Text theo vùng chọn, phân trang A4 tự động, Tìm & Thay thế (Ctrl+F/H), Mở/Lưu tệp (.mwd/.txt) và gõ tiếng Việt Telex.\nBạn có thể bôi đen chữ rồi bấm B, I, U, S, đổi màu chữ hoặc tô sáng Highlight trực tiếp tại đây!"
            .nfc()
            .collect();
        let len = initial_text.len();

        let mut spans = vec![StyleSpan {
            start: 0,
            end: len,
            style: TextStyle::default(),
        }];
        if let Some(first_nl) = initial_text.find('\n') {
            apply_style_on_range(&mut spans, len, 0, first_nl, |s| {
                s.bold = true;
                s.font_size = 16.0;
            });
        }

        let para_count = initial_text.split(['\n', PAGE_BREAK_CHAR]).count().max(1);
        let mut aligns = vec![TextAlignment::Left; para_count];
        if aligns.len() >= 2 {
            aligns[0] = TextAlignment::Center;
            aligns[1] = TextAlignment::Center;
        }

        Self {
            content: initial_text,
            spans,
            para_alignments: aligns,
            active_style: TextStyle::default(),
            caret_byte_offset: len,
            selection_start: None,
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            clipboard_buffer: String::new(),
            ime_preedit: None,
            format_painter_style: None,
            unikey_backspace_count: 0,
            input_mode: InputMethodMode::AutoTelex,
            is_dirty: false,
            layout_dirty: true,
            find_dirty: true,
            last_typing_undo: Instant::now(),
            can_coalesce_undo: false,
        }
    }
}

impl EditorState {
    /// Tạo tài liệu từ văn bản thuần túy
    pub fn from_plain_text(text: &str) -> Self {
        let normalized: String = text.nfc().collect();
        let caret = normalized.len();
        let spans = if caret > 0 {
            vec![StyleSpan {
                start: 0,
                end: caret,
                style: TextStyle::default(),
            }]
        } else {
            Vec::new()
        };
        let para_count = normalized.split(['\n', PAGE_BREAK_CHAR]).count().max(1);
        Self {
            content: normalized,
            spans,
            para_alignments: vec![TextAlignment::Left; para_count],
            active_style: TextStyle::default(),
            caret_byte_offset: caret,
            selection_start: None,
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            clipboard_buffer: String::new(),
            ime_preedit: None,
            format_painter_style: None,
            unikey_backspace_count: 0,
            input_mode: InputMethodMode::AutoTelex,
            is_dirty: false,
            layout_dirty: true,
            find_dirty: true,
            last_typing_undo: Instant::now(),
            can_coalesce_undo: false,
        }
    }

    #[inline]
    pub fn mark_content_changed(&mut self) {
        self.layout_dirty = true;
        self.find_dirty = true;
        self.is_dirty = true;
    }

    pub fn sync_para_alignments(&mut self) {
        let count = self.content.split(['\n', PAGE_BREAK_CHAR]).count().max(1);
        if self.para_alignments.len() < count {
            self.para_alignments.resize(count, TextAlignment::Left);
        } else {
            self.para_alignments.truncate(count);
        }
    }

    pub fn paragraph_index_at(&self, byte_offset: usize) -> usize {
        let clamped = byte_offset.min(self.content.len());
        self.content[..clamped]
            .chars()
            .filter(|&c| c == '\n' || c == PAGE_BREAK_CHAR)
            .count()
    }

    pub fn sync_active_style_to_caret(&mut self) {
        if self.content.is_empty() {
            return;
        }
        self.active_style = crate::document::style_at_offset(&self.spans, self.caret_byte_offset);
    }

    /// Lưu ảnh chụp Undo (tối đa 50 bản để tiết kiệm RAM)
    pub fn save_undo(&mut self) {
        self.can_coalesce_undo = false;
        self.push_undo_snapshot();
    }

    /// Gộp các phím gõ liên tục trong 800ms vào cùng 1 bước Undo
    fn save_undo_typing(&mut self, break_group: bool) {
        let now = Instant::now();
        if !break_group
            && self.can_coalesce_undo
            && now.duration_since(self.last_typing_undo).as_millis() < UNDO_COALESCE_MS
        {
            self.last_typing_undo = now;
            return;
        }
        self.push_undo_snapshot();
        self.last_typing_undo = now;
        self.can_coalesce_undo = !break_group;
    }

    fn push_undo_snapshot(&mut self) {
        if let Some(last) = self.undo_stack.back()
            && last.content == self.content
            && last.spans == self.spans
            && last.para_alignments == self.para_alignments
        {
            return;
        }
        if self.undo_stack.len() >= MAX_UNDO_SNAPSHOTS {
            self.undo_stack.pop_front();
        }
        self.undo_stack.push_back(DocumentSnapshot {
            content: self.content.clone(),
            caret_byte_offset: self.caret_byte_offset,
            spans: self.spans.clone(),
            para_alignments: self.para_alignments.clone(),
        });
        if !self.redo_stack.is_empty() {
            self.redo_stack.clear();
            self.redo_stack.shrink_to_fit();
        }
    }

    fn take_current_snapshot(&mut self) -> DocumentSnapshot {
        DocumentSnapshot {
            content: std::mem::take(&mut self.content),
            caret_byte_offset: self.caret_byte_offset,
            spans: std::mem::take(&mut self.spans),
            para_alignments: std::mem::take(&mut self.para_alignments),
        }
    }

    fn restore_snapshot(&mut self, snap: DocumentSnapshot) {
        self.content = snap.content;
        self.caret_byte_offset = clamp_to_char_boundary(&self.content, snap.caret_byte_offset);
        self.spans = snap.spans;
        self.para_alignments = snap.para_alignments;
        self.selection_start = None;
        self.sync_para_alignments();
        self.sync_active_style_to_caret();
        self.mark_content_changed();
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop_back() {
            self.can_coalesce_undo = false;
            let cur = self.take_current_snapshot();
            self.redo_stack.push(cur);
            self.restore_snapshot(prev);
            return true;
        }
        false
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.can_coalesce_undo = false;
            let cur = self.take_current_snapshot();
            self.undo_stack.push_back(cur);
            self.restore_snapshot(next);
            return true;
        }
        false
    }

    pub fn selected_range(&self) -> Option<(usize, usize)> {
        let start = self.selection_start?;
        let a = clamp_to_char_boundary(&self.content, start);
        let b = clamp_to_char_boundary(&self.content, self.caret_byte_offset);
        (a != b).then(|| (a.min(b), a.max(b)))
    }

    fn remove_range_and_sync(&mut self, from: usize, to: usize) {
        let removed_breaks = self.content[from..to]
            .chars()
            .filter(|&c| c == '\n' || c == PAGE_BREAK_CHAR)
            .count();
        let first_para = self.paragraph_index_at(from);

        self.content.drain(from..to);
        shift_spans_on_delete(&mut self.spans, from, to);

        if removed_breaks > 0 {
            let end_rm = (first_para + 1 + removed_breaks).min(self.para_alignments.len());
            if first_para + 1 < end_rm {
                self.para_alignments.drain(first_para + 1..end_rm);
            }
        }
        self.sync_para_alignments();
        self.mark_content_changed();
    }

    pub fn delete_selection(&mut self) -> bool {
        if let Some((from, to)) = self.selected_range() {
            self.remove_range_and_sync(from, to);
            self.caret_byte_offset = from;
            self.selection_start = None;
            return true;
        }
        false
    }

    pub fn copy_selection(&self) -> Option<String> {
        let (from, to) = self.selected_range()?;
        (from < to && to <= self.content.len()).then(|| self.content[from..to].to_string())
    }

    pub fn select_all(&mut self) {
        self.selection_start = Some(0);
        self.caret_byte_offset = self.content.len();
    }

    pub fn apply_format<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut TextStyle),
    {
        if let Some((from, to)) = self.selected_range() {
            self.save_undo();
            apply_style_on_range(&mut self.spans, self.content.len(), from, to, &mut f);
            f(&mut self.active_style);
            self.layout_dirty = true;
            self.is_dirty = true;
        } else {
            f(&mut self.active_style);
        }
    }

    pub fn toggle_format_painter(&mut self) {
        if self.format_painter_style.is_some() {
            self.format_painter_style = None;
        } else {
            let style = if let Some((from, _)) = self.selected_range() {
                self.spans
                    .iter()
                    .find(|s| s.start <= from && from < s.end)
                    .map(|s| s.style)
                    .unwrap_or(self.active_style)
            } else {
                self.active_style
            };
            self.format_painter_style = Some(style);
        }
    }

    pub fn apply_format_painter_if_active(&mut self) -> bool {
        if let Some((from, to)) = self.selected_range()
            && let Some(style) = self.format_painter_style.take()
        {
            self.save_undo();
            apply_style_on_range(&mut self.spans, self.content.len(), from, to, |s| {
                *s = style;
            });
            self.active_style = style;
            self.mark_content_changed();
            return true;
        }
        false
    }

    pub fn clear_formatting(&mut self) {
        let def_style = TextStyle::default();
        if let Some((from, to)) = self.selected_range() {
            self.save_undo();
            apply_style_on_range(&mut self.spans, self.content.len(), from, to, |s| {
                *s = def_style;
            });
            self.active_style = def_style;
            self.mark_content_changed();
        } else {
            self.active_style = def_style;
        }
    }

    pub fn insert_bullet_item(&mut self) {
        self.insert_text("• ");
    }

    pub fn insert_numbered_item(&mut self) {
        let p_idx = self.paragraph_index_at(self.caret_byte_offset);
        let prefix = format!("{}. ", p_idx + 1);
        self.insert_text(&prefix);
    }

    pub fn apply_preset_style(&mut self, preset: DocumentPresetStyle) {
        let style = preset.to_text_style();
        if let Some((from, to)) = self.selected_range() {
            self.save_undo();
            apply_style_on_range(&mut self.spans, self.content.len(), from, to, |s| {
                *s = style;
            });
            self.active_style = style;
            self.mark_content_changed();
        } else {
            let p_start = self.content[..self.caret_byte_offset]
                .rfind('\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            let p_end = self.content[self.caret_byte_offset..]
                .find('\n')
                .map(|i| self.caret_byte_offset + i)
                .unwrap_or(self.content.len());
            if p_start < p_end {
                self.save_undo();
                apply_style_on_range(&mut self.spans, self.content.len(), p_start, p_end, |s| {
                    *s = style;
                });
                self.active_style = style;
                self.mark_content_changed();
            } else {
                self.active_style = style;
            }
        }
    }

    pub fn set_alignment(&mut self, align: TextAlignment) {
        self.save_undo();
        self.sync_para_alignments();
        let (p_start, p_end) = if let Some((from, to)) = self.selected_range() {
            (self.paragraph_index_at(from), self.paragraph_index_at(to))
        } else {
            let p = self.paragraph_index_at(self.caret_byte_offset);
            (p, p)
        };
        for idx in p_start..=p_end {
            if let Some(a) = self.para_alignments.get_mut(idx) {
                *a = align;
            }
        }
        self.layout_dirty = true;
        self.is_dirty = true;
    }

    pub fn current_alignment(&self) -> TextAlignment {
        let idx = self.paragraph_index_at(self.caret_byte_offset);
        self.para_alignments
            .get(idx)
            .copied()
            .unwrap_or(TextAlignment::Left)
    }

    pub fn insert_text(&mut self, text: &str) {
        let clean_text = text.replace("\r\n", "\n").replace('\r', "\n");
        let has_sel = self.selected_range().is_some();
        let break_group = has_sel
            || clean_text.len() > 4
            || clean_text
                .chars()
                .any(|c| c.is_whitespace() || c == PAGE_BREAK_CHAR);
        self.save_undo_typing(break_group);
        self.delete_selection();

        let normalized: String = clean_text.nfc().collect();
        let pos = self.caret_byte_offset;

        let new_breaks = normalized
            .chars()
            .filter(|&c| c == '\n' || c == PAGE_BREAK_CHAR)
            .count();
        if new_breaks > 0 {
            let cur_p = self.paragraph_index_at(pos);
            let cur_align = self
                .para_alignments
                .get(cur_p)
                .copied()
                .unwrap_or(TextAlignment::Left);
            for offset in 1..=new_breaks {
                let insert_at = (cur_p + offset).min(self.para_alignments.len());
                self.para_alignments.insert(insert_at, cur_align);
            }
        }

        self.content.insert_str(pos, &normalized);
        shift_spans_on_insert(&mut self.spans, pos, normalized.len(), self.active_style);
        self.caret_byte_offset += normalized.len();
        self.selection_start = None;
        self.sync_para_alignments();
        self.mark_content_changed();
    }

    pub fn handle_text_input(&mut self, text: &str, allow_telex: bool) {
        if allow_telex
            && self.input_mode == InputMethodMode::AutoTelex
            && self.selected_range().is_none()
        {
            let mut chars = text.chars();
            if let (Some(ch), None) = (chars.next(), chars.next())
                && ch.is_ascii_alphabetic()
            {
                let prefix = &self.content[..self.caret_byte_offset];
                let word_start = prefix
                    .char_indices()
                    .rev()
                    .take_while(|&(_, c)| c.is_alphabetic())
                    .last()
                    .map(|(idx, _)| idx)
                    .unwrap_or(self.caret_byte_offset);
                let current_word = &self.content[word_start..self.caret_byte_offset];
                if let Some(replaced) = telex::try_apply_telex(current_word, ch) {
                    self.save_undo_typing(false);
                    let normalized: String = replaced.nfc().collect();
                    let old_end = self.caret_byte_offset;
                    let word_style = self
                        .spans
                        .iter()
                        .find(|s| s.start <= word_start && word_start < s.end)
                        .map(|s| s.style)
                        .unwrap_or(self.active_style);

                    self.content.replace_range(word_start..old_end, &normalized);
                    shift_spans_on_delete(&mut self.spans, word_start, old_end);
                    shift_spans_on_insert(
                        &mut self.spans,
                        word_start,
                        normalized.len(),
                        word_style,
                    );
                    self.caret_byte_offset = word_start + normalized.len();
                    self.selection_start = None;
                    self.mark_content_changed();
                    return;
                }
            }
        }
        self.insert_text(text);
    }

    pub fn backspace(&mut self) {
        if self.selected_range().is_some() {
            self.save_undo();
            self.delete_selection();
            self.sync_active_style_to_caret();
            return;
        }
        if self.caret_byte_offset == 0 {
            return;
        }
        let end = clamp_to_char_boundary(&self.content, self.caret_byte_offset);
        let grapheme_offset = self.content[..end]
            .grapheme_indices(true)
            .next_back()
            .map(|(offset, _)| offset);
        if let Some(offset) = grapheme_offset {
            self.save_undo_typing(false);
            self.remove_range_and_sync(offset, end);
            self.caret_byte_offset = offset;
            if end > offset {
                self.unikey_backspace_count += 1;
            }
            self.sync_active_style_to_caret();
        }
    }

    pub fn delete_word_backward(&mut self) {
        if self.delete_selection() || self.caret_byte_offset == 0 {
            return;
        }
        let prefix = &self.content[..self.caret_byte_offset];
        let mut target_offset = 0;
        let mut passed_char = false;
        for (idx, ch) in prefix.char_indices().rev() {
            if ch.is_whitespace() {
                if passed_char {
                    target_offset = idx + ch.len_utf8();
                    break;
                }
            } else {
                passed_char = true;
            }
        }
        self.save_undo();
        self.selection_start = Some(target_offset);
        self.delete_selection();
        self.sync_active_style_to_caret();
    }

    pub fn delete_forward(&mut self) {
        if self.selected_range().is_some() {
            self.save_undo();
            self.delete_selection();
            self.sync_active_style_to_caret();
            return;
        }
        if self.caret_byte_offset >= self.content.len() {
            return;
        }
        let start = clamp_to_char_boundary(&self.content, self.caret_byte_offset);
        self.caret_byte_offset = start;
        let grapheme_len = self.content[start..]
            .grapheme_indices(true)
            .next()
            .map(|(_, g)| g.len());
        if let Some(len) = grapheme_len {
            self.save_undo_typing(false);
            let end = start + len;
            self.remove_range_and_sync(start, end);
            self.sync_active_style_to_caret();
        }
    }

    pub fn delete_word_forward(&mut self) {
        if self.selected_range().is_some() {
            self.save_undo();
            self.delete_selection();
            self.sync_active_style_to_caret();
            return;
        }
        if self.caret_byte_offset >= self.content.len() {
            return;
        }
        let start = clamp_to_char_boundary(&self.content, self.caret_byte_offset);
        let suffix = &self.content[start..];
        let mut rel_end = suffix.len();
        let mut passed_non_ws = false;
        for (idx, ch) in suffix.char_indices() {
            if ch.is_whitespace() {
                if passed_non_ws {
                    rel_end = idx;
                    break;
                }
            } else {
                passed_non_ws = true;
            }
        }
        if rel_end > 0 {
            self.save_undo();
            self.remove_range_and_sync(start, start + rel_end);
            self.caret_byte_offset = start;
            self.sync_active_style_to_caret();
        }
    }

    pub fn move_left_with_selection(&mut self, keep_selection: bool) {
        if !keep_selection && let Some((from, _)) = self.selected_range() {
            self.caret_byte_offset = from;
            self.selection_start = None;
            self.sync_active_style_to_caret();
            return;
        }
        if keep_selection && self.selection_start.is_none() {
            self.selection_start = Some(self.caret_byte_offset);
        }
        if self.caret_byte_offset > 0
            && let Some((offset, _)) = self.content[..self.caret_byte_offset]
                .grapheme_indices(true)
                .next_back()
        {
            self.caret_byte_offset = offset;
        }
        if !keep_selection {
            self.selection_start = None;
        }
        self.sync_active_style_to_caret();
    }

    pub fn move_right_with_selection(&mut self, keep_selection: bool) {
        if !keep_selection && let Some((_, to)) = self.selected_range() {
            self.caret_byte_offset = to;
            self.selection_start = None;
            self.sync_active_style_to_caret();
            return;
        }
        if keep_selection && self.selection_start.is_none() {
            self.selection_start = Some(self.caret_byte_offset);
        }
        if self.caret_byte_offset < self.content.len()
            && let Some((_, grapheme)) = self.content[self.caret_byte_offset..]
                .grapheme_indices(true)
                .next()
        {
            self.caret_byte_offset += grapheme.len();
        }
        if !keep_selection {
            self.selection_start = None;
        }
        self.sync_active_style_to_caret();
    }

    pub fn move_to_doc_boundary(&mut self, to_end: bool, keep_selection: bool) {
        if keep_selection && self.selection_start.is_none() {
            self.selection_start = Some(self.caret_byte_offset);
        } else if !keep_selection {
            self.selection_start = None;
        }
        self.caret_byte_offset = if to_end { self.content.len() } else { 0 };
        self.sync_active_style_to_caret();
    }

    pub fn move_word_left_with_selection(&mut self, keep_selection: bool) {
        if keep_selection && self.selection_start.is_none() {
            self.selection_start = Some(self.caret_byte_offset);
        } else if !keep_selection {
            self.selection_start = None;
        }
        if self.caret_byte_offset == 0 {
            return;
        }
        let prefix = &self.content[..self.caret_byte_offset];
        let mut it = prefix.char_indices().rev().peekable();
        while let Some(&(_, ch)) = it.peek() {
            if ch.is_whitespace() {
                it.next();
            } else {
                break;
            }
        }
        if let Some(&(_, first_ch)) = it.peek() {
            let is_alnum = first_ch.is_alphanumeric() || first_ch == '_';
            let mut target_offset = 0;
            for (idx, ch) in it {
                let same_class = if is_alnum {
                    ch.is_alphanumeric() || ch == '_'
                } else {
                    !ch.is_whitespace() && !ch.is_alphanumeric() && ch != '_'
                };
                if !same_class {
                    target_offset = idx + ch.len_utf8();
                    break;
                }
            }
            self.caret_byte_offset = target_offset;
        } else {
            self.caret_byte_offset = 0;
        }
        self.sync_active_style_to_caret();
    }

    pub fn move_word_right_with_selection(&mut self, keep_selection: bool) {
        if keep_selection && self.selection_start.is_none() {
            self.selection_start = Some(self.caret_byte_offset);
        } else if !keep_selection {
            self.selection_start = None;
        }
        if self.caret_byte_offset >= self.content.len() {
            return;
        }
        let start = clamp_to_char_boundary(&self.content, self.caret_byte_offset);
        let suffix = &self.content[start..];
        let mut it = suffix.char_indices().peekable();
        while let Some(&(_, ch)) = it.peek() {
            if ch.is_whitespace() {
                it.next();
            } else {
                break;
            }
        }
        if let Some(&(_, first_ch)) = it.peek() {
            let is_alnum = first_ch.is_alphanumeric() || first_ch == '_';
            let mut end_offset = suffix.len();
            for (idx, ch) in it {
                let same_class = if is_alnum {
                    ch.is_alphanumeric() || ch == '_'
                } else {
                    !ch.is_whitespace() && !ch.is_alphanumeric() && ch != '_'
                };
                if !same_class {
                    end_offset = idx;
                    break;
                }
            }
            self.caret_byte_offset = start + end_offset;
        } else {
            self.caret_byte_offset = self.content.len();
        }
        self.sync_active_style_to_caret();
    }

    pub fn select_word_at(&mut self, byte_offset: usize) {
        let clamped = clamp_to_char_boundary(&self.content, byte_offset);
        if self.content.is_empty() {
            return;
        }
        if self.content[clamped..]
            .chars()
            .next()
            .is_some_and(|c| c.is_whitespace())
        {
            let start = self.content[..clamped]
                .char_indices()
                .rev()
                .take_while(|&(_, c)| c.is_whitespace() && c != '\n' && c != PAGE_BREAK_CHAR)
                .last()
                .map(|(i, _)| i)
                .unwrap_or(clamped);
            let rel_end = self.content[clamped..]
                .char_indices()
                .take_while(|&(_, c)| c.is_whitespace() && c != '\n' && c != PAGE_BREAK_CHAR)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(0);
            self.selection_start = Some(start);
            self.caret_byte_offset = clamped + rel_end;
            self.sync_active_style_to_caret();
            return;
        }

        let is_alnum = self.content[clamped..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');

        if is_alnum {
            let start = self.content[..clamped]
                .char_indices()
                .rev()
                .take_while(|&(_, c)| c.is_alphanumeric() || c == '_')
                .last()
                .map(|(i, _)| i)
                .unwrap_or(clamped);
            let rel_end = self.content[clamped..]
                .char_indices()
                .take_while(|&(_, c)| c.is_alphanumeric() || c == '_')
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(0);
            self.selection_start = Some(start);
            self.caret_byte_offset = clamped + rel_end;
            self.sync_active_style_to_caret();
            return;
        }

        let start = self.content[..clamped]
            .char_indices()
            .rev()
            .take_while(|&(_, c)| !c.is_whitespace() && !c.is_alphanumeric() && c != '_')
            .last()
            .map(|(i, _)| i)
            .unwrap_or(clamped);
        let rel_end = self.content[clamped..]
            .char_indices()
            .take_while(|&(_, c)| !c.is_whitespace() && !c.is_alphanumeric() && c != '_')
            .last()
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);
        self.selection_start = Some(start);
        self.caret_byte_offset = clamped + rel_end;
        self.sync_active_style_to_caret();
    }

    pub fn replace_current_match(
        &mut self,
        query: &str,
        replacement: &str,
        case_sensitive: bool,
        active_idx: usize,
    ) -> bool {
        let matches = find_matches(&self.content, query, case_sensitive);
        if matches.is_empty() {
            return false;
        }
        let idx = active_idx.min(matches.len() - 1);
        let (from, to) = matches[idx];
        self.save_undo();
        if let Some(s) = self.spans.iter().find(|s| from >= s.start && from < s.end) {
            self.active_style = s.style;
        }
        self.selection_start = Some(from);
        self.caret_byte_offset = to;
        self.insert_text(replacement);
        true
    }

    pub fn replace_all_matches(
        &mut self,
        query: &str,
        replacement: &str,
        case_sensitive: bool,
    ) -> usize {
        let matches = find_matches(&self.content, query, case_sensitive);
        if matches.is_empty() {
            return 0;
        }
        self.save_undo();
        let rep_norm: String = replacement.nfc().collect();
        let count = matches.len();
        for &(from, to) in matches.iter().rev() {
            let style = self
                .spans
                .iter()
                .find(|s| s.start <= from && from < s.end)
                .map(|s| s.style)
                .unwrap_or(self.active_style);
            self.content.replace_range(from..to, &rep_norm);
            shift_spans_on_delete(&mut self.spans, from, to);
            shift_spans_on_insert(&mut self.spans, from, rep_norm.len(), style);
        }
        self.caret_byte_offset = clamp_to_char_boundary(&self.content, self.caret_byte_offset);
        self.selection_start = None;
        self.sync_para_alignments();
        self.mark_content_changed();
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nfc_normalization_on_insert() {
        let mut ed = EditorState::from_plain_text("");
        ed.insert_text("e\u{0301}");
        assert_eq!(ed.content, "é");
        assert_eq!(ed.content.len(), 2);
        assert_eq!(ed.caret_byte_offset, 2);
    }

    #[test]
    fn test_vietnamese_grapheme_backspace_safety() {
        let mut ed = EditorState::from_plain_text("Tiếng");
        assert_eq!(ed.caret_byte_offset, 7);
        ed.backspace();
        assert_eq!(ed.content, "Tiến");
        ed.backspace();
        assert_eq!(ed.content, "Tiế");
        ed.backspace();
        assert_eq!(ed.content, "Ti");
        assert_eq!(ed.caret_byte_offset, 2);
        assert_eq!(ed.unikey_backspace_count, 3);
    }

    #[test]
    fn test_delete_forward_vietnamese() {
        let mut ed = EditorState::from_plain_text("Việt");
        ed.caret_byte_offset = 0;
        ed.delete_forward();
        assert_eq!(ed.content, "iệt");
        ed.delete_forward();
        assert_eq!(ed.content, "ệt");
        ed.delete_forward();
        assert_eq!(ed.content, "t");
        ed.delete_forward();
        assert_eq!(ed.content, "");
        assert_eq!(ed.caret_byte_offset, 0);
    }

    #[test]
    fn test_caret_movement_boundaries() {
        let mut ed = EditorState::from_plain_text("Tổ quốc");
        let initial_len = ed.content.len();
        ed.move_right_with_selection(false);
        assert_eq!(ed.caret_byte_offset, initial_len);
        while ed.caret_byte_offset > 0 {
            let prev_offset = ed.caret_byte_offset;
            ed.move_left_with_selection(false);
            assert!(ed.caret_byte_offset < prev_offset);
            assert!(ed.content.is_char_boundary(ed.caret_byte_offset));
        }
        ed.move_left_with_selection(false);
        assert_eq!(ed.caret_byte_offset, 0);
    }

    #[test]
    fn test_unikey_simulation_telex() {
        let mut ed = EditorState::from_plain_text("");
        for s in ["v", "i", "ê", "t"] {
            ed.insert_text(s);
        }
        assert_eq!(ed.content, "viêt");
        ed.backspace();
        ed.backspace();
        assert_eq!(ed.content, "vi");
        ed.insert_text("ệt");
        assert_eq!(ed.content, "việt");
        assert_eq!(ed.unikey_backspace_count, 2);
    }

    #[test]
    fn test_unikey_telex_burst_tone_mark() {
        let mut ed = EditorState::from_plain_text("");
        for s in ["t", "i", "ê", "n", "g"] {
            ed.insert_text(s);
        }
        assert_eq!(ed.content, "tiêng");
        ed.backspace();
        ed.backspace();
        ed.backspace();
        for s in ["ế", "n", "g"] {
            ed.insert_text(s);
        }
        assert_eq!(ed.content, "tiếng");
        assert_eq!(ed.unikey_backspace_count, 3);
    }

    #[test]
    fn test_builtin_telex_direct_typing() {
        let mut ed = EditorState::from_plain_text("");
        for ch in "Vieetj Nam".chars() {
            ed.handle_text_input(&ch.to_string(), true);
        }
        assert_eq!(ed.content, "Việt Nam");
    }

    #[test]
    fn test_unikey_escape_no_double_telex_conflict() {
        let mut ed = EditorState::from_plain_text("té");
        ed.backspace();
        ed.handle_text_input("e", false);
        ed.handle_text_input("s", false);
        ed.handle_text_input("t", false);
        assert_eq!(ed.content, "test");
    }

    #[test]
    fn test_undo_redo_cycle() {
        let mut ed = EditorState::from_plain_text("Bắt đầu");
        ed.insert_text(" thêm mới");
        assert_eq!(ed.content, "Bắt đầu thêm mới");
        assert!(ed.undo());
        assert_eq!(ed.content, "Bắt đầu");
        assert!(ed.redo());
        assert_eq!(ed.content, "Bắt đầu thêm mới");
    }

    #[test]
    fn test_undo_coalescing_reduces_snapshots() {
        let mut ed = EditorState::from_plain_text("");
        for ch in "abcdef".chars() {
            ed.insert_text(&ch.to_string());
        }
        assert_eq!(ed.content, "abcdef");
        assert_eq!(ed.undo_stack.len(), 1);
    }

    #[test]
    fn test_selection_and_deletion() {
        let mut ed = EditorState::from_plain_text("Xin chào Việt Nam");
        let start = ed.content.find("chào").unwrap();
        let end = start + "chào".len();
        ed.selection_start = Some(start);
        ed.caret_byte_offset = end;
        assert_eq!(ed.copy_selection(), Some("chào".to_string()));
        ed.delete_selection();
        assert_eq!(ed.content, "Xin  Việt Nam");
    }

    #[test]
    fn test_delete_word_backward() {
        let mut ed = EditorState::from_plain_text("Xin chào các bạn");
        ed.delete_word_backward();
        assert_eq!(ed.content, "Xin chào các ");
        ed.delete_word_backward();
        assert_eq!(ed.content, "Xin chào ");
    }

    #[test]
    fn test_double_click_select_word() {
        let mut ed = EditorState::from_plain_text("Xin chào Việt Nam");
        let idx = ed.content.find("chào").unwrap() + 2;
        ed.select_word_at(idx);
        assert_eq!(ed.copy_selection(), Some("chào".to_string()));
    }

    #[test]
    fn test_rich_text_selection_and_replace_all() {
        let mut ed = EditorState::from_plain_text("Việt Nam đẹp lắm. Người việt yêu hòa bình.");
        let s = ed.content.find("đẹp").unwrap();
        ed.selection_start = Some(s);
        ed.caret_byte_offset = s + "đẹp".len();
        ed.apply_format(|st| st.bold = true);
        assert!(ed.spans.iter().any(|sp| sp.style.bold));

        let count = ed.replace_all_matches("việt", "Việt", false);
        assert_eq!(count, 2);
        assert_eq!(ed.content, "Việt Nam đẹp lắm. Người Việt yêu hòa bình.");
        assert!(ed.undo());
        assert_eq!(ed.content, "Việt Nam đẹp lắm. Người việt yêu hòa bình.");
    }

    #[test]
    fn test_per_paragraph_alignment_and_page_break() {
        let mut ed = EditorState::from_plain_text("Tiêu đề");
        ed.set_alignment(TextAlignment::Center);
        ed.insert_text(&PAGE_BREAK_CHAR.to_string());
        ed.insert_text("Trang hai");
        ed.set_alignment(TextAlignment::Justify);
        assert_eq!(
            ed.para_alignments,
            vec![TextAlignment::Center, TextAlignment::Justify]
        );
    }

    #[test]
    fn test_undo_restores_backspace_and_delete_selection() {
        let mut ed = EditorState::from_plain_text("Xin chào");
        ed.backspace();
        assert_eq!(ed.content, "Xin chà");
        assert!(ed.undo());
        assert_eq!(ed.content, "Xin chào");

        ed.selection_start = Some(0);
        ed.caret_byte_offset = 4;
        ed.backspace();
        assert_eq!(ed.content, "chào");
        assert!(ed.undo());
        assert_eq!(ed.content, "Xin chào");
    }

    #[test]
    fn test_replace_all_preserves_utf8_caret_boundary() {
        let mut ed = EditorState::from_plain_text("abc tiếng Việt");
        // Đặt con trỏ vào byte 8 (trước/sau vùng bị rút ngắn khiến độ dài dịch chuyển)
        ed.caret_byte_offset = ed.content.find("Việt").unwrap() + "Việt".len();
        ed.replace_all_matches("abc", "cộng hòa", false);
        assert!(ed.content.is_char_boundary(ed.caret_byte_offset));
        ed.backspace();
        assert!(ed.content.is_char_boundary(ed.caret_byte_offset));
    }

    #[test]
    fn test_delete_forward_word_and_whitespace_double_click() {
        let mut ed = EditorState::from_plain_text("Cộng hòa Xã hội");
        // Đặt con trỏ ngay sau "Cộng" (trước " hòa") và xóa chữ phía sau -> phải giữ nguyên "Cộng"
        ed.caret_byte_offset = "Cộng".len();
        ed.delete_word_forward();
        assert_eq!(ed.content, "Cộng Xã hội");

        // Double-click vào dấu cách giữa "Cộng" và "Xã" không được chọn lùi vào "Cộng"
        ed.select_word_at("Cộng".len());
        assert_eq!(ed.copy_selection(), Some(" ".to_string()));
        ed.delete_forward();
        assert_eq!(ed.content, "CộngXã hội");
    }

    #[test]
    fn test_double_click_punctuation_separation_and_word_nav() {
        let mut ed = EditorState::from_plain_text("Xin chào, bạn!");
        // Double-click vào chữ "chào" không được dính dấu phẩy
        let chao_pos = ed.content.find("chào").unwrap();
        ed.select_word_at(chao_pos);
        assert_eq!(ed.copy_selection(), Some("chào".to_string()));

        // Double-click vào dấu phẩy ',' chỉ chọn riêng dấu phẩy
        let comma_pos = ed.content.find(',').unwrap();
        ed.select_word_at(comma_pos);
        assert_eq!(ed.copy_selection(), Some(",".to_string()));

        // Di chuyển bằng Ctrl+Arrow (move_word_left/right)
        ed.caret_byte_offset = 0;
        ed.move_word_right_with_selection(false);
        assert_eq!(ed.caret_byte_offset, "Xin".len());
        ed.move_word_right_with_selection(true);
        assert_eq!(ed.selection_start, Some("Xin".len()));
        assert_eq!(ed.copy_selection(), Some(" chào".to_string()));

        ed.move_word_left_with_selection(false);
        assert_eq!(ed.selection_start, None);
        assert_eq!(ed.caret_byte_offset, "Xin ".len());

        ed.move_word_left_with_selection(false);
        assert_eq!(ed.caret_byte_offset, 0);
    }

    #[test]
    fn test_format_painter_copy_and_apply() {
        let mut ed = EditorState::from_plain_text("Văn bản gốc cần sao chép định dạng.");
        // Gán style cho "Văn bản"
        let van_ban_len = "Văn bản".len();
        ed.selection_start = Some(0);
        ed.caret_byte_offset = van_ban_len;
        ed.apply_format(|s| {
            s.bold = true;
            s.font_size = 20.0;
        });

        // Bật format painter tại vùng chọn "Văn bản"
        ed.toggle_format_painter();
        assert!(ed.format_painter_style.is_some());
        let copied = ed.format_painter_style.unwrap();
        assert!(copied.bold);
        assert_eq!(copied.font_size, 20.0);

        // Chọn "định dạng" ở cuối câu
        let target_pos = ed.content.find("định dạng").unwrap();
        let target_len = "định dạng".len();
        ed.selection_start = Some(target_pos);
        ed.caret_byte_offset = target_pos + target_len;

        // Áp dụng chổi sơn
        let applied = ed.apply_format_painter_if_active();
        assert!(applied);
        assert!(ed.format_painter_style.is_none()); // Đã tự tắt sau khi sơn

        // Kiểm tra span tại "định dạng"
        let span_match = ed
            .spans
            .iter()
            .find(|s| s.start <= target_pos && target_pos < s.end);
        assert!(span_match.is_some());
        let s = span_match.unwrap();
        assert!(s.style.bold);
        assert_eq!(s.style.font_size, 20.0);
    }

    #[test]
    fn test_clear_formatting() {
        let mut ed = EditorState::from_plain_text("Đoạn văn được định dạng rất nhiều.");
        ed.selection_start = Some(0);
        ed.caret_byte_offset = ed.content.len();
        ed.apply_format(|s| {
            s.bold = true;
            s.italic = true;
            s.font_size = 24.0;
        });

        // Xóa định dạng toàn bộ
        ed.clear_formatting();
        let def = TextStyle::default();
        assert_eq!(ed.active_style, def);
        for s in &ed.spans {
            assert_eq!(s.style, def);
        }
    }

    #[test]
    fn test_document_preset_styles() {
        let mut ed = EditorState::from_plain_text("Dòng tiêu đề chính\nDòng nội dung bình thường.");
        // Áp dụng Heading 1 cho dòng đầu tiên
        let line1_len = "Dòng tiêu đề chính".len();
        ed.caret_byte_offset = 5; // Nằm ở giữa dòng 1
        ed.apply_preset_style(DocumentPresetStyle::Heading1);

        let h1_style = DocumentPresetStyle::Heading1.to_text_style();
        assert_eq!(ed.active_style.font_size, h1_style.font_size);
        assert!(ed.active_style.bold);

        // Kiểm tra span dòng 1 được áp dụng h1_style
        let span1 = ed.spans.iter().find(|s| s.start == 0).unwrap();
        assert_eq!(span1.end, line1_len);
        assert_eq!(span1.style.font_size, 22.0);

        // Áp dụng Quote cho dòng 2
        ed.caret_byte_offset = line1_len + 5;
        ed.apply_preset_style(DocumentPresetStyle::Quote);
        let q_style = DocumentPresetStyle::Quote.to_text_style();
        assert!(ed.active_style.italic);
        assert_eq!(ed.active_style.color, q_style.color);

        // Áp dụng Heading 3
        ed.apply_preset_style(DocumentPresetStyle::Heading3);
        assert_eq!(ed.active_style.font_size, 16.0);
        assert!(ed.active_style.bold);
    }

    #[test]
    fn test_bullet_and_numbered_lists() {
        let mut ed = EditorState::from_plain_text("Đoạn 1\nĐoạn 2");
        ed.caret_byte_offset = 0;
        ed.insert_bullet_item();
        assert!(ed.content.starts_with("• Đoạn 1"));

        // Chuyển tới Đoạn 2 và chèn số thứ tự
        let d2_pos = ed.content.find("Đoạn 2").unwrap();
        ed.caret_byte_offset = d2_pos;
        ed.insert_numbered_item();
        assert!(ed.content.contains("2. Đoạn 2"));
    }
}
