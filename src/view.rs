//! How a folder is shown: rows or tiles, thumbnails, folder sizes and the details pane.
//! Everything slow (shell thumbnails, adding up folders, reading files) runs on small worker pools and reports back
//! through `post`, so the window never waits on the disk.
use crate::core::{fsops, media};
use crate::util::*;
use crate::{key_at, post, with, App, Entry, InfoRow, State, TileItem, TileRow};
use slint::{ComponentHandle, Image, Model, ModelRc, Rgba8Pixel, SharedPixelBuffer, SharedString, VecModel};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex};

/// A tiny worker pool that runs the newest request first, so scrolling fast never waits on rows already gone by.
pub struct Pool<J: Send + 'static> { q: Arc<(Mutex<Vec<J>>, Condvar)> }
impl<J: Send + 'static> Pool<J> {
    pub fn new(n: usize, work: impl Fn(J) + Send + Sync + 'static) -> Self {
        let q: Arc<(Mutex<Vec<J>>, Condvar)> = Arc::new((Mutex::new(Vec::new()), Condvar::new()));
        let work = Arc::new(work);
        for _ in 0..n {
            let (q, work) = (q.clone(), work.clone());
            std::thread::spawn(move || loop {
                let job = {
                    let (m, cv) = &*q;
                    let mut g = m.lock().unwrap();
                    while g.is_empty() { g = cv.wait(g).unwrap(); }
                    g.pop().unwrap()
                };
                work(job);
            });
        }
        Pool { q }
    }
    pub fn push(&self, j: J) {
        let (m, cv) = &*self.q;
        let mut g = m.lock().unwrap();
        g.push(j);
        if g.len() > 160 { g.remove(0); }
        cv.notify_one();
    }
    pub fn clear(&self) { self.q.0.lock().unwrap().clear(); }
}

pub struct Media {
    pub tiles: bool,
    pub cols: usize,
    pub tile_model: Rc<VecModel<TileRow>>,
    pub thumbs: HashMap<String, Option<Image>>,
    pub thumb_pending: HashSet<String>,
    pub sizes: HashMap<String, (u64, bool, i64)>,
    pub size_pending: HashSet<String>,
    pub tpool: Pool<(String, String, u32)>,   // key, path, px
    pub spool: Pool<String>,                  // folder path
    pub pane_timer: slint::Timer,
    pub pane_seq: u32,
    pub pane_rows: Vec<(String, String)>,
}

impl Media {
    pub fn new() -> Media {
        let tpool = Pool::new(3, |(key, path, px): (String, String, u32)| {
            let img = media::thumb(&path, px);
            post(move |ui, st| thumb_done(ui, st, key, path, img));
        });
        let spool = Pool::new(2, |path: String| {
            let r = media::folder_size(path.clone(), Some(1500)).ok();
            post(move |ui, st| size_done(ui, st, path, r));
        });
        Media { tiles: false, cols: 1, tile_model: Rc::new(VecModel::default()), thumbs: HashMap::new(), thumb_pending: HashSet::new(), sizes: HashMap::new(), size_pending: HashSet::new(), tpool, spool, pane_timer: slint::Timer::default(), pane_seq: 0, pane_rows: vec![] }
    }
}

// (width, height, picture height) for small / medium / large tiles
pub const TILES: [(f32, f32, f32); 3] = [(116.0, 150.0, 84.0), (152.0, 192.0, 124.0), (208.0, 252.0, 180.0)];
pub fn tile_idx(st: &State) -> usize { match st.cfg.opts.tile.as_str() { "s" => 0, "l" => 2, _ => 1 } }

/// pictures must be a real share of the folder, or fifty folders and a few stray images would become a wall of icons
fn auto_view(st: &State) -> bool {
    let (mut n, mut vis, mut dirs) = (0usize, 0usize, 0usize);
    for e in &st.entries {
        if !st.cfg.opts.hidden && e.hidden { continue; }
        n += 1;
        if e.dir { dirs += 1; } else if crate::kinds::is_visual(&e.name) { vis += 1; }
    }
    n >= 4 && vis >= 3 && (vis as f32 / n as f32 >= 0.35 || ((vis + dirs) as f32 / n as f32 >= 0.6 && vis as f32 / n as f32 >= 0.15))
}
pub fn view_pin(st: &State) -> i32 { match st.cfg.views.get(&st.cwd.to_lowercase()).map(|s| s.as_str()) { Some("rows") => 1, Some("tiles") => 2, _ => 0 } }
pub fn wants_tiles(st: &State) -> bool {
    let pick = st.cfg.views.get(&st.cwd.to_lowercase()).cloned().unwrap_or_else(|| st.cfg.opts.view.clone());
    match pick.as_str() { "rows" => false, "tiles" => true, _ => auto_view(st) }
}

