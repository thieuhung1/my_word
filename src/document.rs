use std::path::Path;
use unicode_normalization::UnicodeNormalization;

/// Ký tự ngắt trang thủ công (ASCII Form Feed U+000C)
pub const PAGE_BREAK_CHAR: char = '\x0c';

/// Kiểu căn lề đoạn văn bản
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextAlignment {
    Left,
    Center,
    Right,
    Justify,
}

impl TextAlignment {
    fn to_code(self) -> &'static str {
        match self {
            TextAlignment::Left => "L",
            TextAlignment::Center => "C",
            TextAlignment::Right => "R",
            TextAlignment::Justify => "J",
        }
    }

    fn from_code(s: &str) -> Self {
        match s {
            "C" => TextAlignment::Center,
            "R" => TextAlignment::Right,
            "J" => TextAlignment::Justify,
            _ => TextAlignment::Left,
        }
    }
}

/// Định dạng Rich Text áp dụng cho từng đoạn chữ (Span)
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TextStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub font_size: f32,
    pub color: [u8; 3],
    pub highlight: Option<[u8; 3]>,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            font_size: 14.0,
            color: [30, 41, 59], // Slate-800
            highlight: None,
        }
    }
}

/// Một dải byte [start..end) mang cùng kiểu định dạng
#[derive(Clone, PartialEq, Debug)]
pub struct StyleSpan {
    pub start: usize,
    pub end: usize,
    pub style: TextStyle,
}

/// Ảnh chụp trạng thái để Hoàn tác / Làm lại cả chữ lẫn định dạng
#[derive(Clone)]
pub struct DocumentSnapshot {
    pub content: String,
    pub caret_byte_offset: usize,
    pub spans: Vec<StyleSpan>,
    pub para_alignments: Vec<TextAlignment>,
}

/// Chuẩn hóa danh sách spans: loại rỗng, sắp xếp, gộp các span kề nhau cùng style
pub fn normalize_spans(spans: &mut Vec<StyleSpan>) {
    spans.retain(|s| s.start < s.end);
    spans.sort_by_key(|s| (s.start, s.end));
    let mut merged: Vec<StyleSpan> = Vec::with_capacity(spans.len());
    for span in spans.drain(..) {
        if let Some(last) = merged.last_mut()
            && last.end == span.start
            && last.style == span.style
        {
            last.end = span.end;
            continue;
        }
        merged.push(span);
    }
    *spans = merged;
}

/// Bảo đảm vùng [0..total_len) được phủ kín bởi các spans liên tiếp
pub fn ensure_full_coverage(spans: &mut Vec<StyleSpan>, total_len: usize, fallback: TextStyle) {
    normalize_spans(spans);
    if total_len == 0 {
        spans.clear();
        return;
    }
    let mut result = Vec::with_capacity(spans.len() + 2);
    let mut cursor = 0;
    for s in spans.iter() {
        let s_start = s.start.min(total_len);
        let s_end = s.end.min(total_len);
        if s_start > cursor {
            result.push(StyleSpan {
                start: cursor,
                end: s_start,
                style: fallback,
            });
        }
        if s_end > s_start.max(cursor) {
            result.push(StyleSpan {
                start: s_start.max(cursor),
                end: s_end,
                style: s.style,
            });
            cursor = s_end;
        }
    }
    if cursor < total_len {
        result.push(StyleSpan {
            start: cursor,
            end: total_len,
            style: fallback,
        });
    }
    *spans = result;
    normalize_spans(spans);
}

/// Cập nhật spans khi chèn đoạn dài `len` bytes tại `pos`
pub fn shift_spans_on_insert(spans: &mut Vec<StyleSpan>, pos: usize, len: usize, style: TextStyle) {
    if len == 0 {
        return;
    }
    let mut next = Vec::with_capacity(spans.len() + 2);
    for s in spans.drain(..) {
        if s.end <= pos {
            next.push(s);
        } else if s.start >= pos {
            next.push(StyleSpan {
                start: s.start + len,
                end: s.end + len,
                style: s.style,
            });
        } else {
            // Cắt đôi span chứa pos
            next.push(StyleSpan {
                start: s.start,
                end: pos,
                style: s.style,
            });
            next.push(StyleSpan {
                start: pos + len,
                end: s.end + len,
                style: s.style,
            });
        }
    }
    next.push(StyleSpan {
        start: pos,
        end: pos + len,
        style,
    });
    *spans = next;
    normalize_spans(spans);
}

