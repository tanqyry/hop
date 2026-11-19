//! The go bar's brain: turn "appdata local ath ver" into folders (the Rust form of the old resolver.js).
//! Beam search down the tree, one folder level per word (up to three words may join: "program files").
use crate::core::{fsops, index::Index};
use crate::matching::{match_name, norm_tok, Kind};
use crate::util::*;
use std::collections::HashMap;
use std::collections::HashSet;

pub struct Env<'a> {
    pub cwd: &'a str,
    pub home: &'a str,
    pub aliases: &'a [(String, String)],
    pub drives: &'a [String],
    pub vars: &'a HashMap<String, String>,
    pub lone: bool,
    pub boost: &'a dyn Fn(&str) -> f64,
    pub index: &'a Index,
}

#[derive(Clone)]
pub struct Step {
    pub tok: String,
    pub path: String,
    pub name: String,
    pub kind: String,   // "alias", "starts with", ...
    pub idx: Vec<usize>,
    pub jump: bool,
    pub note: String,
    pub via: Vec<String>,
    pub total: usize,
    pub alts: usize,
}

pub struct Res {
    pub steps: Vec<Step>,
    pub lone: Vec<(String, String)>, // name, path of each lone folder opened
    pub final_path: String,
    pub dir: String,                 // the folder to open (after lone folders)
    pub file: Option<String>,        // select this file after opening `dir`
    pub counts: Option<(u32, u32)>,
    pub partial: bool,
    pub unresolved: String,
    pub score: f64,
}

pub struct Resolution {
    pub results: Vec<Res>,
    pub ms: f64,
    pub scanned: usize,
}

#[derive(Clone)]
struct State { path: String, i: usize, score: i32, dir: bool, steps: Vec<Step> }

