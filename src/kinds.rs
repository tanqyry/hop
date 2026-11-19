//! What kind of thing a file is, from its name: a category (drives icons and auto view) and a label for the Type column.
pub fn cat_of(name: &str, dir: bool) -> (&'static str, String) {
    if dir { return ("folder", "Folder".into()); }
    let ext = match name.rfind('.') { Some(i) if i > 0 => name[i + 1..].to_lowercase(), _ => String::new() };
    let (c, l): (&str, &str) = match ext.as_str() {
        "txt" => ("text", "Text"), "md" => ("text", "Markdown"), "log" => ("text", "Log"), "ini" | "cfg" => ("text", "Config"),
        "toml" => ("code", "TOML"), "json" => ("code", "JSON"), "js" => ("code", "JavaScript"), "ts" => ("code", "TypeScript"),
        "rs" => ("code", "Rust"), "py" => ("code", "Python"), "html" => ("code", "HTML"), "css" => ("code", "CSS"),
        "vdf" | "xnb" => ("data", "Data"),
        "png" => ("image", "PNG image"), "jpg" | "jpeg" => ("image", "JPEG image"), "gif" => ("image", "GIF image"), "webp" => ("image", "WebP image"),
        "bmp" => ("image", "Bitmap"), "svg" => ("image", "SVG image"), "ico" => ("image", "Icon"),
        "mp3" => ("audio", "MP3 audio"), "flac" => ("audio", "FLAC audio"), "wav" => ("audio", "WAV audio"), "m3u" => ("text", "Playlist"),
        "mp4" => ("video", "MP4 video"), "mkv" => ("video", "MKV video"), "mov" => ("video", "QuickTime video"), "webm" => ("video", "WebM video"),
        "zip" => ("archive", "ZIP archive"), "7z" => ("archive", "7-Zip archive"), "rar" => ("archive", "RAR archive"), "iso" => ("archive", "Disc image"),
        "jar" => ("archive", "Java archive"), "gz" => ("archive", "GZip archive"),
        "exe" => ("exe", "Application"), "msi" => ("exe", "Installer"), "cmd" | "bat" => ("exe", "Command"), "lnk" => ("text", "Shortcut"),
        "dll" => ("sys", "Library"), "sys" => ("sys", "System file"), "node" => ("sys", "Native module"),
        "pdf" => ("doc", "PDF"), "docx" => ("doc", "Word document"), "xlsx" => ("doc", "Spreadsheet"), "pptx" => ("doc", "Presentation"),
        "db" | "sqlite" | "dat" | "ldb" | "bin" => ("data", "Data"), "tmp" => ("sys", "Temp file"), "dmp" => ("sys", "Crash dump"), "ttf" | "otf" => ("sys", "Font"),
        "" => ("text", "File"),
        other => return ("text", format!("{} file", other.to_uppercase())),
    };
    (c, l.into())
}

/// worth trying in the built-in editor (the backend still refuses binary files)
pub fn can_edit(name: &str, size: u64) -> bool {
    !matches!(cat_of(name, false).0, "image" | "audio" | "video" | "archive" | "exe" | "sys" | "doc") && size < 2 * 1048576
}

pub fn is_visual(name: &str) -> bool {
    let n = name.to_lowercase();
    ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "ico", "mp4", "mkv", "mov", "webm", "avi"].iter().any(|e| n.ends_with(&format!(".{e}")))
}
