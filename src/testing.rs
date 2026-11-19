//! `hop.exe --script steps.txt` drives the app with synthetic events and takes snapshots of its own window,
//! so behaviour can be tested without touching the OS keyboard or screen.
//!   wait 500          pause
//!   key ctrl+l        press a key (Down, Up, Left, Right, Return, Escape, Backspace, Home, End, PageUp, PageDown, F5, ...)
//!   type appdata      type text
//!   click 400 300     left click at window coordinates
//!   shot name         save <name>.png next to the script
//!   quit
use crate::App;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, SharedString};

fn named(n: &str) -> Option<SharedString> {
    let k = match n.to_lowercase().as_str() {
        "down" => Key::DownArrow, "up" => Key::UpArrow, "left" => Key::LeftArrow, "right" => Key::RightArrow,
        "return" | "enter" => Key::Return, "escape" | "esc" => Key::Escape, "backspace" => Key::Backspace, "delete" => Key::Delete,
        "home" => Key::Home, "end" => Key::End, "pageup" => Key::PageUp, "pagedown" => Key::PageDown, "tab" => Key::Tab,
        "f2" => Key::F2, "f5" => Key::F5, _ => return None,
    };
    Some(k.into())
}

fn on_ui(weak: &slint::Weak<App>, f: impl FnOnce(&App) + Send + 'static) {
    let w = weak.clone();
    let _ = slint::invoke_from_event_loop(move || { if let Some(ui) = w.upgrade() { f(&ui); } });
}

pub fn run(ui: &App, script: &str) {
    let lines: Vec<String> = std::fs::read_to_string(script).unwrap_or_default().lines().map(String::from).collect();
    let dir = std::path::Path::new(script).parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let weak = ui.as_weak();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(900));
        for line in lines {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            let (cmd, arg) = line.split_once(' ').unwrap_or((line, ""));
            match cmd {
                "wait" => std::thread::sleep(std::time::Duration::from_millis(arg.parse().unwrap_or(200))),
                "key" => {
                    let parts: Vec<&str> = arg.split('+').collect();
                    let key = parts.last().copied().unwrap_or("");
                    let mods: Vec<SharedString> = parts[..parts.len().saturating_sub(1)].iter().filter_map(|m| match m.to_lowercase().as_str() { "ctrl" => Some(Key::Control.into()), "shift" => Some(Key::Shift.into()), "alt" => Some(Key::Alt.into()), _ => None }).collect();
                    let text = named(key).unwrap_or_else(|| key.into());
                    let (m2, t2) = (mods.clone(), text.clone());
                    on_ui(&weak, move |ui| { let w = ui.window(); for m in &m2 { w.dispatch_event(WindowEvent::KeyPressed { text: m.clone() }); } w.dispatch_event(WindowEvent::KeyPressed { text: t2.clone() }); w.dispatch_event(WindowEvent::KeyReleased { text: t2 }); for m in m2.iter().rev() { w.dispatch_event(WindowEvent::KeyReleased { text: m.clone() }); } });
                    std::thread::sleep(std::time::Duration::from_millis(120));
                }
                "type" => {
                    for ch in arg.chars() {
                        let t: SharedString = ch.to_string().into();
                        on_ui(&weak, move |ui| { let w = ui.window(); w.dispatch_event(WindowEvent::KeyPressed { text: t.clone() }); w.dispatch_event(WindowEvent::KeyReleased { text: t }); });
                        std::thread::sleep(std::time::Duration::from_millis(35));
                    }
                }
                "click" => {
                    let n: Vec<f32> = arg.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                    if n.len() == 2 {
                        let pos = LogicalPosition::new(n[0], n[1]);
                        on_ui(&weak, move |ui| { let w = ui.window(); w.dispatch_event(WindowEvent::PointerMoved { position: pos }); w.dispatch_event(WindowEvent::PointerPressed { position: pos, button: PointerEventButton::Left }); w.dispatch_event(WindowEvent::PointerReleased { position: pos, button: PointerEventButton::Left }); });
                    }
                    std::thread::sleep(std::time::Duration::from_millis(150));
                }
                "size" => {
                    let n: Vec<f32> = arg.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                    if n.len() == 2 { let (w, h) = (n[0], n[1]); on_ui(&weak, move |ui| { ui.window().set_size(slint::LogicalSize::new(w, h)); }); }
                    std::thread::sleep(std::time::Duration::from_millis(400));
                }
                "rclick" => {
                    let n: Vec<f32> = arg.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                    if n.len() == 2 {
                        let pos = LogicalPosition::new(n[0], n[1]);
                        on_ui(&weak, move |ui| { let w = ui.window(); w.dispatch_event(WindowEvent::PointerMoved { position: pos }); w.dispatch_event(WindowEvent::PointerPressed { position: pos, button: PointerEventButton::Right }); w.dispatch_event(WindowEvent::PointerReleased { position: pos, button: PointerEventButton::Right }); });
                    }
                    std::thread::sleep(std::time::Duration::from_millis(250));
                }
                "drag" => {
                    let n: Vec<f32> = arg.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                    if n.len() == 4 {
                        let (a, b) = (LogicalPosition::new(n[0], n[1]), LogicalPosition::new(n[2], n[3]));
                        on_ui(&weak, move |ui| { let w = ui.window(); w.dispatch_event(WindowEvent::PointerMoved { position: a }); w.dispatch_event(WindowEvent::PointerPressed { position: a, button: PointerEventButton::Left }); });
                        for k in 1..=8 { let f = k as f32 / 8.0; let p = LogicalPosition::new(a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f); on_ui(&weak, move |ui| { ui.window().dispatch_event(WindowEvent::PointerMoved { position: p }); }); std::thread::sleep(std::time::Duration::from_millis(40)); }
                        on_ui(&weak, move |ui| { ui.window().dispatch_event(WindowEvent::PointerReleased { position: b, button: PointerEventButton::Left }); });
                    }
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
                "shot" => {
                    let path = dir.join(format!("{arg}.png"));
                    on_ui(&weak, move |ui| {
                        if let Ok(buf) = ui.window().take_snapshot() {
                            if let Ok(f) = std::fs::File::create(&path) {
                                let mut enc = png::Encoder::new(std::io::BufWriter::new(f), buf.width(), buf.height());
                                enc.set_color(png::ColorType::Rgba);
                                enc.set_depth(png::BitDepth::Eight);
                                if let Ok(mut w) = enc.write_header() { let _ = w.write_image_data(buf.as_bytes()); }
                            }
                        }
                    });
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
                "quit" => { let _ = slint::invoke_from_event_loop(|| { let _ = slint::quit_event_loop(); }); }
                _ => {}
            }
        }
    });
}
