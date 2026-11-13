use std::path::Path;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

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
#[derive(Clone, Copy, Debug)]
pub struct TextStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub font_size: f32,
    pub color: [u8; 3],
    pub highlight: Option<[u8; 3]>,
}

impl PartialEq for TextStyle {
    fn eq(&self, other: &Self) -> bool {
        self.bold == other.bold
            && self.italic == other.italic
            && self.underline == other.underline
            && self.strikethrough == other.strikethrough
            && self.color == other.color
            && self.highlight == other.highlight
            && {
                let s_sz = if self.font_size.is_finite() {
                    (self.font_size * 10.0).round() as i32
                } else {
                    0
                };
                let o_sz = if other.font_size.is_finite() {
                    (other.font_size * 10.0).round() as i32
                } else {
                    0
                };
                s_sz == o_sz
            }
    }
}

impl Eq for TextStyle {}

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

/// Ghi đè kiểu dáng lên dải [from..to) trong danh sách spans liên tiếp
fn overwrite_span_range(spans: &mut Vec<StyleSpan>, from: usize, to: usize, style: TextStyle) {
    if from >= to {
        return;
    }
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
            if s.end > to {
                next.push(StyleSpan {
                    start: to,
                    end: s.end,
                    style: s.style,
                });
            }
        }
    }
    next.push(StyleSpan {
        start: from,
        end: to,
        style,
    });
    next.sort_by_key(|s| s.start);
    *spans = next;
}

