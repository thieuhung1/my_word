use crate::document::{
    PAGE_BREAK_CHAR, StyleSpan, TextAlignment, deserialize_document, serialize_mwd,
};
use crate::editor::EditorState;
use crate::ui::apply_egui_visuals;
use eframe::egui::{self, FontFamily};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

pub enum FileDialogResult {
    Open(PathBuf),
    Save(PathBuf),
}

const OPEN_DIALOG_SCRIPT: &str = r#"Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.OpenFileDialog; $d.Filter = 'My Word / Text (*.mwd;*.txt;*.md)|*.mwd;*.txt;*.md|All files (*.*)|*.*'; if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Write-Output $d.FileName }"#;
const SAVE_DIALOG_SCRIPT: &str = r#"Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.SaveFileDialog; $d.Filter = 'My Word Rich Text (*.mwd)|*.mwd|Plain Text (*.txt)|*.txt|Markdown (*.md)|*.md'; $d.FileName = 'tai_lieu.mwd'; if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Write-Output $d.FileName }"#;

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
        let chosen = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Sta", "-Command", script])
            .output()
            .ok()
            .and_then(|o| {
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
    let is_plain = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("txt") || e.eq_ignore_ascii_case("md"));
    let payload = if is_plain {
        editor.content.replace(PAGE_BREAK_CHAR, "\n")
    } else {
        serialize_mwd(&editor.content, &editor.spans, &editor.para_alignments)
    };
    std::fs::write(path, payload.as_bytes())
}

pub fn load_document(
    path: &Path,
) -> Result<(String, Vec<StyleSpan>, Vec<TextAlignment>), std::io::Error> {
    let raw = std::fs::read_to_string(path)?;
    Ok(deserialize_document(&raw, path))
}

pub fn write_autosave(editor: &EditorState) {
    if let Some(local_app) = std::env::var_os("LOCALAPPDATA") {
        let dir = PathBuf::from(local_app).join("my_word");
        if std::fs::create_dir_all(&dir).is_ok() {
            let payload = serialize_mwd(&editor.content, &editor.spans, &editor.para_alignments);
            let _ = std::fs::write(dir.join("autosave.mwd"), payload.as_bytes());
        }
    }
}

/// Nạp duy nhất phông hệ thống Windows (bỏ bộ font mặc định của egui để tiết kiệm RAM)
pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::empty();

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
            .insert(FontFamily::Proportional, vec!["sys_regular".to_owned()]);
        fonts
            .families
            .insert(FontFamily::Monospace, vec!["sys_regular".to_owned()]);
    }

    if load_font(
        &mut fonts,
        "sys_bold",
        &[
            "C:\\Windows\\Fonts\\segoeuib.ttf",
            "C:\\Windows\\Fonts\\arialbd.ttf",
        ],
    ) {
        fonts.families.insert(
            FontFamily::Name("Bold".into()),
            vec!["sys_bold".to_owned(), "sys_regular".to_owned()],
        );
    }
    if load_font(
        &mut fonts,
        "sys_italic",
        &[
            "C:\\Windows\\Fonts\\segoeuii.ttf",
            "C:\\Windows\\Fonts\\ariali.ttf",
        ],
    ) {
        fonts.families.insert(
            FontFamily::Name("Italic".into()),
            vec!["sys_italic".to_owned(), "sys_regular".to_owned()],
        );
    }
    if load_font(
        &mut fonts,
        "sys_bold_italic",
        &[
            "C:\\Windows\\Fonts\\segoeuiz.ttf",
            "C:\\Windows\\Fonts\\arialbi.ttf",
        ],
    ) {
        fonts.families.insert(
            FontFamily::Name("BoldItalic".into()),
            vec![
                "sys_bold_italic".to_owned(),
                "sys_bold".to_owned(),
                "sys_regular".to_owned(),
            ],
        );
    }

    ctx.set_fonts(fonts);
    apply_egui_visuals(ctx, false);
}