// ------------------------------------------------------------ thumbnails and sizes

fn thumb_wants(e: &Entry) -> bool {
    if e.dir || e.size >= 4_000_000_000 { return false; }
    let n = e.name.to_lowercase();
    ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "ico", "mp4", "mkv", "mov", "webm", "avi", "exe", "msi", "lnk", "pdf"].iter().any(|x| n.ends_with(&format!(".{x}")))
}
fn tkey(st: &State, e: &Entry, px: u32) -> String { format!("{}|{}|{}", join(&st.cwd, &e.name).to_lowercase(), e.mtime, px) }
pub fn thumb_for(st: &State, e: &Entry, px: u32) -> Option<Image> {
    if !thumb_wants(e) { return None; }
    st.media.thumbs.get(&tkey(st, e, px)).cloned().flatten()
}
pub fn dir_size_text(st: &State, e: &Entry) -> String {
    if !e.dir || !st.cfg.opts.folder_sizes { return String::new(); }
    match st.media.sizes.get(&join(&st.cwd, &e.name).to_lowercase()) { Some((s, done, _)) => format!("{}{}", if *done { "" } else { "≥ " }, fmt_size(*s)), None => String::new() }
}
pub fn size_of(st: &State, e: &Entry) -> u64 {
    if e.dir { st.media.sizes.get(&join(&st.cwd, &e.name).to_lowercase()).map(|x| x.0).unwrap_or(0) } else { e.size }
}

fn thumb_done(ui: &App, st: &mut State, key: String, path: String, img: Option<media::Rgba>) {
    st.media.thumb_pending.remove(&key);
    let image = img.map(|r| Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&r.data, r.w, r.h)));
    st.media.thumbs.insert(key, image);
    if st.media.thumbs.len() > 800 { let k = st.media.thumbs.keys().next().cloned(); if let Some(k) = k { st.media.thumbs.remove(&k); } }
    if let Some(name) = path.rsplit('\\').next() { if parent(&path).map(|p| eq(&p, &st.cwd)).unwrap_or(false) { refresh_item(ui, st, &name.to_lowercase()); } }
}
fn size_done(ui: &App, st: &mut State, path: String, r: Option<media::Sizes>) {
    let k = path.to_lowercase();
    st.media.size_pending.remove(&k);
    if let Some(r) = r { st.media.sizes.insert(k.clone(), (r.size, r.complete, now_ms())); }
    if parent(&path).map(|p| eq(&p, &st.cwd)).unwrap_or(false) {
        if st.sort_key == 3 && st.media.size_pending.is_empty() { crate::build_view(st); crate::push_rows(ui, st); } else { refresh_item(ui, st, &leaf(&path).to_lowercase()); }
        crate::push_status(ui, st);
    }
}

/// ask for whatever is on screen and not known yet
pub fn request_visible(ui: &App, st: &mut State) {
    let n = st.view.len();
    if n == 0 { return; }
    let (y, h) = (-ui.get_list_y(), ui.get_list_h().max(300.0));
    let (rh, cols) = if st.media.tiles { (TILES[tile_idx(st)].1, st.media.cols.max(1)) } else { (30.0, 1) };
    let first = ((y / rh) as usize).saturating_sub(1) * cols;
    let last = ((((y + h) / rh) as usize) + 2) * cols;
    let px = if st.media.tiles { 256 } else { 40 };
    for p in first..last.min(n) {
        let e = &st.entries[st.view[p]];
        let path = join(&st.cwd, &e.name);
        if thumb_wants(e) {
            let key = tkey(st, e, px);
            if !st.media.thumbs.contains_key(&key) && st.media.thumb_pending.insert(key.clone()) { st.media.tpool.push((key, path.clone(), px)); }
        } else if e.dir && st.cfg.opts.folder_sizes {
            let k = path.to_lowercase();
            let stale = st.media.sizes.get(&k).map(|x| now_ms() - x.2 > 120_000).unwrap_or(true);
            if stale && st.media.size_pending.insert(k) { st.media.spool.push(path); }
        }
    }
}

// ------------------------------------------------------------ the list: rows or tiles

fn tile_item(st: &State, p: usize) -> TileItem {
    let e = &st.entries[st.view[p]];
    let img = thumb_for(st, e, 256);
    let ext = if e.dir { String::new() } else { match e.name.rfind('.') { Some(i) if i > 0 => e.name[i + 1..].chars().take(5).collect::<String>().to_uppercase(), _ => String::new() } };
    TileItem {
        name: e.name.as_str().into(), dir: e.dir,
        size: (if e.dir { dir_size_text(st, e) } else { fmt_size(e.size) }).into(), ext: ext.into(),
        selected: st.sel.contains(&e.k), hidden: e.hidden,
        cut: st.clip.as_ref().map(|c| c.cut && c.paths.iter().any(|q| eq(q, &join(&st.cwd, &e.name)))).unwrap_or(false),
        renaming: false, has_thumb: img.is_some(), thumb: img.unwrap_or_default(),
    }
}
fn tile_row(st: &State, r: usize) -> TileRow {
    let cols = st.media.cols.max(1);
    let items: Vec<TileItem> = (r * cols..((r + 1) * cols).min(st.view.len())).map(|p| tile_item(st, p)).collect();
    TileRow { items: ModelRc::new(VecModel::from(items)) }
}