/// Cập nhật spans khi xóa vùng byte `[from..to)`
pub fn shift_spans_on_delete(spans: &mut Vec<StyleSpan>, from: usize, to: usize) {
    if from >= to {
        return;
    }
    let del_len = to - from;
    let mut next = Vec::with_capacity(spans.len());
    for s in spans.drain(..) {
        if s.end <= from {
            next.push(s);
        } else if s.start >= to {
            next.push(StyleSpan {
                start: s.start - del_len,
                end: s.end - del_len,
                style: s.style,
            });
        } else {
            if s.start < from {
                next.push(StyleSpan {
                    start: s.start,
                    end: from,
                    style: s.style,
                });
            }
            if s.end > to {
                next.push(StyleSpan {
                    start: from,
                    end: s.end - del_len,
                    style: s.style,
                });
            }
        }
    }
    *spans = next;
    normalize_spans(spans);
}

/// Áp dụng biến đổi style lên dải byte `[from..to)`
pub fn apply_style_on_range<F>(
    spans: &mut Vec<StyleSpan>,
    total_len: usize,
    from: usize,
    to: usize,
    mut f: F,
) where
    F: FnMut(&mut TextStyle),
{
    if from >= to || from >= total_len {
        return;
    }
    let to = to.min(total_len);
    ensure_full_coverage(spans, total_len, TextStyle::default());

    let mut next = Vec::with_capacity(spans.len() + 2);
    for s in spans.drain(..) {
        if s.end <= from || s.start >= to {
            next.push(s);
        } else {
            if s.start < from {
                next.push(StyleSpan {
                    start: s.start,
                    end: from,
                    style: s.style,
                });
            }
            let mid_start = s.start.max(from);
            let mid_end = s.end.min(to);
            if mid_start < mid_end {
                let mut modified = s.style;
                f(&mut modified);
                next.push(StyleSpan {
                    start: mid_start,
                    end: mid_end,
                    style: modified,
                });
            }
            if s.end > to {
                next.push(StyleSpan {
                    start: to,
                    end: s.end,
                    style: s.style,
                });
            }
        }
    }
    *spans = next;
    normalize_spans(spans);
}

/// Lấy danh sách các span cắt đúng trong phạm vi đoạn `[para_start..para_end)` (quy đổi sang local offset)
pub fn spans_for_slice(
    spans: &[StyleSpan],
    para_start: usize,
    para_end: usize,
) -> Vec<(usize, usize, TextStyle)> {
    let para_len = para_end.saturating_sub(para_start);
    if para_len == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut cursor = 0;
    for s in spans {
        if s.end <= para_start || s.start >= para_end {
            continue;
        }
        let local_s = s.start.saturating_sub(para_start);
        let local_e = (s.end.min(para_end)).saturating_sub(para_start);
        if local_s > cursor {
            out.push((cursor, local_s, TextStyle::default()));
        }
        if local_e > local_s {
            out.push((local_s, local_e, s.style));
            cursor = local_e;
        }
    }
    if cursor < para_len {
        out.push((cursor, para_len, TextStyle::default()));
    }
    out
}

/// Tìm tất cả vị trí khớp chuỗi (tìm chính xác theo ký tự UTF-8, hỗ trợ không phân biệt hoa/thường tiếng Việt)
pub fn find_matches(content: &str, query: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    if query.is_empty() || content.is_empty() {
        return Vec::new();
    }
    let query_norm: String = query.nfc().collect();
    if case_sensitive {
        return content
            .match_indices(&query_norm)
            .map(|(idx, matched)| (idx, idx + matched.len()))
            .collect();
    }

    let query_lower: Vec<char> = query_norm.chars().flat_map(|c| c.to_lowercase()).collect();
    if query_lower.is_empty() {
        return Vec::new();
    }

    // Duyệt streaming trực tiếp trên content, không cấp phát Vec toàn văn bản
    let mut results = Vec::new();
    let mut skip_until_byte = 0;
    for (start_b, _) in content.char_indices() {
        if start_b < skip_until_byte {
            continue;
        }
        let mut q_idx = 0;
        let mut end_b = start_b;
        let mut matched = false;
        'match_loop: for (rel_b, ch) in content[start_b..].char_indices() {
            end_b = start_b + rel_b + ch.len_utf8();
            for lc in ch.to_lowercase() {
                if lc != query_lower[q_idx] {
                    break 'match_loop;
                }
                q_idx += 1;
                if q_idx == query_lower.len() {
                    matched = true;
                    break 'match_loop;
                }
            }
        }
        if matched {
            results.push((start_b, end_b));
            skip_until_byte = end_b;
        }
    }
    results
}

