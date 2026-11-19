//! The built-in text editor's disk side: read a file as text, save it back without losing its line endings or BOM.
use std::fs;
use std::io::Write;
use std::time::UNIX_EPOCH;

const MAX: u64 = 2 * 1048576;

pub struct Text {
    /// always '\n' line endings; `crlf` remembers what the file used
    pub text: String,
    pub crlf: bool,
    pub bom: bool,
    pub size: u64,
    pub mtime: i64,
}

fn mtime(md: &fs::Metadata) -> i64 {
    md.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_millis() as i64).unwrap_or(0)
}

pub fn read_text(path: String) -> Result<Text, String> {
    let md = fs::metadata(&path).map_err(|e| e.to_string())?;
    if md.is_dir() {
        return Err("This is a folder".into());
    }
    if md.len() > MAX {
        return Err("File is too large for the built-in editor (over 2 MB)".into());
    }
    let raw = fs::read(&path).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => "Access is denied".to_string(),
        _ => e.to_string(),
    })?;
    if raw.starts_with(&[0xFF, 0xFE]) || raw.starts_with(&[0xFE, 0xFF]) {
        return Err("UTF-16 files are not supported yet".into());
    }
    let bom = raw.starts_with(&[0xEF, 0xBB, 0xBF]);
    let body = if bom { &raw[3..] } else { &raw[..] };
    if body.contains(&0) {
        return Err("This looks like a binary file".into());
    }
    let text = match std::str::from_utf8(body) {
        Ok(s) => s.to_string(),
        Err(_) => return Err("This looks like a binary file (not valid UTF-8)".into()),
    };
    let crlf_n = text.matches("\r\n").count();
    let lf_n = text.matches('\n').count() - crlf_n;
    let text = if crlf_n > 0 { text.replace("\r\n", "\n") } else { text };
    Ok(Text { text, crlf: crlf_n > lf_n, bom, size: md.len(), mtime: mtime(&md) })
}

pub struct Saved {
    pub mtime: i64,
    pub size: u64,
}

/// `expected_mtime` is what the file's modified time was when it was opened. If it changed since, we refuse with
/// "CHANGED" and the UI asks before overwriting. The write goes to a temp file first, then replaces the original,
/// so a crash mid-save can't leave a half-written file.
pub fn write_text(path: String, text: String, crlf: bool, bom: bool, expected_mtime: Option<i64>) -> Result<Saved, String> {
    let orig = fs::metadata(&path).ok();
    if let (Some(exp), Some(m)) = (expected_mtime, &orig) {
        if mtime(m) != exp {
            return Err("CHANGED".into());
        }
    }
    let mut bytes: Vec<u8> = Vec::with_capacity(text.len() + 3);
    if bom {
        bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    if crlf {
        bytes.extend_from_slice(text.replace('\n', "\r\n").as_bytes());
    } else {
        bytes.extend_from_slice(text.as_bytes());
    }
    let p = std::path::Path::new(&path);
    let tmp = p.with_file_name(format!(".{}.hop-tmp", p.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()));
    let res = (|| -> std::io::Result<()> {
        let mut f = fs::File::create(&tmp)?;
        f.write_all(&bytes)?;
        f.sync_all()?;
        if let Some(m) = &orig {
            let _ = fs::set_permissions(&tmp, m.permissions()); // keep read-only/hidden style attributes
        }
        fs::rename(&tmp, p)
    })();
    if let Err(e) = res {
        let _ = fs::remove_file(&tmp);
        return Err(match e.kind() {
            std::io::ErrorKind::PermissionDenied => "Access is denied (is the file read-only?)".into(),
            _ => e.to_string(),
        });
    }
    let md = fs::metadata(&path).map_err(|e| e.to_string())?;
    Ok(Saved { mtime: mtime(&md), size: md.len() })
}
