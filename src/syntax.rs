//! Syntax highlighting: the Rust form of the old syntax/engine.js and langs.js.
//! A language is a table of states; each state is a list of rules tried in order at the cursor:
//!     (regex, class, next_state?)
//! `class` is one of k keyword, s string, n number, c comment, f function, t type, a attribute, m meta, p punctuation, e error.
//! The state at the end of a line carries into the next, which is how multi-line comments and strings work.
//! Rules must not use capturing groups, use (?:...) instead. Lookahead is allowed.
use fancy_regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

pub struct Rule { pub pat: String, pub cls: u8, pub next: Option<&'static str> }
pub struct Lang {
    pub name: &'static str,
    pub exts: &'static [&'static str],
    pub names: &'static [&'static str],
    pub comment: Option<&'static str>,
    states: Vec<(&'static str, Vec<Rule>)>,
    compiled: OnceLock<HashMap<&'static str, (Regex, Vec<(u8, Option<&'static str>)>)>>,
}

pub(crate) fn cls(c: char) -> u8 { match c { 'k' => 1, 's' => 2, 'n' => 3, 'c' => 4, 'f' => 5, 't' => 6, 'a' => 7, 'm' => 8, 'p' => 9, 'e' => 10, '\u{1}' => 11, '\u{2}' => 12, _ => 0 } }
pub(crate) fn r(p: &str, c: char) -> Rule { Rule { pat: p.to_string(), cls: cls(c), next: None } }
pub(crate) fn rn(p: &str, c: char, n: &'static str) -> Rule { Rule { pat: p.to_string(), cls: cls(c), next: Some(n) } }
pub(crate) fn kw(w: &[&str]) -> String { format!(r"\b(?:{})\b", w.join("|")) }

pub(crate) const DQ: &str = r#""(?:[^"\\]|\\.)*""#;
pub(crate) const SQ: &str = r#"'(?:[^'\\]|\\.)*'"#;
pub(crate) const NUM: &str = r"\b(?:0x[\da-fA-F_]+|\d[\d_]*(?:\.\d+)?(?:[eE][+-]?\d+)?)\b";

pub(crate) fn lang(name: &'static str, exts: &'static [&'static str], names: &'static [&'static str], comment: Option<&'static str>, states: Vec<(&'static str, Vec<Rule>)>) -> Lang {
    Lang { name, exts, names, comment, states, compiled: OnceLock::new() }
}

pub fn all() -> &'static Vec<Lang> {
    static L: OnceLock<Vec<Lang>> = OnceLock::new();
    L.get_or_init(build)
}

fn build() -> Vec<Lang> {
    let blk = |s: &'static str| vec![rn(r"\*/", 'c', s), r(r"[^*]+|\*", 'c')];
    let js_kw = kw(&["const", "let", "var", "function", "return", "if", "else", "for", "while", "do", "switch", "case", "break", "continue", "new", "class", "extends", "import", "export", "from", "default", "async", "await", "try", "catch", "finally", "throw", "typeof", "instanceof", "in", "of", "this", "super", "static", "get", "set", "yield", "delete", "void"]);
    let ts_kw = kw(&["const", "let", "var", "function", "return", "if", "else", "for", "while", "do", "switch", "case", "break", "continue", "new", "class", "extends", "implements", "import", "export", "from", "default", "async", "await", "try", "catch", "finally", "throw", "typeof", "instanceof", "in", "of", "this", "super", "static", "get", "set", "interface", "type", "enum", "namespace", "declare", "abstract", "public", "private", "protected", "readonly", "as", "keyof", "infer", "is"]);
    let mut v = vec![
        lang("Plain text", &["txt", "text"], &[], None, vec![("main", vec![])]),
        lang("JavaScript", &["js", "mjs", "cjs"], &[], Some("//"), vec![
            ("main", vec![r("//.*", 'c'), rn(r"/\*", 'c', "blk"), r(&format!("{DQ}|{SQ}"), 's'), rn("`", 's', "tpl"), r(&js_kw, 'k'), r(&kw(&["true", "false", "null", "undefined", "NaN", "Infinity"]), 'n'), r(NUM, 'n'),
                r(r"[A-Z][\w$]*", 't'), r(r"[A-Za-z_$][\w$]*(?=\s*\()", 'f'), r(r"[{}()\[\];,.:?<>=!+\-*/%&|^~]+", 'p')]),
            ("blk", blk("main")),
            ("tpl", vec![r(r"\\.", 'm'), rn("`", 's', "main"), r(r"\$\{", 'm'), r(r"[^`\\$]+|\$", 's')]),
        ]),
        lang("TypeScript", &["ts", "mts", "cts"], &[], Some("//"), vec![
            ("main", vec![r("//.*", 'c'), rn(r"/\*", 'c', "blk"), r(&format!("{DQ}|{SQ}"), 's'), rn("`", 's', "tpl"), r(&ts_kw, 'k'), r(&kw(&["true", "false", "null", "undefined", "NaN"]), 'n'), r(NUM, 'n'),
                r(&kw(&["string", "number", "boolean", "void", "any", "unknown", "never", "object"]), 't'), r(r"[A-Z][\w$]*", 't'), r(r"[A-Za-z_$][\w$]*(?=\s*[(<])", 'f'), r(r"[{}()\[\];,.:?<>=!+\-*/%&|^~]+", 'p')]),
            ("blk", blk("main")),
            ("tpl", vec![r(r"\\.", 'm'), rn("`", 's', "main"), r(r"\$\{", 'm'), r(r"[^`\\$]+|\$", 's')]),
        ]),
        lang("JSON", &["json", "jsonc", "webmanifest"], &[], None, vec![("main", vec![r(&format!(r#"{DQ}(?=\s*:)"#), 'a'), r(DQ, 's'), r(&kw(&["true", "false", "null"]), 'k'), r(r"-?\d[\d.eE+-]*", 'n'), r(r"[{}\[\],:]", 'p')])]),
        lang("CSS", &["css"], &[], None, vec![
            ("main", vec![rn(r"/\*", 'c', "blk"), r(&format!("{DQ}|{SQ}"), 's'), r(r"#[\da-fA-F]{3,8}\b", 'n'), r(r"@[\w-]+", 'k'), r(r"--[\w-]+", 'm'), r(r"[\w-]+(?=\s*:)", 'a'),
                r(r"-?\d*\.?\d+(?:px|em|rem|%|vh|vw|s|ms|deg|fr|ch)?", 'n'), r(r"[.#][\w-]+", 't'), r(r"[{}();:,>+~*]", 'p')]),
            ("blk", blk("main")),
        ]),
        lang("HTML", &["html", "htm"], &[], None, vec![
            ("main", vec![rn("<!--", 'c', "hc"), r(r"<![^>]*>", 'm'), rn(r"</?[\w:-]+", 't', "tag"), r(r"&#?\w+;", 'm')]),
            ("tag", vec![rn(r"/?>", 't', "main"), r(r"[\w:@.-]+(?=\s*=)", 'a'), r(&format!("{DQ}|{SQ}"), 's'), r("=", 'p')]),
            ("hc", vec![rn("-->", 'c', "main"), r(r"[^-]+|-", 'c')]),
        ]),
        lang("Markdown", &["md", "markdown"], &[], None, vec![
            ("main", vec![r(r"^#{1,6}\s.*", 'k'), rn(r"^```.*", 'm', "fence"), r(r"^>.*", 'c'), r(r"^\s*(?:[-*+]|\d+\.)\s", 'p'), r(r"`[^`]*`", 's'), r(r"\*\*[^*]+\*\*|__[^_]+__", 't'), r(r"\*[^*\s][^*]*\*", 'f'), r(r"\[[^\]]*\]\([^)]*\)", 'a')]),
            ("fence", vec![rn(r"^```.*", 'm', "main"), r(r".+", 's')]),
        ]),
        lang("Rust", &["rs"], &[], Some("//"), vec![
            ("main", vec![r("//.*", 'c'), rn(r"/\*", 'c', "blk"), r(DQ, 's'), r(r"'(?:[^'\\]|\\.)'", 's'), r(r"'\w+", 'm'), r(r"#!?\[[^\]]*\]", 'm'),
                r(&kw(&["fn", "let", "mut", "pub", "struct", "enum", "impl", "trait", "use", "mod", "match", "if", "else", "for", "while", "loop", "return", "self", "Self", "crate", "super", "where", "as", "in", "ref", "move", "async", "await", "unsafe", "const", "static", "type", "dyn", "break", "continue"]), 'k'),
                r(&kw(&["true", "false", "None", "Some", "Ok", "Err"]), 'n'), r(NUM, 'n'), r(r"\w+!", 'f'), r(r"[A-Z]\w*", 't'), r(r"[a-z_]\w*(?=\s*[(<])", 'f'), r(r"[{}()\[\];,.:<>=!+\-*/%&|^?]+", 'p')]),
            ("blk", blk("main")),
        ]),
        lang("Python", &["py", "pyw"], &[], Some("#"), vec![
            ("main", vec![r("#.*", 'c'), rn(r#"""""#, 's', "tq1"), rn("'''", 's', "tq2"), r(&format!("{DQ}|{SQ}"), 's'), r(r"@\w+", 'm'),
                r(&kw(&["def", "class", "return", "if", "elif", "else", "for", "while", "in", "is", "not", "and", "or", "import", "from", "as", "with", "try", "except", "finally", "raise", "lambda", "pass", "break", "continue", "yield", "global", "nonlocal", "async", "await", "del", "assert"]), 'k'),
                r(&kw(&["True", "False", "None"]), 'n'), r(NUM, 'n'), r(&kw(&["print", "len", "range", "str", "int", "float", "list", "dict", "set", "open", "self"]), 'f'), r(r"\w+(?=\()", 'f'), r(r"[{}()\[\],.:=<>!+\-*/%&|^]+", 'p')]),
            ("tq1", vec![rn(r#"""""#, 's', "main"), r(r#"[^"]+|""#, 's')]),
            ("tq2", vec![rn("'''", 's', "main"), r(r"[^']+|'", 's')]),
        ]),
        lang("TOML / INI", &["toml", "ini", "cfg", "conf", "properties"], &[".gitconfig", ".editorconfig", "cargo.lock"], Some("#"), vec![("main", vec![r(r"[;#].*", 'c'), r(r"^\s*\[[^\]]+\]", 't'), r(r"^\s*[\w.-]+(?=\s*=)", 'a'), r(&format!("{DQ}|{SQ}"), 's'), r(&kw(&["true", "false"]), 'k'), r(NUM, 'n')])]),
        lang("YAML", &["yml", "yaml"], &[], Some("#"), vec![("main", vec![r("#.*", 'c'), r(r"^\s*(?:-\s+)?[\w.-]+(?=:(?:\s|$))", 'a'), r(&format!("{DQ}|{SQ}"), 's'), r(&kw(&["true", "false", "null", "yes", "no"]), 'k'), r(NUM, 'n'), r(r"[&*]\w+", 'm'), r(r"^\s*-", 'p')])]),
        lang("Shell", &["sh", "bash", "zsh"], &[], Some("#"), vec![("main", vec![r("#.*", 'c'), r(&format!("{DQ}|{SQ}"), 's'), r(r"\$\{?\w+\}?", 'm'), r(&kw(&["if", "then", "else", "elif", "fi", "for", "while", "do", "done", "case", "esac", "function", "in", "set", "export", "local", "return"]), 'k'), r(NUM, 'n')])]),
        lang("Batch", &["bat", "cmd"], &[], Some("rem "), vec![("main", vec![r(r"(?:rem\b|::).*", 'c'), r(r"%[\w~:]+%|%%?\w", 'm'), r(&kw(&["echo", "set", "if", "else", "for", "in", "do", "goto", "call", "exit", "pause", "start", "cd"]), 'k'), r(DQ, 's'), r(NUM, 'n')])]),
        lang("Log", &["log"], &[], None, vec![("main", vec![r(r"\d{4}-\d\d-\d\d[T ]\d\d:\d\d:\d\d\S*", 'n'), r(&kw(&["ERROR", "FATAL", "Exception"]), 'e'), r(&kw(&["WARN", "WARNING"]), 'm'), r(&kw(&["INFO", "DEBUG", "TRACE"]), 'k'), r(r"\[[^\]]*\]", 't')])]),
    ];
    v.extend(crate::langs::more());
    v
}

pub fn for_file(name: &str) -> usize {
    let k = name.to_lowercase();
    let ext = k.rsplit_once('.').map(|(_, e)| e.to_string()).unwrap_or_default();
    all().iter().position(|l| l.names.contains(&k.as_str())).or_else(|| all().iter().position(|l| l.exts.contains(&ext.as_str()))).unwrap_or(0)
}

impl Lang {
    fn compiled(&self) -> &HashMap<&'static str, (Regex, Vec<(u8, Option<&'static str>)>)> {
        self.compiled.get_or_init(|| {
            self.states.iter().map(|(name, rules)| {
                let mut parts: Vec<String> = rules.iter().map(|r| format!("({})", r.pat)).collect();
                parts.push(r"(\w+|\s+|[\s\S])".into()); // fallback so the cursor always advances
                let mut meta: Vec<(u8, Option<&'static str>)> = rules.iter().map(|r| (r.cls, r.next)).collect();
                meta.push((0, None));
                (*name, (Regex::new(&format!(r"\A(?:{})", parts.join("|"))).expect("bad highlight rule"), meta))
            }).collect()
        })
    }
}

/// Tokenise one line starting in `state`. Returns (text, class) pieces and the state the next line starts in.
pub fn line(lang: &Lang, text: &str, state: &'static str) -> (Vec<(String, u8)>, &'static str) {
    let comp = lang.compiled();
    let mut toks: Vec<(String, u8)> = Vec::new();
    let (mut pos, mut st) = (0usize, state);
    while pos < text.len() {
        let (re, meta) = comp.get(st).or_else(|| comp.get("main")).unwrap();
        let rest = &text[pos..];
        let mut hit = None;
        if let Ok(Some(c)) = re.captures(rest) {
            for g in 1..=meta.len() { if let Some(m) = c.get(g) { hit = Some((g - 1, m.end())); break; } }
        }
        let (gi, len) = match hit { Some((g, l)) if l > 0 => (g, l), _ => { let n = rest.chars().next().map(|c| c.len_utf8()).unwrap_or(1); (meta.len() - 1, n) } };
        let piece = rest[..len].to_string();
        let (c, next) = meta[gi];
        match toks.last_mut() { Some(l) if l.1 == c => l.0.push_str(&piece), _ => toks.push((piece, c)) }
        pos += len;
        if let Some(n) = next { st = n; }
    }
    (toks, st)
}

/// state names are 'static strs inside the language table; find the canonical one for a given name
pub fn intern(lang: &Lang, name: &str) -> &'static str { lang.states.iter().find(|(n, _)| *n == name).map(|(n, _)| *n).unwrap_or("main") }