/// Tuần tự hóa tài liệu Rich Text sang định dạng `.mwd` (không cần thư viện ngoài)
pub fn serialize_mwd(content: &str, spans: &[StyleSpan], aligns: &[TextAlignment]) -> String {
    let mut out = String::from("MYWORD_V1\n");
    let align_codes: Vec<&str> = aligns.iter().map(|a| a.to_code()).collect();
    out.push_str("ALIGN=");
    out.push_str(&align_codes.join(","));
    out.push('\n');

    for s in spans {
        let hl = match s.style.highlight {
            Some([r, g, b]) => format!("{},{},{}", r, g, b),
            None => "none".to_string(),
        };
        out.push_str(&format!(
            "SPAN={}..{}|B:{}|I:{}|U:{}|S:{}|SZ:{:.1}|C:{},{},{}|H:{}\n",
            s.start,
            s.end,
            u8::from(s.style.bold),
            u8::from(s.style.italic),
            u8::from(s.style.underline),
            u8::from(s.style.strikethrough),
            s.style.font_size,
            s.style.color[0],
            s.style.color[1],
            s.style.color[2],
            hl
        ));
    }
    out.push_str("---CONTENT---\n");
    out.push_str(content);
    out
}

/// Giải mã tệp `.mwd` hoặc `.txt`/`.md` thường
pub fn deserialize_document(
    raw: &str,
    path: &Path,
) -> (String, Vec<StyleSpan>, Vec<TextAlignment>) {
    let is_mwd = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("mwd"))
        .unwrap_or(false)
        || raw.starts_with("MYWORD_V1\n");

    if !is_mwd {
        let text: String = raw.replace("\r\n", "\n").nfc().collect();
        let len = text.len();
        let spans = if len > 0 {
            vec![StyleSpan {
                start: 0,
                end: len,
                style: TextStyle::default(),
            }]
        } else {
            Vec::new()
        };
        let para_count = text.split(['\n', PAGE_BREAK_CHAR]).count().max(1);
        return (text, spans, vec![TextAlignment::Left; para_count]);
    }

    let normalized_raw = raw.replace("\r\n", "\n");
    let marker = "\n---CONTENT---\n";
    let Some(split_idx) = normalized_raw.find(marker) else {
        let text: String = normalized_raw.nfc().collect();
        return (text, Vec::new(), vec![TextAlignment::Left]);
    };

    let header = &normalized_raw[..split_idx];
    let content: String = normalized_raw[split_idx + marker.len()..].nfc().collect();

    let mut aligns = Vec::new();
    let mut spans = Vec::new();

    for line in header.lines() {
        if let Some(rest) = line.strip_prefix("ALIGN=") {
            for code in rest.split(',') {
                if !code.is_empty() {
                    aligns.push(TextAlignment::from_code(code));
                }
            }
        } else if let Some(rest) = line.strip_prefix("SPAN=")
            && let Some(span) = parse_span_line(rest, &content)
        {
            spans.push(span);
        }
    }

    let para_count = content.split(['\n', PAGE_BREAK_CHAR]).count().max(1);
    while aligns.len() < para_count {
        aligns.push(TextAlignment::Left);
    }
    aligns.truncate(para_count);
    ensure_full_coverage(&mut spans, content.len(), TextStyle::default());

    (content, spans, aligns)
}

