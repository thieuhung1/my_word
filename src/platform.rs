use crate::document::{
    PAGE_BREAK_CHAR, StyleSpan, TextAlignment, deserialize_document, export_word_doc, serialize_mwd,
};
use crate::editor::EditorState;
use eframe::egui::{self, FontFamily};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

pub enum FileDialogResult {
    Open(PathBuf),
    Save(PathBuf),
}

const OPEN_DIALOG_SCRIPT: &str = r#"Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.Form; $f.TopMost = $true; $d = New-Object System.Windows.Forms.OpenFileDialog; $d.Filter = 'Tệp văn bản (*.doc;*.mwd;*.txt;*.md)|*.doc;*.mwd;*.txt;*.md|Microsoft Word (*.doc)|*.doc|My Word Rich Text (*.mwd)|*.mwd|Plain Text (*.txt)|*.txt|Markdown (*.md)|*.md|All files (*.*)|*.*'; if ($d.ShowDialog($f) -eq [System.Windows.Forms.DialogResult]::OK) { $OutputEncoding = [System.Text.Encoding]::UTF8; try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}; Write-Output $d.FileName }"#;
const SAVE_DIALOG_SCRIPT: &str = r#"Add-Type -AssemblyName System.Windows.Forms; $f = New-Object System.Windows.Forms.Form; $f.TopMost = $true; $d = New-Object System.Windows.Forms.SaveFileDialog; $d.Filter = 'Microsoft Word (*.doc)|*.doc|My Word Rich Text (*.mwd)|*.mwd|Plain Text (*.txt)|*.txt|Markdown (*.md)|*.md'; $d.FileName = 'tai_lieu.doc'; if ($d.ShowDialog($f) -eq [System.Windows.Forms.DialogResult]::OK) { $OutputEncoding = [System.Text.Encoding]::UTF8; try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch {}; Write-Output $d.FileName }"#;

fn spawn_powershell_dialog<F>(
    ctx: egui::Context,
    script: &'static str,
    map: F,
) -> Receiver<Option<FileDialogResult>>
where
    F: FnOnce(PathBuf) -> FileDialogResult + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut cmd = std::process::Command::new("powershell");
        cmd.args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Sta",
            "-Command",
            script,
        ]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let chosen = cmd.output().ok().and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            (!s.is_empty()).then(|| map(PathBuf::from(s)))
        });
        let _ = tx.send(chosen);
        ctx.request_repaint();
    });
    rx
}

pub fn spawn_open_dialog(ctx: egui::Context) -> Receiver<Option<FileDialogResult>> {
    spawn_powershell_dialog(ctx, OPEN_DIALOG_SCRIPT, FileDialogResult::Open)
}

pub fn spawn_save_dialog(ctx: egui::Context) -> Receiver<Option<FileDialogResult>> {
    spawn_powershell_dialog(ctx, SAVE_DIALOG_SCRIPT, FileDialogResult::Save)
}

pub fn write_document(path: &Path, editor: &EditorState) -> Result<(), std::io::Error> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let payload = if ext.eq_ignore_ascii_case("txt") || ext.eq_ignore_ascii_case("md") {
        editor.content.replace(PAGE_BREAK_CHAR, "\n")
    } else if ext.eq_ignore_ascii_case("doc")
        || ext.eq_ignore_ascii_case("docx")
        || ext.eq_ignore_ascii_case("htm")
        || ext.eq_ignore_ascii_case("html")
    {
        export_word_doc(&editor.content, &editor.spans, &editor.para_alignments)
    } else {
        serialize_mwd(&editor.content, &editor.spans, &editor.para_alignments)
    };
    std::fs::write(path, payload.as_bytes())
}

const MAX_DOCUMENT_BYTES: u64 = 32 * 1024 * 1024;

pub fn load_document(
    path: &Path,
) -> Result<(String, Vec<StyleSpan>, Vec<TextAlignment>), std::io::Error> {
    let meta = std::fs::metadata(path)?;
    if meta.len() > MAX_DOCUMENT_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "Tệp quá lớn (vượt giới hạn {} MB)",
                MAX_DOCUMENT_BYTES / (1024 * 1024)
            ),
        ));
    }
    let raw = std::fs::read_to_string(path)?;
    Ok(deserialize_document(&raw, path))
}

pub fn autosave_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|local_app| {
        PathBuf::from(local_app)
            .join("my_word")
            .join("autosave.mwd")
    })
}

pub fn autosave_path_doc() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|local_app| {
        PathBuf::from(local_app)
            .join("my_word")
            .join("autosave.doc")
    })
}

