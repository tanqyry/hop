//! One watcher, on the folder you are looking at. Changes are debounced into a single callback.
use notify::{RecursiveMode, Watcher as _};
use std::sync::{mpsc, Mutex};
use std::time::Duration;

#[derive(Default)]
pub struct Watcher(Mutex<Option<notify::RecommendedWatcher>>);

impl Watcher {
    pub fn watch(&self, path: &str, on_change: impl Fn() + Send + 'static) -> Result<(), String> {
        let (tx, rx) = mpsc::channel::<()>();
        let mut w = notify::recommended_watcher(move |r: notify::Result<notify::Event>| {
            if r.is_ok() {
                let _ = tx.send(());
            }
        })
        .map_err(|e| e.to_string())?;
        w.watch(std::path::Path::new(path), RecursiveMode::NonRecursive).map_err(|e| e.to_string())?;
        std::thread::spawn(move || {
            // ends on its own when the watcher (and with it `tx`) is dropped
            while rx.recv().is_ok() {
                while rx.recv_timeout(Duration::from_millis(150)).is_ok() {}
                on_change();
            }
        });
        *self.0.lock().unwrap() = Some(w);
        Ok(())
    }
}
