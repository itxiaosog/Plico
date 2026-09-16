use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::app::panel;
use crate::error::{Error, Result};
use crate::model::Snippet;
use crate::AppState;

/// 默认唤起热键：Windows `Ctrl+Shift+V` / macOS `Cmd+Shift+V`（F4）。
pub fn default_spec() -> &'static str {
    if cfg!(target_os = "macos") {
        "Super+Shift+V"
    } else {
        "Ctrl+Shift+V"
    }
}

/// 默认「粘贴为纯文本」热键：F15，Ctrl+Shift+Alt+V / Cmd+Shift+Option+V。
/// `settings` 的默认值兜底就是它，所以注册失败时不走回退（见 lib.rs）。
#[allow(dead_code)]
pub fn default_plain_spec() -> &'static str {
    if cfg!(target_os = "macos") {
        "Super+Shift+Alt+V"
    } else {
        "Ctrl+Shift+Alt+V"
    }
}

/// 把设置里的热键描述文本解析成可注册的快捷键。
///
/// 解析规则来自 `global-hotkey`：`修饰键+修饰键+主键`，修饰键在前，
/// 大小写不敏感，`Ctrl` / `Control` 等价。所以 `Ctrl+Shift+V` 是合法写法。
pub fn parse(spec: &str) -> Result<Shortcut> {
    Shortcut::from_str(spec).map_err(|e| Error::Other(format!("无法识别的快捷键「{spec}」：{e}")))
}

/// 注册单个热键。
pub fn register(app: &AppHandle, spec: &str) -> Result<()> {
    let shortcut = parse(spec)?;
    app.global_shortcut()
        .register(shortcut)
        .map_err(|e| Error::Other(format!("注册全局热键「{spec}」失败（可能已被其它程序占用）：{e}")))?;
    Ok(())
}

/// 改热键：**先注册新的，成功后再注销旧的**。
///
/// 顺序很关键 —— 反过来的话，新键注册失败会留下「新旧都没注册上」的空窗，
/// 用户会以为 Plico 挂了。现在的顺序最坏情况是旧键仍然可用。
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
            // 注销失败无关紧要：旧键会多占一会儿，但不影响新键已生效
            let _ = shortcuts.unregister(old);
        }
    }

    Ok(())
}

/// 面板内使用的兜底默认值。`Code` / `Modifiers` 留在这里是为了将来
/// 需要按平台拼装快捷键时不用再引一遍。
#[allow(dead_code)]
pub fn default_shortcut() -> Shortcut {
    let modifiers = if cfg!(target_os = "macos") {
        Modifiers::SUPER | Modifiers::SHIFT
    } else {
        Modifiers::CONTROL | Modifiers::SHIFT
    };
    Shortcut::new(Some(modifiers), Code::KeyV)
}

// ---------------- F18 片段全局热键 ----------------

/// 当前已注册的片段热键：`(片段 id, 规范化后的快捷键文本)`。
///
/// 放模块级静态量而不是 `AppState` 字段：热键是**进程全局**的稀缺资源，
/// 生命周期和窗口/数据库无关。锁中毒时取内层值继续用（与 `db.rs` 同一策略）。
static SNIPPET_HOTKEYS: Mutex<Vec<(i64, String)>> = Mutex::new(Vec::new());

/// 把快捷键文本归一成规范写法，好让 `ctrl+shift+1` 和 `Ctrl+Shift+1`
/// 被认成同一个键。空串归一成 `None`。
pub fn normalize(spec: &str) -> Result<Option<String>> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Ok(None);
    }
    Ok(Some(parse(spec)?.into_string()))
}

