//! Guess a language from what is actually typed, for files whose extension says otherwise (a .txt full of Lua, say).
//! Each language has a few signature patterns with weights; the best score wins if it clearly beats the current one.
use crate::syntax;
use fancy_regex::Regex;
use std::sync::OnceLock;

struct Sig { lang: &'static str, pats: Vec<(Regex, i32)> }

fn sigs() -> &'static Vec<Sig> {
    static S: OnceLock<Vec<Sig>> = OnceLock::new();
    S.get_or_init(|| {
        let t: &[(&str, &[(&str, i32)])] = &[
            ("Lua", &[(r"(?m)^\s*local\s+\w+", 2), (r"(?m)\bthen\s*$", 2), (r"(?m)^\s*end\s*$", 1), (r"\bnil\b", 1), (r"~=", 2), (r"(?m)^\s*--", 2), (r"\belseif\b", 2), (r"\b(?:ipairs|pairs)\(", 2), (r"\bfunction\s+[\w.:]+\s*\(", 1), (r"\blocal\s+function\b", 3)]),
            ("Python", &[(r"(?m)^\s*def\s+\w+\(.*\)\s*(?:->.*)?:\s*$", 3), (r"(?m)^\s*(?:import\s+\w|from\s+[\w.]+\s+import)", 2), (r"(?m)^\s*(?:elif\b|except\b.*:)", 3), (r"\bself\.\w+", 1), (r"if __name__", 4), (r"(?m)^\s*class\s+\w+(?:\(.*\))?:\s*$", 3), (r"\bprint\(", 1), (r"\b(?:None|True|False)\b", 1)]),
            ("Rust", &[(r"\bfn\s+\w+", 2), (r"\blet\s+mut\b", 3), (r"\b(?:println|vec|format|panic)!", 3), (r"\bimpl\b[^\n]*\{", 2), (r"\buse\s+\w+::", 3), (r"#\[derive", 4), (r"\bpub\s+(?:fn|struct|enum)\b", 3)]),
            ("JavaScript", &[(r"\bconsole\.\w+", 2), (r"\b(?:const|let)\s+\w+\s*=", 2), (r"=>", 1), (r"\bfunction\s*\w*\(", 1), (r"\brequire\(|module\.exports", 3), (r"===|!==", 2), (r"\b(?:document|window)\.\w+", 2)]),
            ("TypeScript", &[(r"\b(?:const|let)\s+\w+\s*=", 1), (r"\bconsole\.\w+", 1), (r"=>", 1), (r":\s*(?:string|number|boolean)\b", 4), (r"\binterface\s+\w+\s*\{", 4), (r"\btype\s+\w+\s*=", 3), (r"\bimport\s+[^\n]*\bfrom\b", 1), (r"<\w+(?:\[\])?>", 1)]),
            ("HTML", &[(r"(?i)<!DOCTYPE html|<html|<body|<head|<script|<div", 4), (r"</\w+>", 1)]),
            ("XML", &[(r"(?m)^\s*<\?xml", 8)]),
            ("CSS", &[(r"(?m)^\s*[.#][\w-]+[^\n]*\{\s*$", 3), (r"(?m)^\s+[a-z-]+:\s*[^;{}]+;\s*$", 2), (r"@media|@import|@font-face", 3)]),
            ("Shell", &[(r"(?m)^#!.*\b(?:ba|z)?sh\b", 8), (r"\b(?:fi|esac|done)\b", 2), (r"\$\(|\$\{", 1), (r"(?m)^\s*(?:echo|export|cd)\s", 1)]),
            ("PowerShell", &[(r"\b[A-Z][a-z]+-[A-Z]\w+", 3), (r"\$\w+\s*=", 2), (r"(?i)\bparam\s*\(", 3), (r"\[CmdletBinding", 5), (r"Write-Host", 4)]),
            ("SQL", &[(r"(?is)\bselect\b.*\bfrom\b", 4), (r"(?i)\b(?:insert\s+into|create\s+table|alter\s+table|delete\s+from)\b", 5), (r"(?i)\bwhere\b", 1)]),
            ("PHP", &[(r"<\?php", 8), (r"\$\w+\s*->", 3), (r"\bfunction\s+\w+\(\$", 2)]),
            ("Ruby", &[(r"(?m)^\s*def\s+\w+[?!]?\s*(?:\(.*\))?\s*$", 3), (r"\bputs\b", 3), (r"\battr_\w+", 4), (r"\bdo\s*\|\w+\|", 4), (r"\brequire\s+['\x22]", 2), (r"\belsif\b", 2), (r"(?m)^\s*end\s*$", 1)]),
            ("Go", &[(r"(?m)^package\s+\w+", 4), (r"\bfunc\s+(?:\(\w+\s+\*?\w+\)\s+)?\w+\(", 4), (r":=", 2), (r"\bfmt\.\w+", 4)]),
            ("Java", &[(r"\bpublic\s+(?:static\s+)?(?:class|void|final)\b", 3), (r"System\.out\.print", 5), (r"(?m)^import\s+java\.", 5), (r"@Override", 3)]),
            ("C", &[(r"#include\s*<\w+\.h>", 5), (r"\bint\s+main\s*\(", 4), (r"\bprintf\(", 3), (r"\bmalloc\(", 3)]),
            ("C++", &[(r"#include\s*<(?:iostream|vector|string|map|memory|algorithm)>", 7), (r"std::", 4), (r"\bcout\b", 4), (r"\bnamespace\b", 2), (r"template\s*<", 3), (r"\bint\s+main\s*\(", 2)]),
            ("C#", &[(r"(?m)^using\s+System", 6), (r"Console\.Write", 5), (r"\bnamespace\s+[\w.]+", 2), (r"\b(?:string|var)\s+\w+\s*=", 1)]),
            ("Kotlin", &[(r"\bfun\s+\w+\(", 4), (r"\bval\s+\w+", 3)]),
            ("Swift", &[(r"\bfunc\s+\w+\([^\n]*\)\s*(?:->|\{)", 3), (r"\bguard\s+let\b", 5), (r"(?m)^import\s+(?:SwiftUI|Foundation|UIKit)", 6)]),
            ("Diff / Patch", &[(r"(?m)^@@ -\d+(?:,\d+)? \+\d+(?:,\d+)? @@", 8), (r"(?m)^diff --git", 8), (r"(?m)^(?:---|\+\+\+) \S", 3)]),
        ];
        t.iter().map(|(l, ps)| Sig { lang: l, pats: ps.iter().filter_map(|(p, w)| Regex::new(p).ok().map(|r| (r, *w))).collect() }).collect()
    })
}