/// Bảo đảm vùng [0..total_len) được phủ kín bởi các spans liên tiếp,
/// ưu tiên span xuất hiện sau (Last-Write-Wins chuẩn Rich Text) khi có chồng lấn.
pub fn ensure_full_coverage(spans: &mut Vec<StyleSpan>, total_len: usize, fallback: TextStyle) {
    if total_len == 0 {
        spans.clear();
        return;
    }
    // Lọc các span hợp lệ trong phạm vi [0..total_len)
    spans.retain(|s| s.start < s.end && s.start < total_len);
    for s in spans.iter_mut() {
        s.end = s.end.min(total_len);
    }

    let mut coverage = vec![StyleSpan {
        start: 0,
        end: total_len,
        style: fallback,
    }];

    for s in spans.drain(..) {
        overwrite_span_range(&mut coverage, s.start, s.end, s.style);
    }

    normalize_spans(&mut coverage);
    *spans = coverage;
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
    let mut out = Vec::with_capacity(spans.len().clamp(1, 8));
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

/// Tìm tất cả vị trí khớp chuỗi (hỗ trợ cả tiếng Việt tổ hợp NFD, dựng sẵn NFC, và không phân biệt hoa/thường)
/// Chỉ số byte (start, end) trả về luôn luôn ánh xạ chính xác trên `content` gốc, không gây panic khi cắt lát.
pub fn find_matches(content: &str, query: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    if query.is_empty() || content.is_empty() {
        return Vec::new();
    }
    let query_norm: String = query.nfc().collect();
    if query_norm.is_empty() {
        return Vec::new();
    }

    let query_chars: Vec<char> = if case_sensitive {
        query_norm.chars().collect()
    } else {
        query_norm.chars().flat_map(|c| c.to_lowercase()).collect()
    };
    if query_chars.is_empty() {
        return Vec::new();
    }

    // Chuẩn hóa từng grapheme cluster của content sang NFC để tìm kiếm,
    // đồng thời lưu lại ánh xạ byte bắt đầu và byte kết thúc trong content gốc.
    let cap = content.len();
    let mut norm_chars: Vec<char> = Vec::with_capacity(cap);
    let mut orig_starts: Vec<usize> = Vec::with_capacity(cap);
    let mut orig_ends: Vec<usize> = Vec::with_capacity(cap);

    for (g_start, g_str) in content.grapheme_indices(true) {
        let g_end = g_start + g_str.len();
        let nfc_str: String = g_str.nfc().collect();
        for ch in nfc_str.chars() {
            if case_sensitive {
                norm_chars.push(ch);
                orig_starts.push(g_start);
                orig_ends.push(g_end);
            } else {
                for lc in ch.to_lowercase() {
                    norm_chars.push(lc);
                    orig_starts.push(g_start);
                    orig_ends.push(g_end);
                }
            }
        }
    }

    let q_len = query_chars.len();
    if norm_chars.len() < q_len {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut i = 0;
    while i + q_len <= norm_chars.len() {
        if norm_chars[i..i + q_len] == query_chars {
            let start_b = orig_starts[i];
            let end_b = orig_ends[i + q_len - 1];
            if results
                .last()
                .is_none_or(|&(_, last_end)| start_b >= last_end)
            {
                results.push((start_b, end_b));
                i += q_len;
                continue;
            }
        }
        i += 1;
    }

    results
}

/// Chuẩn hóa chuỗi xuống dòng thành `\n` và đồng bộ lại byte offsets cho danh sách spans
fn normalize_newlines_and_spans(content: &str, spans: &[StyleSpan]) -> (String, Vec<StyleSpan>) {
    let mut clean = String::with_capacity(content.len());
    let mut map = Vec::with_capacity(content.len() + 1);
    let mut chars = content.char_indices().peekable();
    let mut cur_old = 0;

    while let Some((old_idx, ch)) = chars.next() {
        while cur_old < old_idx {
            map.push(clean.len());
            cur_old += 1;
        }
        if ch == '\r' {
            if let Some(&(_, next_ch)) = chars.peek()
                && next_ch == '\n'
            {
                chars.next();
                map.push(clean.len());
                cur_old += 1;
                clean.push('\n');
                map.push(clean.len());
                cur_old += 1;
                continue;
            }
            map.push(clean.len());
            cur_old += 1;
            clean.push('\n');
        } else {
            for _ in 0..ch.len_utf8() {
                map.push(clean.len());
                cur_old += 1;
            }
            clean.push(ch);
        }
    }
    while cur_old <= content.len() {
        map.push(clean.len());
        cur_old += 1;
    }

    let mut mapped_spans = Vec::with_capacity(spans.len());
    for s in spans {
        let new_start = *map.get(s.start).unwrap_or(&clean.len());
        let new_end = *map.get(s.end).unwrap_or(&clean.len());
        if new_start < new_end {
            mapped_spans.push(StyleSpan {
                start: new_start,
                end: new_end,
                style: s.style,
            });
        }
    }
    (clean, mapped_spans)
}

/// Tuần tự hóa tài liệu Rich Text sang định dạng `.mwd` (không cần thư viện ngoài)
pub fn serialize_mwd(content: &str, spans: &[StyleSpan], aligns: &[TextAlignment]) -> String {
    // Luôn chuẩn hóa xuống dòng về '\n' và ánh xạ lại spans để ngăn lỗi lệch byte khi gặp CRLF
    let (clean_content, clean_spans) = if content.contains('\r') {
        normalize_newlines_and_spans(content, spans)
    } else {
        (content.to_string(), spans.to_vec())
    };

    let mut out = String::from("MYWORD_V1\n");
    let align_codes: Vec<&str> = aligns.iter().map(|a| a.to_code()).collect();
    out.push_str("ALIGN=");
    out.push_str(&align_codes.join(","));
    out.push('\n');

    for s in &clean_spans {
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
    out.push_str(&clean_content);
    out
}

const MAX_DESERIALIZE_SPANS: usize = 100_000;
const MAX_DESERIALIZE_ALIGNS: usize = 50_000;

/// Giải mã tệp `.mwd` hoặc `.txt`/`.md` thường
pub fn deserialize_document(
    raw: &str,
    path: &Path,
) -> (String, Vec<StyleSpan>, Vec<TextAlignment>) {
    let has_mwd_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("mwd"))
        .unwrap_or(false);

    let has_marker = raw.contains("\n---CONTENT---\n")
        || raw.contains("\r\n---CONTENT---\r\n")
        || raw.contains("\r---CONTENT---\r");

    let starts_with_mwd = raw.starts_with("MYWORD_V1\n")
        || raw.starts_with("MYWORD_V1\r\n")
        || raw.starts_with("MYWORD_V1\r");

    // Chỉ nhận diện là file .mwd nếu có phần mở rộng .mwd HOẶC (bắt đầu bằng MYWORD_V1 VÀ có marker ---CONTENT---)
    let is_mwd = has_mwd_ext || (starts_with_mwd && has_marker);

    if !is_mwd {
        let text: String = raw
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .nfc()
            .collect();
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

    let normalized_raw = raw.replace("\r\n", "\n").replace('\r', "\n");
    let marker = "\n---CONTENT---\n";
    let Some(split_idx) = normalized_raw.find(marker) else {
        let text: String = normalized_raw.nfc().collect();
        return (text, Vec::new(), vec![TextAlignment::Left]);
    };

    let header = &normalized_raw[..split_idx];
    let content: String = normalized_raw[split_idx + marker.len()..].nfc().collect();

    // Kiểm tra xem raw content gốc có chứa \r hay không để sẵn sàng cứu các span bị lưu lệch theo CRLF
    let raw_content_had_cr = if let Some(m_idx) = raw.find("---CONTENT---") {
        let after_marker = &raw[m_idx + "---CONTENT---".len()..];
        let after_nl = if let Some(stripped) = after_marker.strip_prefix("\r\n") {
            stripped
        } else if let Some(stripped) = after_marker
            .strip_prefix('\n')
            .or_else(|| after_marker.strip_prefix('\r'))
        {
            stripped
        } else {
            after_marker
        };
        after_nl.contains('\r')
    } else {
        false
    };

    let para_count = content.split(['\n', PAGE_BREAK_CHAR]).count().max(1);
    let mut aligns = Vec::new();
    let mut spans = Vec::new();

    for line in header.lines() {
        if let Some(rest) = line.strip_prefix("ALIGN=") {
            for code in rest.split(',') {
                if !code.is_empty() && aligns.len() < para_count.min(MAX_DESERIALIZE_ALIGNS) {
                    aligns.push(TextAlignment::from_code(code));
                }
            }
        } else if let Some(rest) = line.strip_prefix("SPAN=")
            && spans.len() < MAX_DESERIALIZE_SPANS
            && let Some(span) = parse_span_line(rest, &content, raw_content_had_cr, raw)
        {
            spans.push(span);
        }
    }

    while aligns.len() < para_count {
        aligns.push(TextAlignment::Left);
    }
    aligns.truncate(para_count);
    ensure_full_coverage(&mut spans, content.len(), TextStyle::default());

    (content, spans, aligns)
}

fn parse_span_line(
    s: &str,
    content: &str,
    raw_content_had_cr: bool,
    raw: &str,
) -> Option<StyleSpan> {
    let mut parts = s.split('|');
    let range_str = parts.next()?;
    let (a, b) = range_str.split_once("..")?;
    let mut start: usize = a.parse().ok()?;
    let mut end: usize = b.parse().ok()?;

    // Nếu start/end không hợp lệ trên content đã chuẩn hóa, nhưng raw content có chứa \r:
    // Thử dịch chuyển bù trừ các ký tự '\r' bị xóa trước vị trí byte đó.
    if (start >= end
        || end > content.len()
        || !content.is_char_boundary(start)
        || !content.is_char_boundary(end))
        && raw_content_had_cr
        && let Some(m_idx) = raw.find("---CONTENT---")
    {
        let after_marker = &raw[m_idx + "---CONTENT---".len()..];
        let raw_content = if let Some(stripped) = after_marker.strip_prefix("\r\n") {
            stripped
        } else if let Some(stripped) = after_marker
            .strip_prefix('\n')
            .or_else(|| after_marker.strip_prefix('\r'))
        {
            stripped
        } else {
            after_marker
        };
        if end <= raw_content.len() {
            let start_cr = raw_content[..start.min(raw_content.len())]
                .chars()
                .filter(|&c| c == '\r')
                .count();
            let end_cr = raw_content[..end.min(raw_content.len())]
                .chars()
                .filter(|&c| c == '\r')
                .count();
            let adj_start = start.saturating_sub(start_cr);
            let adj_end = end.saturating_sub(end_cr);
            if adj_start < adj_end
                && adj_end <= content.len()
                && content.is_char_boundary(adj_start)
                && content.is_char_boundary(adj_end)
            {
                start = adj_start;
                end = adj_end;
            }
        }
    }

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
            if v != "0" && v != "1" {
                return None;
            }
            style.bold = v == "1";
        } else if let Some(v) = part.strip_prefix("I:") {
            if v != "0" && v != "1" {
                return None;
            }
            style.italic = v == "1";
        } else if let Some(v) = part.strip_prefix("U:") {
            if v != "0" && v != "1" {
                return None;
            }
            style.underline = v == "1";
        } else if let Some(v) = part.strip_prefix("S:") {
            if v != "0" && v != "1" {
                return None;
            }
            style.strikethrough = v == "1";
        } else if let Some(v) = part.strip_prefix("SZ:") {
            let sz: f32 = v.parse().ok()?;
            if !sz.is_finite() || sz < 1.0 || sz > 500.0 {
                return None;
            }
            style.font_size = sz.clamp(8.0, 72.0);
        } else if let Some(v) = part.strip_prefix("C:") {
            let rgb_parts: Vec<&str> = v.split(',').collect();
            if rgb_parts.len() != 3 {
                return None;
            }
            let r: u8 = rgb_parts[0].parse().ok()?;
            let g: u8 = rgb_parts[1].parse().ok()?;
            let b: u8 = rgb_parts[2].parse().ok()?;
            style.color = [r, g, b];
        } else if let Some(v) = part.strip_prefix("H:") {
            if v == "none" {
                style.highlight = None;
            } else {
                let rgb_parts: Vec<&str> = v.split(',').collect();
                if rgb_parts.len() != 3 {
                    return None;
                }
                let r: u8 = rgb_parts[0].parse().ok()?;
                let g: u8 = rgb_parts[1].parse().ok()?;
                let b: u8 = rgb_parts[2].parse().ok()?;
                style.highlight = Some([r, g, b]);
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

    #[test]
    fn test_mwd_crlf_roundtrip_preserves_spans_and_aligns() {
        let content_crlf = "Dòng 1: Tiêu đề\r\nDòng 2: Nội dung tiếng Việt\rDòng 3: Kết luận";
        let l1_end = "Dòng 1: Tiêu đề".len();
        let l2_start = content_crlf.find("Dòng 2").unwrap();
        let l2_end = l2_start + "Dòng 2: Nội dung tiếng Việt".len();
        let l3_start = content_crlf.find("Dòng 3").unwrap();
        let l3_end = l3_start + "Dòng 3: Kết luận".len();

        let spans = vec![
            StyleSpan {
                start: 0,
                end: l1_end,
                style: TextStyle {
                    bold: true,
                    ..Default::default()
                },
            },
            StyleSpan {
                start: l2_start,
                end: l2_end,
                style: TextStyle {
                    italic: true,
                    ..Default::default()
                },
            },
            StyleSpan {
                start: l3_start,
                end: l3_end,
                style: TextStyle {
                    underline: true,
                    ..Default::default()
                },
            },
        ];
        let aligns = vec![
            TextAlignment::Center,
            TextAlignment::Left,
            TextAlignment::Right,
        ];

        let serialized = serialize_mwd(content_crlf, &spans, &aligns);
        // Kiểm tra rằng serialized không chứa '\r'
        assert!(!serialized.contains("\r\n") && !serialized.contains('\r'));

        let (out_content, out_spans, out_aligns) =
            deserialize_document(&serialized, Path::new("crlf.mwd"));

        // Content đã được chuẩn hóa về '\n'
        assert!(!out_content.contains('\r'));
        assert_eq!(out_aligns.len(), 3);
        assert_eq!(out_aligns, aligns);

        // Các span vẫn bám đúng theo nội dung
        let bold_span = out_spans.iter().find(|s| s.style.bold).unwrap();
        assert_eq!(
            &out_content[bold_span.start..bold_span.end],
            "Dòng 1: Tiêu đề"
        );

        let italic_span = out_spans.iter().find(|s| s.style.italic).unwrap();
        assert_eq!(
            &out_content[italic_span.start..italic_span.end],
            "Dòng 2: Nội dung tiếng Việt"
        );

        let underline_span = out_spans.iter().find(|s| s.style.underline).unwrap();
        assert_eq!(
            &out_content[underline_span.start..underline_span.end],
            "Dòng 3: Kết luận"
        );
    }

    #[test]
    fn test_find_matches_nfd_vietnamese() {
        // NFD tự sinh từ chuỗi gốc
        let orig = "Học tiếng Việt rất hay. Tiếng Việt giàu đẹp.";
        let nfd_content: String = orig.nfd().collect();
        assert_ne!(nfd_content, orig); // Đảm bảo thực sự là NFD (chứa các combining marks)

        // Query chuẩn NFC: "tiếng"
        let matches = find_matches(&nfd_content, "tiếng", true);
        assert_eq!(matches.len(), 1);
        let (s, e) = matches[0];
        // Cắt lát &nfd_content[s..e] phải không bị panic và khi chuẩn hóa NFC phải đúng là "tiếng"
        let matched_slice = &nfd_content[s..e];
        let matched_nfc: String = matched_slice.nfc().collect();
        assert_eq!(matched_nfc, "tiếng");

        // Không phân biệt hoa/thường: tìm "TIẾNG VIỆT"
        let ci_matches = find_matches(&nfd_content, "TIẾNG VIỆT", false);
        assert_eq!(ci_matches.len(), 2);
        let m0_nfc: String = nfd_content[ci_matches[0].0..ci_matches[0].1]
            .nfc()
            .collect();
        let m1_nfc: String = nfd_content[ci_matches[1].0..ci_matches[1].1]
            .nfc()
            .collect();
        assert_eq!(m0_nfc, "tiếng Việt");
        assert_eq!(m1_nfc, "Tiếng Việt");
    }

    #[test]
    fn test_deserialize_mwd_rejects_corrupted_colors_and_plain_text() {
        // Plain text file vô tình có dòng đầu là MYWORD_V1 nhưng không có marker ---CONTENT---
        let plain_txt = "MYWORD_V1\nĐây là hướng dẫn sử dụng phần mềm.";
        let (text, _, _) = deserialize_document(plain_txt, Path::new("guide.txt"));
        assert_eq!(text, "MYWORD_V1\nĐây là hướng dẫn sử dụng phần mềm.");

        // Màu hỏng: thiếu kênh Blue C:255,0 hoặc tràn u8 C:999,0,0
        let corrupted_color =
            "MYWORD_V1\nALIGN=L\nSPAN=0..4|C:255,0\nSPAN=0..4|C:999,0,0\n---CONTENT---\ntest";
        let (_, spans, _) = deserialize_document(corrupted_color, Path::new("corrupted.mwd"));
        // Cả 2 span hỏng đều bị loại bỏ, fallback về màu mặc định
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].style.color, TextStyle::default().color);
    }

    #[test]
    fn test_ensure_full_coverage_overlapping_spans_last_write_wins() {
        let mut spans = vec![
            StyleSpan {
                start: 0,
                end: 10,
                style: TextStyle {
                    bold: true,
                    ..Default::default()
                },
            },
            StyleSpan {
                start: 5,
                end: 15,
                style: TextStyle {
                    italic: true,
                    ..Default::default()
                },
            },
        ];
        ensure_full_coverage(&mut spans, 20, TextStyle::default());

        // Phải phủ kín 0..20, không chồng lấn, span 5..15 ghi đè 5..10 (last-write-wins)
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].start, 0);
        assert_eq!(spans[0].end, 5);
        assert!(spans[0].style.bold && !spans[0].style.italic);

        assert_eq!(spans[1].start, 5);
        assert_eq!(spans[1].end, 15);
        assert!(!spans[1].style.bold && spans[1].style.italic);

        assert_eq!(spans[2].start, 15);
        assert_eq!(spans[2].end, 20);
        assert!(!spans[2].style.bold && !spans[2].style.italic);
    }

    #[test]
    fn test_text_style_float_precision_equality() {
        let st1 = TextStyle {
            font_size: 14.0,
            ..Default::default()
        };
        let st2 = TextStyle {
            font_size: 14.00001,
            ..Default::default()
        };
        assert_eq!(st1, st2);

        let mut spans = vec![
            StyleSpan {
                start: 0,
                end: 5,
                style: st1,
            },
            StyleSpan {
                start: 5,
                end: 10,
                style: st2,
            },
        ];
        normalize_spans(&mut spans);
        // Hai span kề nhau có font_size 14.0 và 14.00001 phải được gộp thành 1 span 0..10
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].start, 0);
        assert_eq!(spans[0].end, 10);
    }
}