/// decide rows vs tiles, size the cells, and (re)build whichever model is showing
pub fn push_list(ui: &App, st: &mut State) {
    let tiles = wants_tiles(st);
    st.media.tiles = tiles;
    ui.set_tiles(tiles);
    ui.set_view_pin(view_pin(st));
    ui.set_tile_size(tile_idx(st) as i32);
    if tiles {
        let (w, h, th) = TILES[tile_idx(st)];
        ui.set_tile_w(w); ui.set_tile_h(h); ui.set_tile_th(th);
        let lw = ui.get_list_w();
        let cw = if lw > 0.0 { lw } else { 700.0 };
        st.media.cols = (((cw - 8.0) / w) as usize).max(1);
        let rows: Vec<TileRow> = (0..st.view.len().div_ceil(st.media.cols)).map(|r| tile_row(st, r)).collect();
        st.media.tile_model = Rc::new(VecModel::from(rows));
        ui.set_tile_rows(ModelRc::from(st.media.tile_model.clone()));
        ui.set_entries(ModelRc::new(VecModel::from(Vec::<crate::EntryRow>::new())));
    } else {
        st.media.cols = 1;
        let rows: Vec<crate::EntryRow> = (0..st.view.len()).map(|p| crate::row_of(st, p)).collect();
        st.rows = Rc::new(VecModel::from(rows));
        ui.set_entries(ModelRc::from(st.rows.clone()));
        ui.set_tile_rows(ModelRc::new(VecModel::from(Vec::<TileRow>::new())));
    }
    ui.set_empty_text(if st.filter.is_empty() { "empty folder".into() } else { format!("nothing matches \"{}\"", st.filter).into() });
    ui.set_has_pane(st.cfg.opts.pane);
}

/// the window got wider or narrower: tiles may need a different number of columns
pub fn on_layout(ui: &App, st: &mut State) {
    if !st.media.tiles { return; }
    let (w, _, _) = TILES[tile_idx(st)];
    let cols = (((ui.get_list_w() - 8.0) / w) as usize).max(1);
    if cols != st.media.cols { push_list(ui, st); }
}

/// repaint one entry (a thumbnail or size arrived, or its selection changed)
pub fn refresh_item(ui: &App, st: &State, key: &str) {
    let Some(&p) = st.pos.get(key) else { return };
    let _ = ui;
    if st.media.tiles {
        let r = p / st.media.cols.max(1);
        if r < st.media.tile_model.row_count() { st.media.tile_model.set_row_data(r, tile_row(st, r)); }
    } else if p < st.rows.row_count() {
        st.rows.set_row_data(p, crate::row_of(st, p));
    }
}

pub fn row_height(st: &State) -> f32 { if st.media.tiles { TILES[tile_idx(st)].1 } else { 30.0 } }

// ------------------------------------------------------------ the details pane

pub fn schedule_pane(st: &mut State) {
    st.media.pane_timer.start(slint::TimerMode::SingleShot, std::time::Duration::from_millis(70), || { with(|ui, st| update_pane(ui, st)); });
}

fn set_pane_rows(ui: &App, rows: &[(String, String)]) {
    let v: Vec<InfoRow> = rows.iter().map(|(k, v)| InfoRow { k: k.as_str().into(), v: v.replace('\\', "\\\u{200B}").into() }).collect();
    ui.set_pane_rows(ModelRc::new(VecModel::from(v)));
}