fn score(sig: &Sig, text: &str) -> i32 {
    sig.pats.iter().map(|(re, w)| { let n = re.find_iter(text).take(3).filter(|m| m.is_ok()).count() as i32; n * w }).sum()
}

/// the language this text looks like, if it is clearly different from `current` (an index into syntax::all()).
/// A language without a signature table is only replaced when it is plain text.
pub fn detect(lines: &[String], current: usize) -> Option<usize> {
    let all = syntax::all();
    let sample: String = lines.iter().take(300).map(|s| s.as_str()).collect::<Vec<_>>().join("\n");
    if sample.trim().len() < 12 { return None; }
    let tr = sample.trim_start();
    if (tr.starts_with('{') || tr.starts_with('[')) && serde_json::from_str::<serde_json::Value>(&sample).is_ok() {
        return all.iter().position(|l| l.name == "JSON").filter(|i| *i != current);
    }
    let cur_name = all[current].name;
    let cur_has = sigs().iter().any(|s| s.lang == cur_name);
    if !cur_has && current != 0 { return None; }
    let cur_score = sigs().iter().find(|s| s.lang == cur_name).map(|s| score(s, &sample)).unwrap_or(0);
    let (mut best, mut best_s) = (None, 0);
    for s in sigs() { if s.lang == cur_name { continue; } let sc = score(s, &sample); if sc > best_s { best_s = sc; best = Some(s.lang); } }
    if best_s >= 6 && best_s >= cur_score + 4 { best.and_then(|n| all.iter().position(|l| l.name == n)) } else { None }
}