pub fn resolve(query: &str, env: &Env) -> Resolution {
    let t0 = std::time::Instant::now();
    let mut cache: HashMap<String, Vec<(String, bool)>> = HashMap::new();
    let mut scanned = 0usize;
    let mut budget = 40i32;
    let mut list = |p: &str, cache: &mut HashMap<String, Vec<(String, bool)>>| -> Vec<(String, bool)> {
        if let Some(v) = cache.get(p) { return v.clone(); }
        scanned += 1;
        let v: Vec<(String, bool)> = std::fs::read_dir(p)
            .map(|rd| rd.flatten().map(|e| (e.file_name().to_string_lossy().into_owned(), e.file_type().map(|t| t.is_dir()).unwrap_or(false))).collect())
            .unwrap_or_default();
        cache.insert(p.to_string(), v.clone());
        v
    };

    let toks: Vec<String> = query.split(|c: char| c.is_whitespace() || c == '\\' || c == '/').filter(|t| !t.is_empty()).map(String::from).collect();
    let n = toks.len();
    if n == 0 { return Resolution { results: vec![], ms: 0.0, scanned: 0 }; }
    let jt = |i: usize, k: usize| norm_tok(&toks[i..i + k].join(""));
    let label = |i: usize, k: usize| toks[i..i + k].join(" ");
    let mut buckets: Vec<Vec<State>> = vec![Vec::new(); n + 1];

    /* step 1: where can the first word(s) start? alias, env var, drive, here, home, the index */
    let mut first: HashMap<String, (i32, Step, bool)> = HashMap::new();
    let mut add = |first: &mut HashMap<String, (i32, Step, bool)>, path: String, score: i32, st: Step, dir: bool| {
        match first.get(&path) { Some(o) if o.0 >= score => {} _ => { first.insert(path, (score, st, dir)); } }
    };
    for k in 1..=n.min(3) {
        let raw = toks[0].to_lowercase();
        let tok = jt(0, k);
        let last = k == n;
        let lab = label(0, k);
        let mk = |path: &str, kind: &str, idx: Vec<usize>, note: &str, jump: bool| Step { tok: lab.clone(), path: path.to_string(), name: leaf(path).to_string(), kind: kind.into(), idx, jump, note: note.into(), via: vec![], total: 0, alts: 0 };
        if k == 1 {
            if raw.starts_with('%') && raw.ends_with('%') && raw.len() > 2 {
                if let Some(p) = env.vars.get(&raw[1..raw.len() - 1]) {
                    if std::path::Path::new(p).is_dir() { add(&mut first, p.clone(), 1200, mk(p, "env var", vec![], "", false), true); }
                }
            } else if raw == "~" {
                add(&mut first, env.home.to_string(), 1150, mk(env.home, "home", vec![], "", false), true);
            } else {
                if let Some((_, p)) = env.aliases.iter().find(|(a, _)| *a == raw) {
                    if std::path::Path::new(p).is_dir() { add(&mut first, p.clone(), 1200, mk(p, "alias", vec![], "", false), true); }
                }
                let r = raw.trim_end_matches(':');
                if r.len() == 1 && r.as_bytes()[0].is_ascii_alphabetic() {
                    let dp = format!("{}:\\", r.to_uppercase());
                    if env.drives.contains(&dp) { add(&mut first, dp.clone(), if raw.ends_with(':') || n > 1 { 1100 } else { 870 }, mk(&dp, "drive", vec![], "", false), true); }
                }
            }
        }
        if tok.is_empty() { continue; }
        for (base, bonus, note) in [(env.cwd, 70, "in this folder"), (env.home, 25, "under home")] {
            for (name, dir) in list(base, &mut cache) {
                if !dir && !last { continue; }
                if let Some(m) = match_name(&tok, &name) {
                    let p = join(base, &name);
                    let st = Step { tok: lab.clone(), path: p.clone(), name: name.clone(), kind: m.kind.label().into(), idx: m.idx, jump: false, note: note.into(), via: vec![], total: 0, alts: 0 };
                    add(&mut first, p, m.score + bonus + (k as i32 - 1) * 40, st, dir);
                }
            }
        }
        let tokstr: String = tok.iter().collect();
        for d in crate::core::index::lookup(env.index, &tokstr, None, None, Some(30)) {
            if let Some(m) = match_name(&tok, &d.name) {
                if m.score >= 640 {
                    let st = Step { tok: lab.clone(), path: d.path.clone(), name: d.name.clone(), kind: m.kind.label().into(), idx: m.idx, jump: true, note: String::new(), via: vec![], total: 0, alts: 0 };
                    let sc = m.score - 45 - d.depth as i32 * 5 + (env.boost)(&d.path) as i32 + (k as i32 - 1) * 40;
                    add(&mut first, d.path, sc, st, true);
                }
            }
        }
    }
    let mut seeds: Vec<(String, (i32, Step, bool))> = first.into_iter().collect();
    seeds.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    for (path, (score, st, dir)) in seeds.into_iter().take(12) {
        let k = st.tok.split(' ').count();
        buckets[k].push(State { path, i: k, score, dir, steps: vec![st] });
    }

    /* step 2+: beam search down the tree */
    let mut done: Vec<State> = Vec::new();
    let mut stuck: Vec<State> = Vec::new();
    for i in 1..=n {
        let mut b = std::mem::take(&mut buckets[i]);
        b.sort_by(|x, y| y.score.cmp(&x.score));
        let mut seen = HashSet::new();
        let front: Vec<State> = b.into_iter().filter(|s| seen.insert(s.path.clone())).take(10).collect();
        if i == n { done.extend(front); continue; }
        for s in front {
            if !s.dir { continue; }
            let ents = list(&s.path, &mut cache);
            let mut any = false;
            for k in 1..=(n - i).min(3) {
                let tok = jt(i, k);
                if tok.is_empty() { continue; }
                let last = i + k == n;
                let mut ranked: Vec<(String, bool, crate::matching::Match)> = ents.iter().filter(|(_, d)| *d || last).filter_map(|(nm, d)| match_name(&tok, nm).map(|m| (nm.clone(), *d, m))).collect();
                ranked.sort_by(|a, b| b.2.score.cmp(&a.2.score).then(a.0.len().cmp(&b.0.len())));
                let best = ranked.first().map(|r| r.2.score).unwrap_or(0);
                if k > 1 && best < 700 { continue; }
                let alts = ranked.len();
                for (nm, d, m) in ranked.into_iter().take(4) {
                    if m.score < best - 250 { break; }
                    any = true;
                    let p = join(&s.path, &nm);
                    let mut steps = s.steps.clone();
                    steps.push(Step { tok: label(i, k), path: p.clone(), name: nm, kind: m.kind.label().into(), idx: m.idx, jump: false, note: String::new(), via: vec![], total: ents.len(), alts });
                    buckets[i + k].push(State { path: p, i: i + k, score: s.score + m.score + (k as i32 - 1) * 40, dir: d, steps });
                }
                if k == 1 && best < 500 {
                    // nothing good directly below: ask the index for matches a few levels down
                    let tokstr: String = tok.iter().collect();
                    let mut hits: Vec<(String, String, Vec<String>, i32, i32)> = Vec::new(); // name, path, via, score, depth
                    for x in crate::core::index::lookup(env.index, &tokstr, Some(s.path.clone()), Some(4), Some(12)) {
                        if let Some(m) = match_name(&tok, &x.name) {
                            if m.score >= 600 { hits.push((x.name.clone(), x.path.clone(), x.via.clone(), m.score, x.via.len() as i32 + 1)); }
                        }
                    }
                    if hits.is_empty() {
                        for (en, ed) in &ents {
                            if !*ed || budget <= 0 { continue; }
                            budget -= 1;
                            let p1 = join(&s.path, en);
                            for (nm, d) in list(&p1, &mut cache) {
                                if !d && !last { continue; }
                                if let Some(m) = match_name(&tok, &nm) { if m.score >= 600 { hits.push((nm.clone(), join(&p1, &nm), vec![en.clone()], m.score, 2)); } }
                            }
                        }
                    }
                    hits.sort_by(|a, b| (b.3 - b.4 * 90).cmp(&(a.3 - a.4 * 90)));
                    for (nm, p, via, sc, d) in hits.into_iter().take(3) {
                        any = true;
                        let m = match_name(&tok, &nm);
                        let mut steps = s.steps.clone();
                        steps.push(Step { tok: label(i, 1), path: p.clone(), name: nm, kind: m.as_ref().map(|m| m.kind.label()).unwrap_or("").into(), idx: m.map(|m| m.idx).unwrap_or_default(), jump: false, note: String::new(), via, total: ents.len(), alts: 0 });
                        let isdir = std::path::Path::new(&p).is_dir();
                        buckets[i + 1].push(State { path: p, i: i + 1, score: s.score + sc - (d - 1) * 90, dir: isdir, steps });
                    }
                }
            }
            if !any { stuck.push(s); }
        }
    }

    /* finish: frecency tie-break, lone-folder auto-open, counts */
    let finish = |s: &State, partial: bool| -> Res {
        let mut r = Res { steps: s.steps.clone(), lone: vec![], final_path: s.path.clone(), dir: s.path.clone(), file: None, counts: None, partial,
                          unresolved: if partial { toks[s.i..].join(" ") } else { String::new() }, score: s.score as f64 + (env.boost)(&s.path) };
        if !s.dir { r.file = Some(leaf(&s.path).to_string()); r.dir = parent(&s.path).unwrap_or_default(); return r; }
        let mut pk = fsops::peek(s.path.clone());
        while env.lone && r.lone.len() < 6 {
            let Some(only) = pk.only.clone() else { break };
            let p = join(&r.dir, &only);
            r.lone.push((only, p.clone()));
            r.dir = p.clone();
            pk = fsops::peek(p.clone());
        }
        r.counts = Some((pk.dirs, pk.files));
        r
    };
    let mut ranked: Vec<(f64, State)> = done.into_iter().map(|s| (s.score as f64 + (env.boost)(&s.path), s)).collect();
    ranked.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let (mut out, mut seen) = (Vec::new(), HashSet::new());
    for (_, s) in &ranked {
        if !seen.insert(s.path.to_lowercase()) { continue; }
        out.push(finish(s, false));
        if out.len() >= 6 { break; }
    }
    if out.is_empty() {
        stuck.sort_by(|a, b| b.i.cmp(&a.i).then(b.score.cmp(&a.score)));
        for s in stuck.iter().take(3) { out.push(finish(s, true)); }
    }
    let _ = Kind::Exact;
    Resolution { results: out, ms: t0.elapsed().as_secs_f64() * 1000.0, scanned }
}
