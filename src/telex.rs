/// Bộ gõ tiếng Việt Telex tích hợp (tùy chọn bên cạnh Unikey/Windows IME)
/// Hỗ trợ nguyên âm mũ/móc (â, ă, ê, ô, ơ, ư, ươ), chữ đ, và 5 thanh điệu (s, f, r, x, j) + xóa dấu (z).

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    None = 0,
    Sac = 1,   // s
    Huyen = 2, // f
    Hoi = 3,   // r
    Nga = 4,   // x
    Nang = 5,  // j
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct VowelInfo {
    base: char, // lowercase base: 'a','â','ă','e','ê','i','o','ô','ơ','u','ư','y'
    tone: Tone,
    upper: bool,
}

const VOWEL_TABLE: &[(char, [char; 6])] = &[
    ('a', ['a', 'á', 'à', 'ả', 'ã', 'ạ']),
    ('â', ['â', 'ấ', 'ầ', 'ẩ', 'ẫ', 'ậ']),
    ('ă', ['ă', 'ắ', 'ằ', 'ẳ', 'ẵ', 'ặ']),
    ('e', ['e', 'é', 'è', 'ẻ', 'ẽ', 'ẹ']),
    ('ê', ['ê', 'ế', 'ề', 'ể', 'ễ', 'ệ']),
    ('i', ['i', 'í', 'ì', 'ỉ', 'ĩ', 'ị']),
    ('o', ['o', 'ó', 'ò', 'ỏ', 'õ', 'ọ']),
    ('ô', ['ô', 'ố', 'ồ', 'ổ', 'ỗ', 'ộ']),
    ('ơ', ['ơ', 'ớ', 'ờ', 'ở', 'ỡ', 'ợ']),
    ('u', ['u', 'ú', 'ù', 'ủ', 'ũ', 'ụ']),
    ('ư', ['ư', 'ứ', 'ừ', 'ử', 'ữ', 'ự']),
    ('y', ['y', 'ý', 'ỳ', 'ỷ', 'ỹ', 'ỵ']),
];

fn parse_vowel(ch: char) -> Option<VowelInfo> {
    let upper = ch.is_uppercase();
    let lower = ch.to_lowercase().next()?;
    for &(base, variants) in VOWEL_TABLE {
        for (idx, &v) in variants.iter().enumerate() {
            if v == lower {
                let tone = match idx {
                    0 => Tone::None,
                    1 => Tone::Sac,
                    2 => Tone::Huyen,
                    3 => Tone::Hoi,
                    4 => Tone::Nga,
                    5 => Tone::Nang,
                    _ => Tone::None,
                };
                return Some(VowelInfo { base, tone, upper });
            }
        }
    }
    None
}

fn compose_vowel(base: char, tone: Tone, upper: bool) -> Option<char> {
    for &(b, variants) in VOWEL_TABLE {
        if b == base {
            let c = variants[tone as usize];
            return Some(if upper {
                c.to_uppercase().next().unwrap_or(c)
            } else {
                c
            });
        }
    }
    None
}

fn is_modified_vowel(base: char) -> bool {
    matches!(base, 'â' | 'ă' | 'ê' | 'ô' | 'ơ' | 'ư')
}

