//! File operations. Deletes go to the Recycle Bin, so everything here can be undone.
use crate::core::fsops;
use crate::util::*;
use crate::{go, open_row, push_status, push_rows, refresh, set_selection, toast, App, Hist, State};
use slint::ComponentHandle;
use std::collections::HashSet;

pub enum Undo {
    Rename { from: String, to: String },   // `from` is the current path, `to` is what it was called
    Create(String),
    Delete(Vec<String>),
    Paste(Vec<String>),
    Move(Vec<(String, String)>),           // (original, now)
}

pub struct Clip { pub cut: bool, pub paths: Vec<String> }

pub fn sel_paths(st: &State) -> Vec<String> {
    let mut v: Vec<(usize, String)> = st.sel.iter().filter_map(|k| st.pos.get(k).map(|&p| (p, join(&st.cwd, &st.entries[st.view[p]].name)))).collect();
    v.sort_by_key(|x| x.0);
    v.into_iter().map(|x| x.1).collect()
}

fn plural(n: usize, w: &str) -> String { format!("{} {}{}", n, w, if n == 1 { "" } else { "s" }) }

/// show a message in the status bar for a few seconds
pub fn notify(ui: &App, st: &mut State, msg: &str) {
    toast(ui, st, msg);
    slint::Timer::single_shot(std::time::Duration::from_millis(4500), || { crate::with(|ui, st| { st.toast.clear(); push_status(ui, st); }); });
}

pub fn select_names(ui: &App, st: &mut State, names: &[String]) {
    let keys: HashSet<String> = names.iter().map(|n| n.to_lowercase()).filter(|k| st.pos.contains_key(k)).collect();
    let first = keys.iter().filter_map(|k| st.pos.get(k)).min().copied();
    set_selection(ui, st, keys, first, first);
}

pub fn delete(ui: &App, st: &mut State) {
    let paths = sel_paths(st);
    if paths.is_empty() { return; }
    match fsops::delete_items(paths.clone(), &st.index) {
        Ok(()) => {
            st.undo.push(Undo::Delete(paths.clone()));
            st.sel.clear();
            refresh(ui, st);
            notify(ui, st, &format!("Deleted {} · Ctrl+Z to undo", if paths.len() == 1 { format!("\"{}\"", leaf(&paths[0])) } else { plural(paths.len(), "item") }));
        }
        Err(e) => notify(ui, st, &format!("Couldn't delete: {e}")),
    }
}

pub fn copy(ui: &App, st: &mut State, cut: bool) {
    let paths = sel_paths(st);
    if paths.is_empty() { return; }
    let n = paths.len();
    st.clip = Some(Clip { cut, paths });
    push_rows(ui, st);
    notify(ui, st, &format!("{} {}", if cut { "Cut" } else { "Copied" }, plural(n, "item")));
}

pub fn paste(ui: &App, st: &mut State) {
    let Some(c) = st.clip.take() else { return };
    if !c.cut {
        match fsops::copy_items(c.paths.clone(), st.cwd.clone(), &st.index) {
            Ok(made) => {
                st.undo.push(Undo::Paste(made.clone()));
                st.clip = Some(Clip { cut: false, paths: c.paths });
                refresh(ui, st);
                select_names(ui, st, &made.iter().map(|p| leaf(p).to_string()).collect::<Vec<_>>());
                notify(ui, st, &format!("Pasted {}", plural(made.len(), "item")));
            }
            Err(e) => { st.clip = Some(c); notify(ui, st, &format!("Couldn't paste: {e}")); }
        }
    } else {
        let here = st.cwd.to_lowercase();
        let srcs: Vec<String> = c.paths.iter().filter(|p| parent(p).map(|q| q.to_lowercase()) != Some(here.clone())).cloned().collect();
        if srcs.is_empty() { push_rows(ui, st); return; }
        match fsops::move_items(srcs, st.cwd.clone(), &st.index) {
            Ok(moved) => {
                st.undo.push(Undo::Move(moved.iter().map(|m| (m.from.clone(), m.to.clone())).collect()));
                refresh(ui, st);
                select_names(ui, st, &moved.iter().map(|m| leaf(&m.to).to_string()).collect::<Vec<_>>());
                notify(ui, st, &format!("Moved {}", plural(moved.len(), "item")));
            }
            Err(e) => { st.clip = Some(c); notify(ui, st, &format!("Couldn't move: {e}")); }
        }
    }
}

pub fn undo(ui: &App, st: &mut State) {
    let Some(u) = st.undo.pop() else { return notify(ui, st, "Nothing to undo") };
    let res: Result<(), String> = match u {
        Undo::Delete(paths) => fsops::restore_items(paths),
        Undo::Create(p) => fsops::delete_items(vec![p], &st.index),
        Undo::Paste(made) => fsops::delete_items(made, &st.index),
        Undo::Rename { from, to } => fsops::rename_item(from, leaf(&to).to_string(), &st.index).map(|_| ()),
        Undo::Move(moves) => {
            let mut r = Ok(());
            for (orig, now) in moves.into_iter().rev() {
                match fsops::move_items(vec![now], parent(&orig).unwrap_or_default(), &st.index) { Ok(_) => {} Err(e) => { r = Err(e); break; } }
            }
            r
        }
    };
    match res {
        Ok(()) => { refresh(ui, st); notify(ui, st, "Undone"); }
        Err(e) => notify(ui, st, &format!("Couldn't undo: {e}")),
    }
}