pub fn update_pane(ui: &App, st: &mut State) {
    if !st.cfg.opts.pane { return; }
    st.media.pane_seq += 1;
    let seq = st.media.pane_seq;
    ui.set_pane_has_image(false);
    ui.set_pane_text(SharedString::new());
    let sel: Vec<usize> = { let mut v: Vec<usize> = st.sel.iter().filter_map(|k| st.pos.get(k).copied()).collect(); v.sort(); v };
    if sel.len() == 1 {
        let e = &st.entries[st.view[sel[0]]];
        let path = join(&st.cwd, &e.name);
        ui.set_pane_title(e.name.as_str().into());
        ui.set_pane_sub(format!("{} · {}", e.kind, e.cat).into());
        let mut rows = vec![("size".to_string(), if e.dir { dir_size_text(st, e) } else { format!("{} · {} bytes", fmt_size(e.size), e.size) }), ("modified".into(), fmt_date(e.mtime))];
        rows.retain(|r| !r.1.is_empty());
        st.media.pane_rows = rows.clone();
        set_pane_rows(ui, &rows);
        let (dir, name, size, wants) = (e.dir, e.name.clone(), e.size, thumb_wants(e));
        std::thread::spawn(move || {
            if dir {
                let names: Vec<String> = std::fs::read_dir(&path).map(|rd| rd.flatten().take(400).map(|x| (x.file_type().map(|t| t.is_dir()).unwrap_or(false), x.file_name().to_string_lossy().into_owned())).collect::<Vec<_>>()).unwrap_or_default().into_iter().take(10).map(|(d, n)| format!("{}{}", if d { "▸ " } else { "  " }, n)).collect();
                let count = std::fs::read_dir(&path).map(|rd| rd.count()).unwrap_or(0);
                post(move |ui, st| { if st.media.pane_seq == seq { ui.set_pane_text(names.join("\n").into()); st.media.pane_rows.push(("items".into(), count.to_string())); let r = st.media.pane_rows.clone(); set_pane_rows(ui, &r); } });
                return;
            }
            if let Ok(info) = media::file_info(path.clone()) {
                let mut extra: Vec<(String, String)> = vec![("created".into(), fmt_date(info.created)), ("accessed".into(), fmt_date(info.accessed))];
                let mut attrs = Vec::new(); if info.readonly { attrs.push("read-only"); } if info.hidden { attrs.push("hidden"); }
                if !attrs.is_empty() { extra.push(("attributes".into(), attrs.join(", "))); }
                if let Some(d) = &info.image { extra.push(("dimensions".into(), format!("{} × {}", d.w, d.h))); }
                if let Some(t) = &info.text { extra.push(("text".into(), format!("{} lines · {} words", t.lines, t.words))); extra.push(("encoding".into(), format!("{} · {}", t.encoding, if t.crlf { "CRLF" } else { "LF" }))); }
                extra.push(("location".into(), parent(&path).unwrap_or_default()));
                post(move |ui, st| { if st.media.pane_seq == seq { st.media.pane_rows.extend(extra); let r = st.media.pane_rows.clone(); set_pane_rows(ui, &r); } });
            }
            if wants {
                if let Some(img) = media::thumb(&path, 480) {
                    post(move |ui, st| { if st.media.pane_seq == seq { ui.set_pane_image(Image::from_rgba8(SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(&img.data, img.w, img.h))); ui.set_pane_has_image(true); } });
                }
            } else if crate::kinds::can_edit(&name, size) && size > 0 {
                { let pv = fsops::preview(path); if let Some(t) = pv.text { let txt = t.lines().take(16).collect::<Vec<_>>().join("\n"); post(move |ui, st| { if st.media.pane_seq == seq { ui.set_pane_text(txt.into()); } }); } }
            }
        });
    } else if sel.len() > 1 {
        let total: u64 = sel.iter().map(|&p| size_of(st, &st.entries[st.view[p]])).sum();
        let dirs = sel.iter().filter(|&&p| st.entries[st.view[p]].dir).count();
        ui.set_pane_title(format!("{} selected", sel.len()).into());
        ui.set_pane_sub(format!("{} folders, {} files", dirs, sel.len() - dirs).into());
        set_pane_rows(ui, &[("total".to_string(), fmt_size(total))]);
    } else {
        ui.set_pane_title(if leaf(&st.cwd).is_empty() { st.cwd.as_str().into() } else { leaf(&st.cwd).into() });
        ui.set_pane_sub("Folder".into());
        set_pane_rows(ui, &[("items".to_string(), st.entries.len().to_string()), ("path".into(), st.cwd.clone())]);
    }
}

pub fn pane_action(ui: &App, st: &mut State, a: &str) {
    let Some(&p0) = st.sel.iter().filter_map(|k| st.pos.get(k)).min() else { return };
    let e = &st.entries[st.view[p0]];
    let path = join(&st.cwd, &e.name);
    match a {
        "open" => { crate::open_row(ui, st, p0); }
        "hash" if !e.dir => {
            let seq = st.media.pane_seq;
            std::thread::spawn(move || {
                let h = media::file_hash(path).unwrap_or_else(|e| format!("error: {e}"));
                post(move |ui, st| { if st.media.pane_seq == seq { st.media.pane_rows.push(("sha-256".into(), h)); let r = st.media.pane_rows.clone(); set_pane_rows(ui, &r); } });
            });
        }
        _ => {}
    }
}

pub fn _unused(_: &HashSet<String>) { let _ = key_at; }
