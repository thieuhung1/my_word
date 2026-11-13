use crate::document::{PAGE_BREAK_CHAR, TextAlignment, TextStyle, spans_for_slice};
use crate::editor::EditorState;
use eframe::egui::{self, Align, Color32, FontFamily, FontId, Pos2, Rect, Stroke};
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

/// Kích thước chuẩn khổ giấy A4 tính theo point (72 DPI) và lề chuẩn 1 inch
pub const A4_WIDTH_PT: f32 = 595.0;
pub const A4_HEIGHT_PT: f32 = 842.0;
pub const PAGE_MARGIN_PT: f32 = 72.0;

#[derive(Clone, Copy)]
pub enum PointerQuery {
    Click(Pos2),
    Drag(Pos2),
    DoubleClick(Pos2),
    MoveVertical(Pos2),
}

pub struct ParaRenderData {
    pub start: usize,
    pub end: usize,
    pub alignment: TextAlignment,
    pub followed_by_page_break: bool,
    pub galley: Arc<egui::Galley>,
    pub height: f32,
    pub gap: f32,
    pub row_char_offsets: Vec<usize>,
}

#[derive(Clone, Copy)]
pub struct PlacedPara {
    pub para_data_idx: usize,
    pub y_in_page: f32,
    pub row_start: usize,
    pub row_end: usize,
}

impl PlacedPara {
    pub fn byte_range(&self, p: &ParaRenderData, content: &str) -> (usize, usize) {
        let p_text = &content[p.start..p.end];
        let total_chars = p_text.chars().count();
        let c_start = if self.row_start < p.row_char_offsets.len() {
            p.row_char_offsets[self.row_start]
        } else {
            0
        };
        let c_end = if self.row_end < p.row_char_offsets.len() {
            p.row_char_offsets[self.row_end]
        } else {
            total_chars
        };
        let b_start = p.start + byte_offset_from_char_index(p_text, c_start);
        let b_end = p.start + byte_offset_from_char_index(p_text, c_end);
        (b_start, b_end)
    }
}

#[allow(dead_code)]
pub struct VisualLineInfo {
    pub page_idx: usize,
    pub para_idx: usize,
    pub row_idx: usize,
    pub byte_start: usize,
    pub byte_end: usize,
}

/// Bộ đệm bố cục & thống kê: chỉ tính lại khi nội dung, định dạng hoặc zoom thay đổi
pub struct CachedLayout {
    pub zoom: f32,
    pub usable_width: f32,
    pub usable_height: f32,
    pub paras: Vec<ParaRenderData>,
    pub pages: Vec<Vec<PlacedPara>>,
    pub word_count: usize,
    pub char_count: usize,
}

impl CachedLayout {
    pub fn visual_lines(&self, content: &str) -> Vec<VisualLineInfo> {
        let mut lines = Vec::new();
        for (page_idx, placed_list) in self.pages.iter().enumerate() {
            for pl in placed_list {
                let p = &self.paras[pl.para_data_idx];
                let p_text = &content[p.start..p.end];
                let total_chars = p_text.chars().count();

                if p.galley.rows.is_empty() || pl.row_start >= pl.row_end {
                    lines.push(VisualLineInfo {
                        page_idx,
                        para_idx: pl.para_data_idx,
                        row_idx: 0,
                        byte_start: p.start,
                        byte_end: p.end,
                    });
                } else {
                    for r_idx in pl.row_start..pl.row_end {
                        let c_start = p.row_char_offsets[r_idx];
                        let c_end = if r_idx + 1 < p.row_char_offsets.len() {
                            p.row_char_offsets[r_idx + 1]
                        } else {
                            total_chars
                        };
                        let b_start = p.start + byte_offset_from_char_index(p_text, c_start);
                        let b_end = p.start + byte_offset_from_char_index(p_text, c_end);
                        lines.push(VisualLineInfo {
                            page_idx,
                            para_idx: pl.para_data_idx,
                            row_idx: r_idx,
                            byte_start: b_start,
                            byte_end: b_end,
                        });
                    }
                }
            }
        }
        lines
    }

    pub fn line_boundary_at(&self, content: &str, caret_byte_offset: usize) -> (usize, usize) {
        let lines = self.visual_lines(content);
        for l in &lines {
            if caret_byte_offset >= l.byte_start && caret_byte_offset <= l.byte_end {
                return (l.byte_start, l.byte_end);
            }
        }
        if let Some(last) = lines.last() {
            (last.byte_start, last.byte_end)
        } else {
            (0, content.len())
        }
    }