fn uniq_name(st: &State, base: &str) -> String {
    let has = |n: &str| st.entries.iter().any(|e| e.k == n.to_lowercase());
    if !has(base) { return base.to_string(); }
    let (stem, ext) = match base.rfind('.') { Some(i) if i > 0 => (&base[..i], &base[i..]), _ => (base, "") };
    (2..).map(|i| format!("{stem} ({i}){ext}")).find(|n| !has(n)).unwrap()
}

pub fn new_item(ui: &App, st: &mut State, dir: bool) {
    let name = uniq_name(st, if dir { "New folder" } else { "New file.txt" });
    match fsops::create_item(st.cwd.clone(), name.clone(), dir, &st.index) {
        Ok(p) => {
            st.undo.push(Undo::Create(p));
            refresh(ui, st);
            select_names(ui, st, &[name]);
            start_rename(ui, st);
        }
        Err(e) => notify(ui, st, &format!("Couldn't create: {e}")),
    }
}

pub fn start_rename(ui: &App, st: &mut State) {
    let Some(p) = st.cur.filter(|&p| p < st.view.len()) else { return };
    let e = &st.entries[st.view[p]];
    st.renaming = Some(e.k.clone());
    ui.set_rename_text(e.name.as_str().into());
    push_rows(ui, st);
}

pub fn rename_cancel(ui: &App, st: &mut State) {
    st.renaming = None;
    push_rows(ui, st);
    ui.invoke_focus_list();
}

pub fn rename_accept(ui: &App, st: &mut State) {
    let Some(k) = st.renaming.take() else { return };
    let new = ui.get_rename_text().to_string();
    let new = new.trim().to_string();
    let old = st.entries.iter().find(|e| e.k == k).map(|e| e.name.clone());
    if let Some(old) = old {
        if !new.is_empty() && new != old {
            if st.entries.iter().any(|e| e.k != k && e.k == new.to_lowercase()) {
                notify(ui, st, &format!("\"{new}\" already exists here"));
            } else {
                match fsops::rename_item(join(&st.cwd, &old), new.clone(), &st.index) {
                    Ok(np) => {
                        st.undo.push(Undo::Rename { from: np, to: join(&st.cwd, &old) });
                        refresh(ui, st);
                        select_names(ui, st, &[new]);
                    }
                    Err(e) => notify(ui, st, &format!("Couldn't rename: {e}")),
                }
            }
        }
    }
    push_rows(ui, st);
    ui.invoke_focus_list();
}

pub fn ctx_action(ui: &App, st: &mut State, a: &str) {
    match a {
        "open" => { if let Some(c) = st.cur { open_row(ui, st, c); } }
        "cut" => copy(ui, st, true),
        "copy" => copy(ui, st, false),
        "paste" => paste(ui, st),
        "rename" => start_rename(ui, st),
        "delete" => delete(ui, st),
        "newdir" => new_item(ui, st, true),
        "newfile" => new_item(ui, st, false),
        "undo" => undo(ui, st),
        "pin" => {
            let target = st.cur.and_then(|c| st.view.get(c).copied()).and_then(|i| st.entries.get(i)).filter(|e| e.dir).map(|e| join(&st.cwd, &e.name)).unwrap_or_else(|| st.cwd.clone());
            crate::toggle_pin(ui, st, &target);
        }
        _ => {}
    }
}

pub fn go_here(ui: &App, st: &mut State, p: &str) { go(ui, st, p, Hist::Push, None); }

// ---- drag rows onto a folder row (list view): dy is how far the pointer moved from the pressed row
fn drag_target(st: &State, i: usize, dy: f32) -> Option<usize> {
    let t = i as i64 + (dy / 30.0).round() as i64;
    if t < 0 || t as usize >= st.view.len() || t as usize == i { return None; }
    let e = st.entries.get(*st.view.get(t as usize)?)?;
    if !e.dir || st.sel.contains(&e.name.to_lowercase()) { return None; }
    Some(t as usize)
}

pub fn row_drag(ui: &App, st: &mut State, i: usize, dy: f32) {
    ui.set_drop_row(drag_target(st, i, dy).map(|t| t as i32).unwrap_or(-1));
}

pub fn row_drop(ui: &App, st: &mut State, i: usize, dy: f32) {
    ui.set_drop_row(-1);
    let Some(t) = drag_target(st, i, dy) else { return };
    let dest = join(&st.cwd, &st.entries[st.view[t]].name);
    let srcs = sel_paths(st);
    if srcs.is_empty() { return; }
    match fsops::move_items(srcs, dest.clone(), &st.index) {
        Ok(moved) => {
            st.undo.push(Undo::Move(moved.iter().map(|m| (m.from.clone(), m.to.clone())).collect()));
            refresh(ui, st);
            notify(ui, st, &format!("Moved {} into {}", plural(moved.len(), "item"), leaf(&dest)));
        }
        Err(e) => notify(ui, st, &format!("Couldn't move: {e}")),
    }
}
