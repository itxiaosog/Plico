use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::app::panel;
use crate::error::{Error, Result};
use crate::model::Snippet;
use crate::AppState;

pub fn default_spec() -> &'static str {
    if cfg!(target_os = "macos") {
        "Super+Shift+V"
    } else {
        "Ctrl+Shift+V"
    }
}

#[allow(dead_code)]
pub fn default_plain_spec() -> &'static str {
    if cfg!(target_os = "macos") {
        "Super+Shift+Alt+V"
    } else {
        "Ctrl+Shift+Alt+V"
    }
}

pub fn parse(spec: &str) -> Result<Shortcut> {
    Shortcut::from_str(spec).map_err(|e| Error::Other(format!("无法识别的快捷键「{spec}」：{e}")))
}

pub fn register(app: &AppHandle, spec: &str) -> Result<()> {
    let shortcut = parse(spec)?;
    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| Error::Other(format!("注册全局热键「{spec}」失败（可能已被其它程序占用）：{e}")))?;
    Ok(())
}

pub fn apply(app: &AppHandle, from: &str, to: &str) -> Result<()> {
    let next = parse(to)?;
    let shortcuts = app.global_shortcut();

    if !shortcuts.is_registered(next) {
        shortcuts
            .register(next)
            .map_err(|e| Error::Other(format!("快捷键「{to}」注册失败（可能已被其它程序占用）：{e}")))?;
    }

    if let Ok(old) = parse(from) {
        if old != next {
            let _ = shortcuts.unregister(old);
        }
    }

    Ok(())
}

#[allow(dead_code)]
pub fn default_shortcut() -> Shortcut {
    let modifiers = if cfg!(target_os = "macos") {
        Modifiers::SUPER | Modifiers::SHIFT
    } else {
        Modifiers::CONTROL | Modifiers::SHIFT
    };
    Shortcut::new(Some(modifiers), Code::KeyV)
}

static SNIPPET_HOTKEYS: Mutex<Vec<(i64, String)>> = Mutex::new(Vec::new());

pub fn normalize(spec: &str) -> Result<Option<String>> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Ok(None);
    }
    Ok(Some(parse(spec)?.into_string()))
}

pub fn sync_snippets(app: &AppHandle, snippets: &[Snippet], reserved: &[String]) -> Vec<String> {
    let mut warnings = Vec::new();

    let reserved_keys: HashSet<String> = reserved
        .iter()
        .filter_map(|r| parse(r).ok().map(|k| k.into_string()))
        .collect();
    let mut desired: Vec<(i64, String)> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for s in snippets {
        let Some(raw) = s.shortcut.as_deref().map(str::trim).filter(|v| !v.is_empty()) else {
            continue;
        };
        let key = match parse(raw) {
            Ok(k) => k.into_string(),
            Err(e) => {
                warnings.push(format!("片段「{}」的快捷键无法识别：{e}", s.title));
                continue;
            }
        };
        if reserved_keys.contains(&key) {
            warnings.push(format!(
                "片段「{}」的快捷键 {raw} 已被面板热键占用，本次跳过",
                s.title
            ));
            continue;
        }
        if !seen.insert(key.clone()) {
            warnings.push(format!("片段「{}」的快捷键 {raw} 与另一个片段重复，本次跳过", s.title));
            continue;
        }
        desired.push((s.id, key));
    }

    let shortcuts = app.global_shortcut();
    let mut registered = SNIPPET_HOTKEYS.lock().unwrap_or_else(|e| e.into_inner());

    let desired_keys: HashSet<&str> = desired.iter().map(|(_, k)| k.as_str()).collect();
    for (_, spec) in registered.iter() {
        if desired_keys.contains(spec.as_str()) {
            continue;
        }
        if let Ok(k) = parse(spec) {
            let _ = shortcuts.unregister(k);
        }
    }

    let mut now: Vec<(i64, String)> = Vec::new();
    for (id, spec) in &desired {
        if registered.iter().any(|(_, s)| s == spec) {
            now.push((*id, spec.clone()));
            continue;
        }
        match parse(spec) {
            Ok(k) => match shortcuts.register(k) {
                Ok(()) => now.push((*id, spec.clone())),
                Err(e) => warnings.push(format!(
                    "片段快捷键 {spec} 注册失败（可能已被其它程序占用）：{e}"
                )),
            },
            Err(e) => warnings.push(format!("片段快捷键 {spec} 无法识别：{e}")),
        }
    }

    *registered = now;
    warnings
}