    pub fn navigate_vertical_line(
        &self,
        content: &str,
        caret_byte_offset: usize,
        is_up: bool,
    ) -> Option<usize> {
        let lines = self.visual_lines(content);
        if lines.is_empty() {
            return None;
        }

        let mut curr_idx = 0;
        for (i, l) in lines.iter().enumerate() {
            if caret_byte_offset >= l.byte_start && caret_byte_offset <= l.byte_end {
                curr_idx = i;
                break;
            }
        }

        let target_idx = if is_up {
            if curr_idx == 0 {
                return Some(0);
            }
            curr_idx - 1
        } else {
            if curr_idx + 1 >= lines.len() {
                return Some(content.len());
            }
            curr_idx + 1
        };

        let curr_line = &lines[curr_idx];
        let target_line = &lines[target_idx];
        let rel_in_curr = caret_byte_offset.saturating_sub(curr_line.byte_start);
        let target_len = target_line.byte_end - target_line.byte_start;
        let rel_clamped = rel_in_curr.min(target_len);
        let target_byte = target_line.byte_start + rel_clamped;

        Some(crate::editor::clamp_to_char_boundary(content, target_byte))
    }
}

pub fn font_id_for_style(style: &TextStyle, zoom: f32) -> FontId {
    let scaled = style.font_size * zoom;
    match (style.bold, style.italic) {
        (true, true) => FontId::new(scaled, FontFamily::Name("BoldItalic".into())),
        (true, false) => FontId::new(scaled, FontFamily::Name("Bold".into())),
        (false, true) => FontId::new(scaled, FontFamily::Name("Italic".into())),
        (false, false) => FontId::proportional(scaled),
    }
}

pub fn byte_offset_from_char_index(line: &str, char_idx: usize) -> usize {
    line.char_indices()
        .nth(char_idx)
        .map_or(line.len(), |(b, _)| b)
}

pub fn char_index_from_byte_offset(line: &str, byte_offset: usize) -> usize {
    line[..byte_offset.min(line.len())].chars().count()
}

#[allow(clippy::too_many_arguments)]
pub fn paint_range_highlight(
    painter: &egui::Painter,
    galley: &egui::Galley,
    line: &str,
    local_from: usize,
    local_to: usize,
    draw_origin: Pos2,
    fill: Color32,
    row_start: usize,
    row_end: usize,
) {
    if local_from >= local_to {
        return;
    }
    let start_c = char_index_from_byte_offset(line, local_from);
    let end_c = char_index_from_byte_offset(line, local_to);
    if start_c >= end_c {
        return;
    }

    let mut row_char_offset = 0;
    for (row_idx, row) in galley.rows.iter().enumerate() {
        let row_len = row.glyphs.len();
        let row_end_c = row_char_offset + row_len;
        if row_idx >= row_start
            && row_idx < row_end
            && start_c < row_end_c
            && end_c > row_char_offset
        {
            let s_idx = start_c.saturating_sub(row_char_offset);
            let e_idx = (end_c - row_char_offset).min(row_len);
            let x1 = if s_idx < row.glyphs.len() {
                row.glyphs[s_idx].pos.x
            } else {
                row.rect.max.x
            };
            let x2 = if e_idx > 0 && e_idx <= row.glyphs.len() {
                let g = &row.glyphs[e_idx - 1];
                g.pos.x + g.size.x
            } else {
                row.rect.min.x
            };
            if x2 > x1 {
                let r = Rect::from_min_max(
                    Pos2::new(draw_origin.x + x1, draw_origin.y + row.rect.min.y),
                    Pos2::new(draw_origin.x + x2, draw_origin.y + row.rect.max.y),
                );
                painter.rect_filled(r, 2.0, fill);
            }
        }
        row_char_offset = row_end_c + usize::from(row.ends_with_newline);
    }
}

/// Tính lại `CachedLayout` khi `editor.layout_dirty` hoặc `zoom` thay đổi và trả về tham chiếu hợp lệ
pub fn ensure_layout_cache<'a>(
    cache: &'a mut Option<CachedLayout>,
    editor: &mut EditorState,
    zoom: f32,
    ctx: &egui::Context,
) -> &'a CachedLayout {
    let dirty = editor.layout_dirty
        || cache
            .as_ref()
            .is_some_and(|c| (c.zoom - zoom).abs() > f32::EPSILON);
    if dirty {
        *cache = None;
    }
    cache.get_or_insert_with(|| {
        let built = build_layout_cache(editor, zoom, ctx);
        editor.layout_dirty = false;
        built
    })
}