/// Thử xử lý 1 ký tự ASCII nhập vào ở vị trí cuối từ hiện tại `word`.
/// Trả về `Some(new_word)` nếu có biến đổi Telex, hoặc `None` nếu giữ nguyên ký tự gốc.
pub fn try_apply_telex(word: &str, key: char) -> Option<String> {
    if word.is_empty() || !key.is_ascii_alphabetic() {
        return None;
    }

    let lower_key = key.to_ascii_lowercase();
    let mut chars: Vec<char> = word.chars().collect();

    // 1. Xử lý 'd' -> 'đ' (hoặc 'đ' + 'd' -> hoàn tác về 'dd')
    if lower_key == 'd' {
        for i in 0..chars.len() {
            let c = chars[i];
            if c == 'd' || c == 'D' {
                chars[i] = if c == 'D' { 'Đ' } else { 'đ' };
                return Some(chars.into_iter().collect());
            }
        }
        // Nếu không có 'd' thường nào trong từ, hoàn tác 'đ'/'Đ' về 'd'/'D' và thêm phím 'd'
        for i in (0..chars.len()).rev() {
            let c = chars[i];
            if c == 'đ' || c == 'Đ' {
                chars[i] = if c == 'Đ' { 'D' } else { 'd' };
                chars.push(key);
                return Some(chars.into_iter().collect());
            }
        }
        return None;
    }

    // 2. Xử lý nguyên âm mũ ('a', 'e', 'o')
    if matches!(lower_key, 'a' | 'e' | 'o') {
        let target_from = lower_key;
        let target_to = match lower_key {
            'a' => 'â',
            'e' => 'ê',
            'o' => 'ô',
            _ => unreachable!(),
        };
        if chars.len() >= 2
            && chars[chars.len() - 1].to_ascii_lowercase() == target_from
            && chars[chars.len() - 2].to_ascii_lowercase() == target_from
        {
            return None;
        }
        for i in (0..chars.len()).rev() {
            if let Some(vi) = parse_vowel(chars[i]) {
                if vi.base == target_from {
                    chars[i] = compose_vowel(target_to, vi.tone, vi.upper)?;
                    return Some(chars.into_iter().collect());
                } else if vi.base == target_to && i == chars.len() - 1 {
                    chars[i] = compose_vowel(target_from, vi.tone, vi.upper)?;
                    chars.push(key);
                    return Some(chars.into_iter().collect());
                }
            }
        }
        return None;
    }

    // 3. Xử lý 'w' (ă, ơ, ư, ươ)
    if lower_key == 'w' {
        // Kiểm tra hoàn tác cặp 'ươ' liền kề thành 'uo' + 'w'
        for i in 0..chars.len().saturating_sub(1) {
            if let (Some(v1), Some(v2)) = (parse_vowel(chars[i]), parse_vowel(chars[i + 1]))
                && v1.base == 'ư'
                && v2.base == 'ơ'
            {
                chars[i] = compose_vowel('u', v1.tone, v1.upper)?;
                chars[i + 1] = compose_vowel('o', v2.tone, v2.upper)?;
                chars.push(key);
                return Some(chars.into_iter().collect());
            }
        }

        // Biến đổi 'uo' thành 'ươ'
        for i in 0..chars.len().saturating_sub(1) {
            if let (Some(v1), Some(v2)) = (parse_vowel(chars[i]), parse_vowel(chars[i + 1]))
                && v1.base == 'u'
                && v2.base == 'o'
            {
                chars[i] = compose_vowel('ư', v1.tone, v1.upper)?;
                chars[i + 1] = compose_vowel('ơ', v2.tone, v2.upper)?;
                return Some(chars.into_iter().collect());
            }
        }
        for i in (0..chars.len()).rev() {
            if let Some(vi) = parse_vowel(chars[i]) {
                let mapped = match vi.base {
                    'a' => Some('ă'),
                    'o' => Some('ơ'),
                    'u' => Some('ư'),
                    _ => None,
                };
                if let Some(new_base) = mapped {
                    chars[i] = compose_vowel(new_base, vi.tone, vi.upper)?;
                    return Some(chars.into_iter().collect());
                }
                if matches!(vi.base, 'ă' | 'ơ' | 'ư') && i == chars.len() - 1 {
                    let revert = match vi.base {
                        'ă' => 'a',
                        'ơ' => 'o',
                        'ư' => 'u',
                        _ => unreachable!(),
                    };
                    chars[i] = compose_vowel(revert, vi.tone, vi.upper)?;
                    chars.push(key);
                    return Some(chars.into_iter().collect());
                }
            }
        }
        if chars.iter().all(|c| parse_vowel(*c).is_none()) {
            chars.push(if key.is_uppercase() { 'Ư' } else { 'ư' });
            return Some(chars.into_iter().collect());
        }
        return None;
    }

    // 4. Xử lý 5 thanh điệu (s, f, r, x, j) và xóa dấu (z)
    let requested_tone = match lower_key {
        's' => Some(Tone::Sac),
        'f' => Some(Tone::Huyen),
        'r' => Some(Tone::Hoi),
        'x' => Some(Tone::Nga),
        'j' => Some(Tone::Nang),
        'z' => Some(Tone::None),
        _ => None,
    };

    if let Some(new_tone) = requested_tone {
        // Thu thập vị trí tất cả nguyên âm trong âm tiết
        let mut vowel_positions: Vec<(usize, VowelInfo)> = Vec::new();
        for (i, &c) in chars.iter().enumerate() {
            if let Some(vi) = parse_vowel(c) {
                vowel_positions.push((i, vi));
            }
        }

        if vowel_positions.is_empty() {
            return None;
        }

        // Bỏ qua 'u' trong 'qu...' và 'i' trong 'gi...' nếu phía sau còn nguyên âm khác
        if vowel_positions.len() >= 2 {
            let first_idx = vowel_positions[0].0;
            let first_base = vowel_positions[0].1.base;
            if first_idx > 0 {
                let prev_lower = chars[first_idx - 1].to_lowercase().next().unwrap_or(' ');
                if (prev_lower == 'q' && first_base == 'u')
                    || (prev_lower == 'g' && first_base == 'i')
                {
                    vowel_positions.remove(0);
                }
            }
        }

        // Chọn vị trí nguyên âm đặt dấu
        let target_idx = if let Some(&(pos, _)) = vowel_positions
            .iter()
            .rfind(|&&(_, v)| is_modified_vowel(v.base))
        {
            // Ưu tiên nguyên âm có mũ/móc (vd: ươ -> ưu tiên ơ nhờ rfind)
            pos
        } else if vowel_positions.len() >= 3 {
            vowel_positions[1].0
        } else if vowel_positions.len() == 2 {
            let last_vowel_pos = vowel_positions[1].0;
            let has_trailing_consonant = last_vowel_pos < chars.len() - 1;
            if has_trailing_consonant {
                vowel_positions[1].0
            } else {
                vowel_positions[0].0
            }
        } else {
            vowel_positions[0].0
        };

        let existing_tone = chars
            .iter()
            .filter_map(|&c| parse_vowel(c).map(|v| v.tone))
            .find(|&t| t != Tone::None)
            .unwrap_or(Tone::None);

        // Nếu gõ z khi chưa có dấu -> gõ chữ z bình thường
        if new_tone == Tone::None && existing_tone == Tone::None {
            return None;
        }

        // Nếu từ đã kết thúc bằng chính phím thanh điệu này (đã escape trước đó, vd: "bos" + "s" -> "boss")
        if existing_tone == Tone::None
            && chars.last().map(|c| c.to_ascii_lowercase()) == Some(lower_key)
        {
            return None;
        }

        // Nếu gõ lặp lại cùng thanh điệu (vd: "tó" + "s" -> "tos") -> hoàn tác dấu và giữ ký tự
        if new_tone != Tone::None && existing_tone == new_tone {
            for c in chars.iter_mut() {
                if let Some(vi) = parse_vowel(*c) {
                    *c = compose_vowel(vi.base, Tone::None, vi.upper)?;
                }
            }
            chars.push(key);
            return Some(chars.into_iter().collect());
        }

        // Xóa dấu cũ trên các nguyên âm khác rồi đặt dấu mới lên target_idx
        for (i, c) in chars.iter_mut().enumerate() {
            if let Some(vi) = parse_vowel(*c) {
                let t = if i == target_idx {
                    new_tone
                } else {
                    Tone::None
                };
                *c = compose_vowel(vi.base, t, vi.upper)?;
            }
        }
        return Some(chars.into_iter().collect());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn type_telex(seq: &str) -> String {
        let mut buf = String::new();
        for ch in seq.chars() {
            if ch.is_ascii_alphabetic() {
                // Lấy từ đang gõ ở cuối buf
                let word_start = buf
                    .char_indices()
                    .rev()
                    .take_while(|&(_, c)| c.is_alphabetic())
                    .last()
                    .map(|(idx, _)| idx)
                    .unwrap_or(buf.len());
                let current_word = &buf[word_start..];
                if let Some(replaced) = try_apply_telex(current_word, ch) {
                    buf.replace_range(word_start.., &replaced);
                    continue;
                }
            }
            buf.push(ch);
        }
        buf
    }

    #[test]
    fn test_telex_common_vietnamese_words() {
        assert_eq!(type_telex("Vieetj"), "Việt");
        assert_eq!(type_telex("tieengs"), "tiếng");
        assert_eq!(
            type_telex("Coongj hoaf Xax hooij Chur nghiax"),
            "Cộng hòa Xã hội Chủ nghĩa"
        );
        assert_eq!(
            type_telex("DDoocj laapj - Tuwj do - Hanhj phucs"),
            "Độc lập - Tự do - Hạnh phúc"
        );
        assert_eq!(type_telex("dduwowcj"), "được");
        assert_eq!(type_telex("nguwowif"), "người");
        assert_eq!(type_telex("quoocs"), "quốc");
        assert_eq!(type_telex("thuyr"), "thủy");
        assert_eq!(type_telex("Nguyeenx"), "Nguyễn");
    }

    #[test]
    fn test_telex_escape_double_key_english() {
        assert_eq!(type_telex("bosss"), "boss");
        assert_eq!(type_telex("addd"), "add");
        assert_eq!(type_telex("dadd"), "dad");
        assert_eq!(type_telex("uoww"), "uow");
        assert_eq!(type_telex("test"), "tét");
        assert_eq!(type_telex("tesst"), "test");
    }

    #[test]
    fn test_telex_tone_removal_z() {
        assert_eq!(type_telex("toansz"), "toan");
    }
}