pub fn has_autosave() -> bool {
    let has_mwd = autosave_path().is_some_and(|p| {
        p.is_file() && std::fs::metadata(&p).map(|m| m.len() > 0).unwrap_or(false)
    });
    let has_doc = autosave_path_doc().is_some_and(|p| {
        p.is_file() && std::fs::metadata(&p).map(|m| m.len() > 0).unwrap_or(false)
    });
    has_mwd || has_doc
}

pub fn read_autosave() -> Result<(String, Vec<StyleSpan>, Vec<TextAlignment>), std::io::Error> {
    if let Some(path) = autosave_path()
        && path.is_file()
    {
        load_document(&path)
    } else if let Some(doc_path) = autosave_path_doc()
        && doc_path.is_file()
    {
        load_document(&doc_path)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Thư mục LOCALAPPDATA không tồn tại hoặc không có tệp sao lưu",
        ))
    }
}

pub fn clear_autosave() {
    if let Some(path) = autosave_path()
        && path.is_file()
    {
        let _ = std::fs::remove_file(path);
    }
    if let Some(doc_path) = autosave_path_doc()
        && doc_path.is_file()
    {
        let _ = std::fs::remove_file(doc_path);
    }
}

pub fn write_autosave(editor: &EditorState) {
    if let Some(path) = autosave_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let payload = serialize_mwd(&editor.content, &editor.spans, &editor.para_alignments);
        let _ = std::fs::write(&path, payload.as_bytes());
    }
    // Luôn sao lưu song song thành file .doc chuẩn Microsoft Word
    if let Some(doc_path) = autosave_path_doc() {
        if let Some(parent) = doc_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let doc_payload = export_word_doc(&editor.content, &editor.spans, &editor.para_alignments);
        let _ = std::fs::write(&doc_path, doc_payload.as_bytes());
    }
}

/// Nạp phông hệ thống Windows làm ưu tiên hàng đầu, đồng thời giữ bộ phông mặc định của egui
/// ở cuối danh sách fallback để đảm bảo không bị thiếu ký tự/glyph lạ.
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let load_font = |fonts: &mut egui::FontDefinitions, name: &str, paths: &[&str]| -> bool {
        for path in paths {
            if let Ok(data) = std::fs::read(path) {
                fonts
                    .font_data
                    .insert(name.to_owned(), egui::FontData::from_owned(data));
                return true;
            }
        }
        false
    };

    if load_font(
        &mut fonts,
        "sys_regular",
        &[
            "C:\\Windows\\Fonts\\segoeui.ttf",
            "C:\\Windows\\Fonts\\arial.ttf",
            "C:\\Windows\\Fonts\\calibri.ttf",
        ],
    ) {
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "sys_regular".to_owned());
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "sys_regular".to_owned());
    }

    if load_font(
        &mut fonts,
        "sys_bold",
        &[
            "C:\\Windows\\Fonts\\segoeuib.ttf",
            "C:\\Windows\\Fonts\\arialbd.ttf",
        ],
    ) {
        let list = fonts
            .families
            .entry(FontFamily::Name("Bold".into()))
            .or_default();
        list.insert(0, "sys_regular".to_owned());
        list.insert(0, "sys_bold".to_owned());
    }
    if load_font(
        &mut fonts,
        "sys_italic",
        &[
            "C:\\Windows\\Fonts\\segoeuii.ttf",
            "C:\\Windows\\Fonts\\ariali.ttf",
        ],
    ) {
        let list = fonts
            .families
            .entry(FontFamily::Name("Italic".into()))
            .or_default();
        list.insert(0, "sys_regular".to_owned());
        list.insert(0, "sys_italic".to_owned());
    }
    if load_font(
        &mut fonts,
        "sys_bold_italic",
        &[
            "C:\\Windows\\Fonts\\segoeuiz.ttf",
            "C:\\Windows\\Fonts\\arialbi.ttf",
        ],
    ) {
        let list = fonts
            .families
            .entry(FontFamily::Name("BoldItalic".into()))
            .or_default();
        list.insert(0, "sys_regular".to_owned());
        list.insert(0, "sys_bold".to_owned());
        list.insert(0, "sys_bold_italic".to_owned());
    }

    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autosave_lifecycle_write_read_clear() {
        let ed = EditorState::from_plain_text("Văn bản kiểm thử lưu tự động");
        write_autosave(&ed);
        assert!(has_autosave());
        if let Some(doc_p) = autosave_path_doc() {
            assert!(doc_p.is_file());
        }

        let res = read_autosave();
        assert!(res.is_ok());
        let (content, spans, aligns) = res.unwrap();
        assert_eq!(content, "Văn bản kiểm thử lưu tự động");
        assert_eq!(spans.len(), 1);
        assert_eq!(aligns.len(), 1);

        clear_autosave();
        assert!(!has_autosave());
        if let Some(doc_p) = autosave_path_doc() {
            assert!(!doc_p.is_file());
        }
    }
}