fn build_layout_cache(editor: &EditorState, zoom: f32, ctx: &egui::Context) -> CachedLayout {
    let page_width = A4_WIDTH_PT * zoom;
    let page_height = A4_HEIGHT_PT * zoom;
    let margin = PAGE_MARGIN_PT * zoom;
    let usable_width = (page_width - margin * 2.0).max(100.0);
    let usable_height = (page_height - margin * 2.0).max(100.0);
    let doc_text_color = Color32::from_rgb(30, 41, 59);
    let base_line_height = (editor.active_style.font_size * 1.6) * zoom;

    let mut paras = Vec::new();
    ctx.fonts(|fonts| {
        let mut scan_start = 0usize;
        let mut p_idx = 0usize;
        let mut i = 0usize;

        loop {
            let sep_char = if let Some(c) = editor.content[i..].chars().next() {
                if c == '\n' || c == PAGE_BREAK_CHAR {
                    Some(c)
                } else {
                    i += c.len_utf8();
                    continue;
                }
            } else {
                None
            };

            let slice = &editor.content[scan_start..i];
            let align = editor
                .para_alignments
                .get(p_idx)
                .copied()
                .unwrap_or(TextAlignment::Left);
            let mut job = egui::text::LayoutJob {
                wrap: egui::text::TextWrapping {
                    max_width: usable_width,
                    ..Default::default()
                },
                halign: match align {
                    TextAlignment::Left | TextAlignment::Justify => Align::LEFT,
                    TextAlignment::Center => Align::Center,
                    TextAlignment::Right => Align::RIGHT,
                },
                justify: align == TextAlignment::Justify,
                ..Default::default()
            };

            let local_spans = spans_for_slice(&editor.spans, scan_start, i);
            if local_spans.is_empty() {
                job.append(
                    "",
                    0.0,
                    egui::TextFormat {
                        font_id: font_id_for_style(&editor.active_style, zoom),
                        color: doc_text_color,
                        line_height: Some((editor.active_style.font_size * 1.55) * zoom),
                        ..Default::default()
                    },
                );
            } else {
                for (ls, le, st) in local_spans {
                    let col = Color32::from_rgb(st.color[0], st.color[1], st.color[2]);
                    let bg = match st.highlight {
                        Some([r, g, b]) => Color32::from_rgb(r, g, b),
                        None => Color32::TRANSPARENT,
                    };
                    job.append(
                        &slice[ls..le],
                        0.0,
                        egui::TextFormat {
                            font_id: font_id_for_style(&st, zoom),
                            color: col,
                            background: bg,
                            underline: if st.underline {
                                Stroke::new(1.1_f32 * zoom, col)
                            } else {
                                Stroke::NONE
                            },
                            strikethrough: if st.strikethrough {
                                Stroke::new(1.2_f32 * zoom, col)
                            } else {
                                Stroke::NONE
                            },
                            line_height: Some((st.font_size * 1.55) * zoom),
                            ..Default::default()
                        },
                    );
                }
            }

            let galley = fonts.layout_job(job);
            let height = galley.size().y.max(base_line_height);
            let gap = if slice.is_empty() {
                6.0 * zoom
            } else {
                3.0 * zoom
            };

            let mut row_char_offsets = Vec::with_capacity(galley.rows.len());
            let mut c_off = 0;
            for row in &galley.rows {
                row_char_offsets.push(c_off);
                c_off += row.glyphs.len() + usize::from(row.ends_with_newline);
            }

            paras.push(ParaRenderData {
                start: scan_start,
                end: i,
                alignment: align,
                followed_by_page_break: sep_char == Some(PAGE_BREAK_CHAR),
                galley,
                height,
                gap,
                row_char_offsets,
            });

            if let Some(c) = sep_char {
                scan_start = i + c.len_utf8();
                i = scan_start;
                p_idx += 1;
            } else {
                break;
            }
        }
    });

    let mut pages: Vec<Vec<PlacedPara>> = vec![Vec::new()];
    let mut y_used = 0.0_f32;
    for (p_data_idx, p) in paras.iter().enumerate() {
        let total_rows = p.galley.rows.len();
        if total_rows == 0 {
            if y_used > 0.0 && y_used + p.height > usable_height {
                pages.push(Vec::new());
                y_used = 0.0;
            }
            if let Some(page) = pages.last_mut() {
                page.push(PlacedPara {
                    para_data_idx: p_data_idx,
                    y_in_page: y_used,
                    row_start: 0,
                    row_end: 0,
                });
            }
            y_used += p.height + p.gap;
        } else {
            let mut curr_row_start = 0;
            while curr_row_start < total_rows {
                let start_row_min_y = p.galley.rows[curr_row_start].rect.min.y;
                let mut r = curr_row_start;
                while r < total_rows {
                    let chunk_h = p.galley.rows[r].rect.max.y - start_row_min_y;
                    if y_used + chunk_h > usable_height {
                        if r == curr_row_start {
                            if y_used > 0.0 {
                                pages.push(Vec::new());
                                y_used = 0.0;
                                break;
                            } else {
                                if let Some(page) = pages.last_mut() {
                                    page.push(PlacedPara {
                                        para_data_idx: p_data_idx,
                                        y_in_page: 0.0,
                                        row_start: curr_row_start,
                                        row_end: curr_row_start + 1,
                                    });
                                }
                                pages.push(Vec::new());
                                y_used = 0.0;
                                curr_row_start += 1;
                                break;
                            }
                        } else {
                            if let Some(page) = pages.last_mut() {
                                page.push(PlacedPara {
                                    para_data_idx: p_data_idx,
                                    y_in_page: y_used,
                                    row_start: curr_row_start,
                                    row_end: r,
                                });
                            }
                            pages.push(Vec::new());
                            y_used = 0.0;
                            curr_row_start = r;
                            break;
                        }
                    }
                    r += 1;
                }
                if r == total_rows && curr_row_start < total_rows {
                    let chunk_h = p.galley.rows[total_rows - 1].rect.max.y - start_row_min_y;
                    if let Some(page) = pages.last_mut() {
                        page.push(PlacedPara {
                            para_data_idx: p_data_idx,
                            y_in_page: y_used,
                            row_start: curr_row_start,
                            row_end: total_rows,
                        });
                    }
                    y_used += chunk_h + p.gap;
                    curr_row_start = total_rows;
                }
            }
        }
        if p.followed_by_page_break {
            pages.push(Vec::new());
            y_used = 0.0;
        }
    }

    CachedLayout {
        zoom,
        usable_width,
        usable_height,
        paras,
        pages,
        word_count: editor.content.unicode_words().count(),
        char_count: editor.content.chars().count(),
    }
}

