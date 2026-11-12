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
}

pub struct PlacedPara {
    pub para_data_idx: usize,
    pub y_in_page: f32,
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

pub fn paint_range_highlight(
    painter: &egui::Painter,
    galley: &egui::Galley,
    line: &str,
    local_from: usize,
    local_to: usize,
    draw_origin: Pos2,
    fill: Color32,
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
    for row in &galley.rows {
        let row_len = row.glyphs.len();
        let row_end_c = row_char_offset + row_len;
        if start_c < row_end_c && end_c > row_char_offset {
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

            paras.push(ParaRenderData {
                start: scan_start,
                end: i,
                alignment: align,
                followed_by_page_break: sep_char == Some(PAGE_BREAK_CHAR),
                galley,
                height,
                gap,
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
        if y_used > 0.0 && y_used + p.height > usable_height {
            pages.push(Vec::new());
            y_used = 0.0;
        }
        if let Some(page) = pages.last_mut() {
            page.push(PlacedPara {
                para_data_idx: p_data_idx,
                y_in_page: y_used,
            });
        }
        y_used += p.height + p.gap;
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

pub fn caret_page_index(cache: &CachedLayout, caret_byte_offset: usize) -> usize {
    for (pi, placed_list) in cache.pages.iter().enumerate() {
        if placed_list.iter().any(|pl| {
            let p = &cache.paras[pl.para_data_idx];
            caret_byte_offset >= p.start && caret_byte_offset <= p.end
        }) {
            return pi;
        }
    }
    0
}