pub fn is_snippet_hotkey(shortcut: &Shortcut) -> bool {
    let spec = shortcut.to_string();
    SNIPPET_HOTKEYS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .any(|(_, s)| s == &spec)
}

pub fn sync_snippets_from_db(app: &AppHandle) -> Vec<String> {
    let state = app.state::<AppState>();
    let snippets = match state.db.list_snippets() {
        Ok(v) => v,
        Err(e) => return vec![format!("读取片段失败，片段热键未同步：{e}")],
    };
    let settings = state.settings();
    sync_snippets(app, &snippets, &[settings.hotkey, settings.plain_hotkey])
}

pub fn log_snippet_sync(app: &AppHandle) {
    for w in sync_snippets_from_db(app) {
        eprintln!("[plico] {w}");
    }
}

pub fn dispatch(app: &AppHandle, shortcut: &Shortcut, event_state: ShortcutState) {
    if event_state != ShortcutState::Pressed {
        return;
    }

    let settings = app.state::<AppState>().settings();

    if let Ok(s) = parse(&settings.hotkey) {
        if s == *shortcut {
            panel::toggle_panel(app);
            return;
        }
    }
    if let Ok(s) = parse(&settings.plain_hotkey) {
        if s == *shortcut {
            paste_latest_plain(app);
            return;
        }
    }
    if is_snippet_hotkey(shortcut) {
        paste_snippet(app, shortcut);
    }
}

fn paste_latest_plain(app: &AppHandle) {
    let state = app.state::<AppState>();
    let items = match state.db.list(None, None, 1) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[plico] 全局纯文本粘贴读库失败：{e}");
            return;
        }
    };
    let Some(item) = items.into_iter().find(|i| {
        matches!(
            i.kind,
            crate::model::ItemType::Text
                | crate::model::ItemType::RichText
                | crate::model::ItemType::Link
                | crate::model::ItemType::Color
        )
    }) else {
        return;
    };

    if let Err(e) =
        crate::clipboard::paste::paste_item_as(app, &state.db, &state.suppress, item.id, true)
    {
        eprintln!("[plico] 全局纯文本粘贴失败：{e}");
    }
}

fn paste_snippet(app: &AppHandle, shortcut: &Shortcut) {
    let state = app.state::<AppState>();
    let snippets = match state.db.list_snippets() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[plico] 片段热键读库失败：{e}");
            return;
        }
    };

    let wanted = shortcut.to_string();
    let Some(snippet) = snippets.into_iter().find(|s| {
        s.shortcut
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .and_then(|v| parse(v).ok())
            .map(|k| k.into_string())
            .as_deref()
            == Some(wanted.as_str())
    }) else {
        return;
    };

    let (text, left_moves) = crate::clipboard::paste::split_cursor_placeholder(&snippet.content);
    if let Err(e) = crate::clipboard::paste::paste_text_with_cursor_offset(
        app,
        &state.suppress,
        &text,
        left_moves,
    ) {
        eprintln!("[plico] 片段热键粘贴失败：{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize, parse};

    #[test]
    fn 空快捷键归一成_none() {
        assert_eq!(normalize("").unwrap(), None);
        assert_eq!(normalize("   ").unwrap(), None);
    }

    #[test]
    fn 不同写法的同一个键被归一成同一串() {
        let a = normalize("Ctrl+Shift+1").unwrap();
        let b = normalize("ctrl + shift + 1").unwrap();
        let c = normalize("Shift+Control+Digit1").unwrap();
        assert!(a.is_some());
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn 无法识别的快捷键报错() {
        assert!(normalize("Ctrl+Foo").is_err());
        assert!(normalize("Ctrl+").is_err());
    }

    #[test]
    fn 归一化结果可以被重新解析回来() {
        let canon = normalize("Ctrl+Alt+1").unwrap().unwrap();
        assert!(parse(&canon).is_ok(), "归一化结果必须能重新解析：{canon}");
    }
}