pub fn caret_page_index(cache: &CachedLayout, content: &str, caret_byte_offset: usize) -> usize {
    for (pi, placed_list) in cache.pages.iter().enumerate() {
        if placed_list.iter().any(|pl| {
            let p = &cache.paras[pl.para_data_idx];
            if caret_byte_offset < p.start || caret_byte_offset > p.end {
                return false;
            }
            if p.galley.rows.is_empty() || pl.row_start >= pl.row_end {
                return true;
            }
            let (b_start, b_end) = pl.byte_range(p, content);
            caret_byte_offset >= b_start && caret_byte_offset <= b_end
        }) {
            return pi;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_long_paragraph_pagination_splits_across_pages() {
        let ctx = egui::Context::default();
        crate::platform::setup_fonts(&ctx);
        let _ = ctx.run(Default::default(), |ctx| {
            let long_text = "Dòng văn bản rất dài không có ngắt dòng Enter để thử nghiệm cơ chế chia trang theo từng dòng galley rows. ".repeat(60);
            let ed = EditorState::from_plain_text(&long_text);
            let cache = build_layout_cache(&ed, 1.0, ctx);

            // Đoạn văn dài phải được ngắt thành nhiều hơn 1 trang A4
            assert!(
                cache.pages.len() >= 2,
                "Đoạn văn dài phải tự ngắt sang ít nhất 2 trang, thực tế: {}",
                cache.pages.len()
            );

            // Kiểm tra phân chia row_start..row_end không bị chồng lấn
            let p0 = &cache.pages[0][0];
            let p1 = &cache.pages[1][0];
            assert_eq!(p0.para_data_idx, p1.para_data_idx);
            assert_eq!(p0.row_start, 0);
            assert!(p0.row_end > 0);
            assert_eq!(p1.row_start, p0.row_end);

            // Kiểm tra điều hướng dòng trực quan
            let lines = cache.visual_lines(&long_text);
            assert!(!lines.is_empty());
            let (b_start, b_end) = cache.line_boundary_at(&long_text, 0);
            assert_eq!(b_start, 0);
            assert!(b_end > 0);

            // Điều hướng xuống dòng kế tiếp
            let next_caret = cache.navigate_vertical_line(&long_text, 0, false);
            assert!(next_caret.is_some());
            assert!(next_caret.unwrap() > 0);
        });
    }
}