/// 同步片段热键。每次片段增删改、以及面板/纯文本热键改动后都要跑一遍。
///
/// 全量重算而不是增量维护：片段是几十条的量级，重算成本可忽略，而增量维护
/// 「哪条改了什么」很容易在编辑/删除的边界上漏掉一个残留注册。
///
/// `reserved` 是面板热键与纯文本热键 —— 撞车的片段键直接跳过：全局快捷键是
/// 稀缺资源，不该让片段把主功能顶掉。
///
/// 返回没能注册的说明，交给调用方记日志。注册失败**不阻断**存片段 ——
/// 键被别的程序占了不该让用户连内容都存不下。
pub fn sync_snippets(app: &AppHandle, snippets: &[Snippet], reserved: &[String]) -> Vec<String> {
    let mut warnings = Vec::new();

    // 1) 先算出「这次想要注册什么」。同一个键被多个片段声明时只留先出现的那条，
    //    顺序稳定（list_snippets 按 id），不会每次启动换一个赢家。
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

    // 2) 注销不再需要的。放在注册之前：同一个键若换了归属，先注销再注册，
    //    否则插件会当成「已经注册过」而静默跳过。
    let desired_keys: HashSet<&str> = desired.iter().map(|(_, k)| k.as_str()).collect();
    for (_, spec) in registered.iter() {
        if desired_keys.contains(spec.as_str()) {
            continue;
        }
        if let Ok(k) = parse(spec) {
            let _ = shortcuts.unregister(k);
        }
    }

    // 3) 注册新增的。失败的不记进 `registered`，下次同步会再试一次。
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

/// 这个快捷键是不是某个已注册的片段。
pub fn is_snippet_hotkey(shortcut: &Shortcut) -> bool {
    let spec = shortcut.to_string();
    SNIPPET_HOTKEYS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .any(|(_, s)| s == &spec)
}

/// 从库里读片段 + 从设置里取保留键，然后同步一次。
///
/// 给「运行中改完片段」的场景用（启动时 `AppState` 还没挂上，走 `sync_snippets`）。
pub fn sync_snippets_from_db(app: &AppHandle) -> Vec<String> {
    let state = app.state::<AppState>();
    let snippets = match state.db.list_snippets() {
        Ok(v) => v,
        Err(e) => return vec![format!("读取片段失败，片段热键未同步：{e}")],
    };
    let settings = state.settings();
    sync_snippets(app, &snippets, &[settings.hotkey, settings.plain_hotkey])
}

/// 把 `sync_snippets_from_db` 的警告打到日志里。
///
/// 注册失败**不是**致命错误：片段内容已经存下了，只是这个键暂时没生效。
/// 把它升级成错误会让用户连内容都保存不了，那是更糟的结果。
pub fn log_snippet_sync(app: &AppHandle) {
    for w in sync_snippets_from_db(app) {
        eprintln!("[plico] {w}");
    }
}

/// 热键事件分发。`lib.rs` 的 plugin handler 把 (app, shortcut, event) 转到这里。
///
/// 三个热键的行为：
///   - 面板热键：toggle 面板
///   - 纯文本热键（F15）：把**最近一条文本类记录**以纯文本写回剪贴板并模拟粘贴。
///     找不到文本记录就不动作 —— 全局快捷键不该在没有可粘内容时打断用户。
///   - 片段热键（F18）：把该片段内容粘到当前应用，`{{cursor}}` 位置落光标。
///
/// 顺序有意义：面板 / 纯文本优先。`sync_snippets` 已经会跳过撞键的片段，
/// 这里再兜一层，保证任何情况下都不会出现「片段把主功能顶掉」。
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

/// F15 全局纯文本粘贴：拿最近一条文本/链接/颜色记录，写剪贴板 + 模拟 Ctrl+V。
///
/// 不依赖面板是否打开 —— 这是「在任意应用里直接粘纯文本」的场景。
fn paste_latest_plain(app: &AppHandle) {
    let state = app.state::<AppState>();
    // 最近一条「能当纯文本粘」的记录：文本 / 富文本 / 链接 / 颜色都算。
    // 图片和文件没有纯文本可粘，直接跳过。
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

/// F18 全局片段粘贴：按键对应的片段内容写剪贴板 + 模拟 Ctrl+V。
///
/// 每次按键都读一遍库而不是缓存片段内容 —— 用户可能刚在设置窗口改过片段，
/// 缓存会在「改完没重启」这段窗口里粘出旧内容。片段是几十条，一次查询可以忽略。
fn paste_snippet(app: &AppHandle, shortcut: &Shortcut) {
    let state = app.state::<AppState>();
    let snippets = match state.db.list_snippets() {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[plico] 片段热键读库失败：{e}");
            return;
        }
    };

    // 按归一化后的写法比对，否则 `ctrl+1` 和 `Ctrl+1` 会被认成两个键
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

    /// 归一化是撞键判定的前提：用户写 `ctrl+shift+1`、`Ctrl + Shift + 1`、
    /// `Shift+Control+Digit1` 说的都是同一个键，不能因为写法不同就放过冲突。
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
        // `sync_snippets` 会把归一化后的文本存进注册表，dispatch 再拿它比对；
        // 这条守住「存进去的写法自己认得出来」。
        let canon = normalize("Ctrl+Alt+1").unwrap().unwrap();
        assert!(parse(&canon).is_ok(), "归一化结果必须能重新解析：{canon}");
    }
}