fn parse_span_line(s: &str, content: &str) -> Option<StyleSpan> {
    let mut parts = s.split('|');
    let range_str = parts.next()?;
    let (a, b) = range_str.split_once("..")?;
    let start: usize = a.parse().ok()?;
    let end: usize = b.parse().ok()?;
    if start >= end
        || end > content.len()
        || !content.is_char_boundary(start)
        || !content.is_char_boundary(end)
    {
        return None;
    }

    let mut style = TextStyle::default();
    for part in parts {
        if let Some(v) = part.strip_prefix("B:") {
            style.bold = v == "1";
        } else if let Some(v) = part.strip_prefix("I:") {
            style.italic = v == "1";
        } else if let Some(v) = part.strip_prefix("U:") {
            style.underline = v == "1";
        } else if let Some(v) = part.strip_prefix("S:") {
            style.strikethrough = v == "1";
        } else if let Some(v) = part.strip_prefix("SZ:") {
            if let Ok(sz) = v.parse::<f32>()
                && sz.is_finite()
            {
                style.font_size = sz.clamp(8.0, 72.0);
            }
        } else if let Some(v) = part.strip_prefix("C:") {
            let rgb: Vec<u8> = v.split(',').filter_map(|n| n.parse().ok()).collect();
            if rgb.len() == 3 {
                style.color = [rgb[0], rgb[1], rgb[2]];
            }
        } else if let Some(v) = part.strip_prefix("H:") {
            if v == "none" {
                style.highlight = None;
            } else {
                let rgb: Vec<u8> = v.split(',').filter_map(|n| n.parse().ok()).collect();
                if rgb.len() == 3 {
                    style.highlight = Some([rgb[0], rgb[1], rgb[2]]);
                }
            }
        }
    }

    Some(StyleSpan { start, end, style })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rich_text_span_apply_and_edit() {
        let mut content = String::from("Cộng hòa Việt Nam");
        let mut spans = vec![StyleSpan {
            start: 0,
            end: content.len(),
            style: TextStyle::default(),
        }];

        // Bôi đen chữ "Việt" và in đậm
        let v_start = content.find("Việt").unwrap();
        let v_end = v_start + "Việt".len();
        apply_style_on_range(&mut spans, content.len(), v_start, v_end, |s| s.bold = true);

        assert_eq!(spans.len(), 3);
        assert!(!spans[0].style.bold);
        assert!(spans[1].style.bold);
        assert_eq!(&content[spans[1].start..spans[1].end], "Việt");
        assert!(!spans[2].style.bold);

        // Chèn thêm " Xã hội" vào trước "Việt"
        let ins = "Xã hội ";
        content.insert_str(v_start, ins);
        shift_spans_on_insert(&mut spans, v_start, ins.len(), TextStyle::default());

        let new_v_start = content.find("Việt").unwrap();
        let bold_span = spans.iter().find(|s| s.style.bold).unwrap();
        assert_eq!(&content[bold_span.start..bold_span.end], "Việt");
        assert_eq!(bold_span.start, new_v_start);
    }

    #[test]
    fn test_mwd_roundtrip_preserves_vietnamese_and_styles() {
        let text = "Tiêu đề\nNội dung nổi bật";
        let mut spans = vec![StyleSpan {
            start: 0,
            end: text.len(),
            style: TextStyle::default(),
        }];
        let title_end = "Tiêu đề".len();
        apply_style_on_range(&mut spans, text.len(), 0, title_end, |s| {
            s.bold = true;
            s.font_size = 20.0;
            s.highlight = Some([254, 240, 138]);
        });
        let aligns = vec![TextAlignment::Center, TextAlignment::Justify];

        let serialized = serialize_mwd(text, &spans, &aligns);
        let (out_text, out_spans, out_aligns) =
            deserialize_document(&serialized, Path::new("test.mwd"));

        assert_eq!(out_text, text);
        assert_eq!(out_aligns, aligns);
        assert!(out_spans[0].style.bold);
        assert_eq!(out_spans[0].style.font_size, 20.0);
        assert_eq!(out_spans[0].style.highlight, Some([254, 240, 138]));
    }

    #[test]
    fn test_find_matches_vietnamese_case_insensitive() {
        let text = "Việt Nam mến yêu. Người việt tự hào.";
        let matches = find_matches(text, "việt", false);
        assert_eq!(matches.len(), 2);
        assert_eq!(&text[matches[0].0..matches[0].1], "Việt");
        assert_eq!(&text[matches[1].0..matches[1].1], "việt");
    }

    #[test]
    fn test_deserialize_mwd_rejects_non_char_boundary_and_nan_size() {
        // "Tiếng" có 'ế' tại bytes 2..5 -> byte 3 nằm giữa ký tự UTF-8; SZ:NaN phải bị bỏ qua
        let crafted =
            "MYWORD_V1\nALIGN=L\nSPAN=1..3|B:1|SZ:NaN\nSPAN=0..7|B:0|SZ:16.0\n---CONTENT---\nTiếng";
        let (content, spans, _) = deserialize_document(crafted, Path::new("crafted.mwd"));
        assert_eq!(content, "Tiếng");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].start, 0);
        assert_eq!(spans[0].end, "Tiếng".len());
        assert!(!spans[0].style.bold);
        assert_eq!(spans[0].style.font_size, 16.0);

        let nan_crafted = "MYWORD_V1\nALIGN=L\nSPAN=0..7|B:1|SZ:NaN\n---CONTENT---\nTiếng";
        let (_, nan_spans, _) = deserialize_document(nan_crafted, Path::new("nan.mwd"));
        assert!(nan_spans[0].style.font_size.is_finite());
        assert_eq!(nan_spans[0].style.font_size, TextStyle::default().font_size);
    }
}
