//! Fragment matching for the go bar (the Rust form of the old match.js).
//! match_name("ath", "Athena") -> a score, why it matched, and which characters of the name matched.
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Norm {
    pub s: Vec<char>,       // lowercase letters and digits only
    pub map: Vec<usize>,    // position in the original name of each char in `s`
    pub w: Vec<usize>,      // where each word starts in `s` (separators, camelCase and digit edges)
}

thread_local! { static CACHE: RefCell<HashMap<String, Rc<Norm>>> = RefCell::new(HashMap::new()); }

pub fn norm_name(name: &str) -> Rc<Norm> {
    CACHE.with(|c| {
        if let Some(n) = c.borrow().get(name) {
            return n.clone();
        }
        let (mut s, mut map, mut w, mut prev) = (Vec::new(), Vec::new(), Vec::new(), 0u8);
        for (i, ch) in name.chars().enumerate() {
            if !ch.is_alphanumeric() {
                prev = 0;
                continue;
            }
            let lo = ch.to_lowercase().next().unwrap_or(ch);
            let cls = if ch.is_numeric() { 3 } else if lo != ch { 2 } else { 1 };
            if prev == 0 || (cls == 2 && prev == 1) || (cls == 3 && prev != 3) || (cls != 3 && prev == 3) {
                w.push(s.len());
            }
            s.push(lo);
            map.push(i);
            prev = cls;
        }
        let n = Rc::new(Norm { s, map, w });
        let mut m = c.borrow_mut();
        if m.len() > 100_000 {
            m.clear();
        }
        m.insert(name.to_string(), n.clone());
        n
    })
}

pub fn norm_tok(t: &str) -> Vec<char> {
    t.chars().filter(|c| c.is_alphanumeric()).flat_map(|c| c.to_lowercase()).collect()
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind { Exact, Prefix, Word, Initials, Contains, Typo, Fuzzy }
impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Exact => "exact", Kind::Prefix => "starts with", Kind::Word => "word start", Kind::Initials => "initials",
            Kind::Contains => "contains", Kind::Typo => "typo fixed", Kind::Fuzzy => "fuzzy",
        }
    }
}

pub struct Match {
    pub score: i32,
    pub kind: Kind,
    pub idx: Vec<usize>, // character positions in the original name
}

fn within1(a: &[char], b: &[char]) -> bool {
    if a == b { return true; }
    let (la, lb) = (a.len(), b.len());
    if la.abs_diff(lb) > 1 { return false; }
    let mut i = 0;
    while i < la && i < lb && a[i] == b[i] { i += 1; }
    if la == lb {
        return a[i + 1..] == b[i + 1..] || (i + 1 < la && a[i] == b[i + 1] && a[i + 1] == b[i] && a[i + 2..] == b[i + 2..]);
    }
    if la > lb { a[i + 1..] == b[i..] } else { b[i + 1..] == a[i..] }
}

pub fn match_name(t: &[char], name: &str) -> Option<Match> {
    let n = norm_name(name);
    let (s, tl) = (&n.s, t.len());
    if tl == 0 || s.is_empty() { return None; }
    let mk = |score: i32, kind: Kind, ix: Vec<usize>| Some(Match { score, kind, idx: ix.into_iter().map(|i| n.map[i]).collect() });
    let rng = |a: usize, b: usize| (a..b).collect::<Vec<_>>();
    if s.as_slice() == t { return mk(1000, Kind::Exact, rng(0, tl)); }
    if s.starts_with(t) { return mk(860 - (((s.len() - tl) * 3) as i32).min(120), Kind::Prefix, rng(0, tl)); }
    for &w in &n.w {
        if w > 0 && s[w..].starts_with(t) { return mk(720 - w as i32, Kind::Word, rng(w, w + tl)); }
    }
    if tl >= 2 && tl <= n.w.len() {
        let (mut wi, mut ix, mut ok) = (0, Vec::new(), true);
        for &c in t {
            while wi < n.w.len() && s[n.w[wi]] != c { wi += 1; }
            if wi == n.w.len() { ok = false; break; }
            ix.push(n.w[wi]);
            wi += 1;
        }
        if ok { return mk(640 - ((n.w.len() - tl) as i32) * 10, Kind::Initials, ix); }
    }
    if let Some(p) = s.windows(tl).position(|w| w == t) { return mk(500 - p as i32, Kind::Contains, rng(p, p + tl)); }
    if tl >= 4 && (within1(t, &s[..tl.min(s.len())]) || within1(t, &s[..(tl + 1).min(s.len())]) || within1(t, &s[..(tl - 1).min(s.len())])) {
        return mk(400, Kind::Typo, rng(0, tl.min(s.len())));
    }
    if tl >= 3 {
        let (mut j, mut ix) = (0, Vec::new());
        for (i, &c) in s.iter().enumerate() {
            if j < tl && c == t[j] { ix.push(i); j += 1; }
        }
        if j == tl {
            let span = ix[tl - 1] - ix[0] + 1;
            if span <= tl * 2 + 2 { return mk(300 - ((span - tl) as i32) * 8, Kind::Fuzzy, ix); }
        }
    }
    None
}
