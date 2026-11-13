use std::fmt::Write as _;
use std::path::Path;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

/// Ký tự ngắt trang thủ công (ASCII Form Feed U+000C)
pub const PAGE_BREAK_CHAR: char = '\x0c';

/// Kiểu căn lề đoạn văn bản
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TextAlignment {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

impl TextAlignment {
    pub fn to_code(self) -> &'static str {
        match self {
            TextAlignment::Left => "L",
            TextAlignment::Center => "C",
            TextAlignment::Right => "R",
            TextAlignment::Justify => "J",
        }
    }

    pub fn from_code(s: &str) -> Self {
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
#[derive(Clone, PartialEq, Eq, Debug)]
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

/// Chuẩn hóa danh sách spans: loại rỗng, sắp xếp, gộp các span kề nhau hoặc chồng lấn cùng style
pub fn normalize_spans(spans: &mut Vec<StyleSpan>) {
    spans.retain(|s| s.start < s.end);
    if spans.is_empty() {
        return;
    }
    spans.sort_by_key(|s| (s.start, s.end));
    let mut merged: Vec<StyleSpan> = Vec::with_capacity(spans.len());
    for span in spans.drain(..) {
        if let Some(last) = merged.last_mut()
            && last.end >= span.start
            && last.style == span.style
        {
            last.end = last.end.max(span.end);
            continue;
        }
        merged.push(span);
    }
    *spans = merged;
}

/// Ghi đè kiểu dáng lên dải [from..to) trong danh sách spans đã được sắp xếp tăng dần.
/// Duy trì thứ tự sắp xếp mà không cần gọi `sort_by_key`.
fn overwrite_span_range(spans: &mut Vec<StyleSpan>, from: usize, to: usize, style: TextStyle) {
    if from >= to {
        return;
    }
    let mut next = Vec::with_capacity(spans.len() + 2);
    let mut inserted = false;

    for s in spans.drain(..) {
        if s.end <= from {
            next.push(s);
        } else if s.start >= to {
            if !inserted {
                next.push(StyleSpan {
                    start: from,
                    end: to,
                    style,
                });
                inserted = true;
            }
            next.push(s);
        } else {
            if s.start < from {
                next.push(StyleSpan {
                    start: s.start,
                    end: from,
                    style: s.style,
                });
            }
            if !inserted {
                next.push(StyleSpan {
                    start: from,
                    end: to,
                    style,
                });
                inserted = true;
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

    if !inserted {
        next.push(StyleSpan {
            start: from,
            end: to,
            style,
        });
    }

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

    // Fast-path O(N): Kiểm tra nếu các span đã tăng dần và không chồng lấn nhau
    let is_disjoint_sorted = spans.windows(2).all(|w| w[0].end <= w[1].start);

    if is_disjoint_sorted {
        let mut coverage = Vec::with_capacity(spans.len() + 2);
        let mut cursor = 0;
        for s in spans.drain(..) {
            if s.start > cursor {
                coverage.push(StyleSpan {
                    start: cursor,
                    end: s.start,
                    style: fallback,
                });
            }
            cursor = s.end;
            coverage.push(s);
        }
        if cursor < total_len {
            coverage.push(StyleSpan {
                start: cursor,
                end: total_len,
                style: fallback,
            });
        }
        normalize_spans(&mut coverage);
        *spans = coverage;
        return;
    }

    // Slow-path cho trường hợp các span bị chồng lấn (Last-Write-Wins)
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

/// Truy vấn nhanh TextStyle tại một vị trí byte cụ thể (Binary Search O(log N))
pub fn style_at_offset(spans: &[StyleSpan], byte_offset: usize) -> TextStyle {
    if spans.is_empty() {
        return TextStyle::default();
    }
    // Khi gõ chữ ở vị trí biên (caret > 0), ưu tiên lấy style của ký tự ngay trước con trỏ
    let idx = spans.partition_point(|s| s.end <= byte_offset.saturating_sub(1));
    spans
        .get(idx)
        .or_else(|| spans.last())
        .map(|s| s.style)
        .unwrap_or_default()
}

/// Cập nhật spans khi chèn đoạn dài `len` bytes tại `pos`
pub fn shift_spans_on_insert(spans: &mut Vec<StyleSpan>, pos: usize, len: usize, style: TextStyle) {
    if len == 0 {
        return;
    }
    let mut next = Vec::with_capacity(spans.len() + 2);
    let mut inserted = false;

    for s in spans.drain(..) {
        if s.end <= pos {
            next.push(s);
        } else if s.start >= pos {
            if !inserted {
                next.push(StyleSpan {
                    start: pos,
                    end: pos + len,
                    style,
                });
                inserted = true;
            }
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
                start: pos,
                end: pos + len,
                style,
            });
            inserted = true;
            next.push(StyleSpan {
                start: pos + len,
                end: s.end + len,
                style: s.style,
            });
        }
    }

    if !inserted {
        next.push(StyleSpan {
            start: pos,
            end: pos + len,
            style,
        });
    }

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

/// Lấy danh sách các span cắt đúng trong phạm vi đoạn `[para_start..para_end)` (quy đổi sang local offset).
/// Sử dụng Binary Search `partition_point` để đạt tốc độ O(log N + K) với K là số span trong đoạn.
pub fn spans_for_slice(
    spans: &[StyleSpan],
    para_start: usize,
    para_end: usize,
) -> Vec<(usize, usize, TextStyle)> {
    let para_len = para_end.saturating_sub(para_start);
    if para_len == 0 {
        return Vec::new();
    }

    let start_idx = spans.partition_point(|s| s.end <= para_start);
    let mut out = Vec::with_capacity(4);
    let mut cursor = 0;

    for s in &spans[start_idx..] {
        if s.start >= para_end {
            break;
        }
        if s.end <= para_start {
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

    let query_chars: Vec<char> = if case_sensitive {
        query.nfc().collect()
    } else {
        query.nfc().flat_map(|c| c.to_lowercase()).collect()
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
        for ch in g_str.nfc() {
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

/// Chuẩn hóa chuỗi đầu vào: chuyển `\r\n` và `\r` thành `\n`, đồng thời chuẩn hóa NFC cho từng grapheme.
/// Trả về `(chuỗi_đã_chuẩn_hóa, bảng_ánh_xạ_byte_cũ_sang_byte_mới)` có độ dài `raw.len() + 1`.
fn normalize_text_and_build_byte_map(raw: &str) -> (String, Vec<usize>) {
    let mut clean = String::with_capacity(raw.len());
    let mut byte_map = vec![0usize; raw.len() + 1];

    for (g_start, g_str) in raw.grapheme_indices(true) {
        let new_start = clean.len();
        if g_str == "\r\n" || g_str == "\r" {
            clean.push('\n');
        } else {
            clean.extend(g_str.nfc());
        }
        let new_end = clean.len();
        let g_len = g_str.len();

        // Mọi byte bên trong grapheme cũ đều ánh xạ về đầu grapheme mới
        byte_map[g_start..(g_start + g_len)].fill(new_start);
        byte_map[g_start + g_len] = new_end;
    }

    (clean, byte_map)
}

/// Chuẩn hóa chuỗi xuống dòng thành `\n` và đồng bộ lại byte offsets cho danh sách spans
fn normalize_newlines_and_spans(content: &str, spans: &[StyleSpan]) -> (String, Vec<StyleSpan>) {
    let (clean, map) = normalize_text_and_build_byte_map(content);
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
    let (clean_content, clean_spans) = normalize_newlines_and_spans(content, spans);

    let mut out = String::with_capacity(clean_content.len() + clean_spans.len() * 48 + 128);
    out.push_str("MYWORD_V1\nALIGN=");
    for (idx, a) in aligns.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        out.push_str(a.to_code());
    }
    out.push('\n');

    for s in &clean_spans {
        let _ = write!(
            out,
            "SPAN={}..{}|B:{}|I:{}|U:{}|S:{}|SZ:{:.1}|C:{},{},{}|H:",
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
        );
        match s.style.highlight {
            Some([r, g, b]) => {
                let _ = writeln!(out, "{},{},{}", r, g, b);
            }
            None => {
                out.push_str("none\n");
            }
        }
    }
    out.push_str("---CONTENT---\n");
    out.push_str(&clean_content);
    out
}

/// Xuất tài liệu sang định dạng Microsoft Word HTML Document (.doc)
/// Tương thích 100% với Microsoft Word, WPS Office, LibreOffice và Google Docs.
pub fn export_word_doc(content: &str, spans: &[StyleSpan], aligns: &[TextAlignment]) -> String {
    let mwd_payload = serialize_mwd(content, spans, aligns);

    let mut html = String::with_capacity(content.len() * 2 + 1024);
    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html xmlns:o='urn:schemas-microsoft-com:office:office' xmlns:w='urn:schemas-microsoft-com:office:word' xmlns='http://www.w3.org/TR/REC-html40'>\n");
    html.push_str("<head>\n<meta charset='utf-8'>\n<title>Tài liệu</title>\n");

    // Nhúng khối dữ liệu gốc MyWord để đảm bảo 100% lossless khi mở lại trong MyWord
    html.push_str("<!-- MYWORD_DATA:\n");
    html.push_str(&mwd_payload);
    html.push_str("\n-->\n");

    // Khai báo Word Document Section & Page Setup (Khổ A4, margin 1in, Print view)
    html.push_str("<!--[if gte mso 9]>\n<xml>\n<w:WordDocument>\n<w:View>Print</w:View>\n<w:Zoom>100</w:Zoom>\n<w:DoNotOptimizeForBrowser/>\n</w:WordDocument>\n</xml>\n<![endif]-->\n");
    html.push_str("<style>\n@page {\n  size: 210mm 297mm;\n  margin: 1in;\n  mso-page-orientation: portrait;\n}\nbody {\n  font-family: 'Segoe UI', Arial, sans-serif;\n  font-size: 14pt;\n  color: #1e293b;\n  line-height: 1.5;\n}\np {\n  margin: 0 0 6pt 0;\n}\n.page-break {\n  page-break-before: always;\n  mso-break-type: section-break;\n}\n</style>\n</head>\n<body>\n");

    let total_len = content.len();
    let mut full_spans = spans.to_vec();
    ensure_full_coverage(&mut full_spans, total_len, TextStyle::default());

    let mut i = 0;
    let mut align_idx = 0;

    while i < total_len {
        let (rel_end, is_page_break, delimiter_len) =
            if let Some(rel) = content[i..].find(['\n', PAGE_BREAK_CHAR]) {
                let ch = content[i + rel..].chars().next().unwrap();
                (rel, ch == PAGE_BREAK_CHAR, ch.len_utf8())
            } else {
                (content[i..].len(), false, 0)
            };
        let p_end = i + rel_end;

        let align = aligns
            .get(align_idx)
            .copied()
            .unwrap_or(TextAlignment::Left);
        let align_css = match align {
            TextAlignment::Left => "text-align: left;",
            TextAlignment::Center => "text-align: center;",
            TextAlignment::Right => "text-align: right;",
            TextAlignment::Justify => "text-align: justify;",
        };

        if is_page_break {
            html.push_str("<div class='page-break'></div>\n");
        }

        html.push_str("<p style='");
        html.push_str(align_css);
        html.push_str("'>");

        if i == p_end {
            html.push_str("&nbsp;");
        } else {
            let p_spans = spans_for_slice(&full_spans, i, p_end);
            for (rel_start, rel_end, style) in p_spans {
                let chunk = &content[i + rel_start..i + rel_end];
                let escaped = escape_html(chunk);

                let mut open_tags = String::new();
                let mut close_tags = String::new();

                let has_custom_style = (style.font_size - 14.0).abs() > 0.1
                    || style.color != [0, 0, 0]
                    || style.highlight.is_some();

                if has_custom_style {
                    let mut inline_css = String::new();
                    if (style.font_size - 14.0).abs() > 0.1 {
                        let _ = write!(inline_css, "font-size: {:.1}pt; ", style.font_size);
                    }
                    if style.color != [0, 0, 0] {
                        let _ = write!(
                            inline_css,
                            "color: rgb({},{},{}); ",
                            style.color[0], style.color[1], style.color[2]
                        );
                    }
                    if let Some([hr, hg, hb]) = style.highlight {
                        let _ = write!(inline_css, "background-color: rgb({},{},{}); ", hr, hg, hb);
                    }
                    open_tags.push_str(&format!("<span style='{}'>", inline_css.trim()));
                    close_tags.insert_str(0, "</span>");
                }

                if style.bold {
                    open_tags.push_str("<b>");
                    close_tags.insert_str(0, "</b>");
                }
                if style.italic {
                    open_tags.push_str("<i>");
                    close_tags.insert_str(0, "</i>");
                }
                if style.underline {
                    open_tags.push_str("<u>");
                    close_tags.insert_str(0, "</u>");
                }
                if style.strikethrough {
                    open_tags.push_str("<s>");
                    close_tags.insert_str(0, "</s>");
                }

                html.push_str(&open_tags);
                html.push_str(&escaped);
                html.push_str(&close_tags);
            }
        }

        html.push_str("</p>\n");

        i = p_end + delimiter_len;
        align_idx += 1;
    }

    html.push_str("</body>\n</html>\n");
    html
}

fn escape_html(s: &str) -> String {
    let mut res = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => res.push_str("&amp;"),
            '<' => res.push_str("&lt;"),
            '>' => res.push_str("&gt;"),
            '"' => res.push_str("&quot;"),
            '\'' => res.push_str("&#39;"),
            _ => res.push(c),
        }
    }
    res
}

fn strip_html_tags(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_script_or_style = false;
    let mut tag_name = String::new();

    let chars: Vec<char> = html.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '<' {
            in_tag = true;
            tag_name.clear();
            i += 1;
            while i < chars.len() && chars[i] != '>' && !chars[i].is_whitespace() {
                tag_name.push(chars[i].to_ascii_lowercase());
                i += 1;
            }
            if tag_name == "style" || tag_name == "script" {
                in_script_or_style = true;
            } else if tag_name == "/style" || tag_name == "/script" {
                in_script_or_style = false;
            } else if matches!(tag_name.as_str(), "p" | "br" | "div" | "/p" | "tr")
                && !text.ends_with('\n')
                && !text.is_empty()
            {
                text.push('\n');
            }
            while i < chars.len() && chars[i] != '>' {
                i += 1;
            }
            if i < chars.len() && chars[i] == '>' {
                in_tag = false;
                i += 1;
            }
            continue;
        }

        if !in_tag && !in_script_or_style {
            text.push(c);
        }
        i += 1;
    }

    let decoded = text
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    decoded.trim().to_string()
}

const MAX_DESERIALIZE_SPANS: usize = 100_000;
const MAX_DESERIALIZE_ALIGNS: usize = 50_000;

/// Tách phần Header và phần Content gốc từ chuỗi raw của file `.mwd`
fn split_mwd_header_and_content(raw: &str) -> Option<(&str, &str)> {
    let mut search_from = 0;
    const MARKER: &str = "---CONTENT---";

    while let Some(rel_pos) = raw[search_from..].find(MARKER) {
        let pos = search_from + rel_pos;
        let valid_before = pos == 0 || matches!(raw.as_bytes()[pos - 1], b'\n' | b'\r');
        let after_idx = pos + MARKER.len();

        if valid_before {
            let after_str = &raw[after_idx..];
            let content_raw = after_str
                .strip_prefix("\r\n")
                .or_else(|| after_str.strip_prefix('\n'))
                .or_else(|| after_str.strip_prefix('\r'));

            if let Some(content) = content_raw {
                let mut header_end = pos;
                if header_end > 0 && raw.as_bytes()[header_end - 1] == b'\n' {
                    header_end -= 1;
                }
                if header_end > 0 && raw.as_bytes()[header_end - 1] == b'\r' {
                    header_end -= 1;
                }
                return Some((&raw[..header_end], content));
            }
        }
        search_from = after_idx;
    }
    None
}

/// Giải mã tệp `.mwd` hoặc `.txt`/`.md` thường
pub fn deserialize_document(
    raw: &str,
    path: &Path,
) -> (String, Vec<StyleSpan>, Vec<TextAlignment>) {
    // 0. Nếu tệp chứa khối dữ liệu gốc <!-- MYWORD_DATA:\n ... \n--> (từ tệp .doc do MyWord xuất ra)
    const MWD_DATA_START: &str = "<!-- MYWORD_DATA:\n";
    const MWD_DATA_END: &str = "\n-->";
    if let Some(start_idx) = raw.find(MWD_DATA_START) {
        let after_start = start_idx + MWD_DATA_START.len();
        if let Some(end_idx) = raw[after_start..].find(MWD_DATA_END) {
            let embedded = &raw[after_start..after_start + end_idx];
            return deserialize_document(embedded, Path::new("embedded.mwd"));
        }
    }

    // 0.1. Nếu là tệp HTML / Word document (.doc/.htm/.html) không có metadata nhúng
    let is_html_or_doc = path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
        e.eq_ignore_ascii_case("doc")
            || e.eq_ignore_ascii_case("docx")
            || e.eq_ignore_ascii_case("html")
            || e.eq_ignore_ascii_case("htm")
    }) || raw.trim_start().starts_with("<!DOCTYPE")
        || raw.trim_start().starts_with("<html");

    if is_html_or_doc {
        let plain = strip_html_tags(raw);
        let (text, _) = normalize_text_and_build_byte_map(&plain);
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

    let has_mwd_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("mwd"))
        .unwrap_or(false);

    let starts_with_mwd = raw.starts_with("MYWORD_V1\n")
        || raw.starts_with("MYWORD_V1\r\n")
        || raw.starts_with("MYWORD_V1\r");

    let split_parts = split_mwd_header_and_content(raw);

    // Chỉ nhận diện là file .mwd nếu có phần mở rộng .mwd HOẶC (bắt đầu bằng MYWORD_V1 VÀ có marker ---CONTENT---)
    let is_mwd = has_mwd_ext || (starts_with_mwd && split_parts.is_some());

    if !is_mwd {
        let (text, _) = normalize_text_and_build_byte_map(raw);
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

    let Some((header_raw, content_raw)) = split_parts else {
        let (text, _) = normalize_text_and_build_byte_map(raw);
        let para_count = text.split(['\n', PAGE_BREAK_CHAR]).count().max(1);
        return (text, Vec::new(), vec![TextAlignment::Left; para_count]);
    };

    // Chuẩn hóa content_raw (cả CRLF lẫn NFC) và lấy bảng ánh xạ byte O(1)
    let (content, byte_map) = normalize_text_and_build_byte_map(content_raw);
    let content_was_modified = content.len() != content_raw.len() || content != content_raw;

    let para_count = content.split(['\n', PAGE_BREAK_CHAR]).count().max(1);
    let mut aligns = Vec::new();
    let mut spans = Vec::new();

    for line in header_raw.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix("ALIGN=") {
            for code in rest.split(',') {
                if !code.is_empty() && aligns.len() < para_count.min(MAX_DESERIALIZE_ALIGNS) {
                    aligns.push(TextAlignment::from_code(code));
                }
            }
        } else if let Some(rest) = line.strip_prefix("SPAN=")
            && spans.len() < MAX_DESERIALIZE_SPANS
            && let Some(span) = parse_span_line(rest, &content, content_was_modified, &byte_map)
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
    content_was_modified: bool,
    byte_map: &[usize],
) -> Option<StyleSpan> {
    let mut parts = s.split('|');
    let range_str = parts.next()?;
    let (a, b) = range_str.split_once("..")?;
    let mut start: usize = a.parse().ok()?;
    let mut end: usize = b.parse().ok()?;

    // Nếu content_raw ban đầu có chứa \r hoặc ký tự tổ hợp NFD, dùng byte_map để quy đổi chính xác sang vị trí mới
    if content_was_modified && end < byte_map.len() && start < end {
        let mapped_start = byte_map[start];
        let mapped_end = byte_map[end];
        if mapped_start < mapped_end
            && mapped_end <= content.len()
            && content.is_char_boundary(mapped_start)
            && content.is_char_boundary(mapped_end)
        {
            start = mapped_start;
            end = mapped_end;
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
            if !sz.is_finite() || !(1.0..=500.0).contains(&sz) {
                return None;
            }
            style.font_size = sz.clamp(8.0, 72.0);
        } else if let Some(v) = part.strip_prefix("C:") {
            style.color = parse_rgb(v)?;
        } else if let Some(v) = part.strip_prefix("H:") {
            if v == "none" {
                style.highlight = None;
            } else {
                style.highlight = Some(parse_rgb(v)?);
            }
        }
    }

    Some(StyleSpan { start, end, style })
}

fn parse_rgb(v: &str) -> Option<[u8; 3]> {
    let mut it = v.split(',');
    let r: u8 = it.next()?.parse().ok()?;
    let g: u8 = it.next()?.parse().ok()?;
    let b: u8 = it.next()?.parse().ok()?;
    if it.next().is_some() {
        return None;
    }
    Some([r, g, b])
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

        // Chèn thêm "Xã hội " vào trước "Việt"
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
    fn test_mwd_crlf_and_nfd_roundtrip_preserves_spans_and_aligns() {
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
        assert!(!serialized.contains("\r\n") && !serialized.contains('\r'));

        let (out_content, out_spans, out_aligns) =
            deserialize_document(&serialized, Path::new("crlf.mwd"));

        assert!(!out_content.contains('\r'));
        assert_eq!(out_aligns.len(), 3);
        assert_eq!(out_aligns, aligns);

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
    fn test_deserialize_raw_nfd_mwd_maps_spans_accurately() {
        // Giả lập một file .mwd có nội dung bị lưu ở dạng tổ hợp NFD
        let raw_nfd: String = "Tiếng Việt".nfd().collect();
        let tieng_nfd: String = "Tiếng".nfd().collect();
        let raw_file = format!(
            "MYWORD_V1\nALIGN=L\nSPAN=0..{}|B:1\n---CONTENT---\n{}",
            tieng_nfd.len(),
            raw_nfd
        );

        let (content, spans, _) = deserialize_document(&raw_file, Path::new("nfd.mwd"));
        assert_eq!(content, "Tiếng Việt");
        let bold_span = spans.iter().find(|s| s.style.bold).unwrap();
        assert_eq!(&content[bold_span.start..bold_span.end], "Tiếng");
    }

    #[test]
    fn test_find_matches_nfd_vietnamese() {
        let orig = "Học tiếng Việt rất hay. Tiếng Việt giàu đẹp.";
        let nfd_content: String = orig.nfd().collect();
        assert_ne!(nfd_content, orig);

        let matches = find_matches(&nfd_content, "tiếng", true);
        assert_eq!(matches.len(), 1);
        let (s, e) = matches[0];
        let matched_slice = &nfd_content[s..e];
        let matched_nfc: String = matched_slice.nfc().collect();
        assert_eq!(matched_nfc, "tiếng");

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
        let plain_txt = "MYWORD_V1\nĐây là hướng dẫn sử dụng phần mềm.";
        let (text, _, _) = deserialize_document(plain_txt, Path::new("guide.txt"));
        assert_eq!(text, "MYWORD_V1\nĐây là hướng dẫn sử dụng phần mềm.");

        let corrupted_color =
            "MYWORD_V1\nALIGN=L\nSPAN=0..4|C:255,0\nSPAN=0..4|C:999,0,0\n---CONTENT---\ntest";
        let (_, spans, _) = deserialize_document(corrupted_color, Path::new("corrupted.mwd"));
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

    #[test]
    fn test_export_word_doc_roundtrip_and_html_fallback() {
        let content = "Tiêu đề tài liệu\nĐoạn văn có chữ đậm và nghiêng.";
        let mut spans = vec![
            StyleSpan {
                start: 0,
                end: "Tiêu đề tài liệu".len(),
                style: TextStyle {
                    bold: true,
                    font_size: 20.0,
                    ..Default::default()
                },
            },
            StyleSpan {
                start: "Tiêu đề tài liệu\nĐoạn văn có ".len(),
                end: "Tiêu đề tài liệu\nĐoạn văn có chữ đậm".len(),
                style: TextStyle {
                    bold: true,
                    ..Default::default()
                },
            },
        ];
        ensure_full_coverage(&mut spans, content.len(), TextStyle::default());
        let aligns = vec![TextAlignment::Center, TextAlignment::Justify];

        let html_doc = export_word_doc(content, &spans, &aligns);

        // 1. Phải chứa các thẻ HTML đặc trưng của Word Document
        assert!(html_doc.contains("<w:WordDocument>"));
        assert!(html_doc.contains("<!-- MYWORD_DATA:"));
        assert!(html_doc.contains("text-align: center;"));
        assert!(html_doc.contains("text-align: justify;"));
        assert!(html_doc.contains("<b>"));

        // 2. Roundtrip qua deserialize_document khôi phục đúng 100%
        let (out_content, out_spans, out_aligns) =
            deserialize_document(&html_doc, Path::new("test.doc"));
        assert_eq!(out_content, content);
        assert_eq!(out_aligns, aligns);
        assert_eq!(out_spans, spans);

        // 3. Mở tệp HTML thông thường không có MYWORD_DATA metadata
        let external_html = "<html><body><p>Dòng 1</p><p>Dòng 2 &amp; bạn bè</p></body></html>";
        let (ext_content, ext_spans, ext_aligns) =
            deserialize_document(external_html, Path::new("external.doc"));
        assert!(ext_content.contains("Dòng 1"));
        assert!(ext_content.contains("Dòng 2 & bạn bè"));
        assert_eq!(ext_aligns.len(), 2);
        assert!(!ext_spans.is_empty());
    }
}
