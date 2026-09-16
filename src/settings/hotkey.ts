/**
 * 快捷键的展示与录制。
 *
 * 存储格式是 Rust 侧 `global-hotkey` 能解析的字符串：`Ctrl+Shift+V`。
 * 修饰键在前、主键在后，大小写不敏感。
 *
 * 主键统一从 `KeyboardEvent.code` 取，而不是 `key`：`code` 是物理按键，
 * 不受输入法、大小写、键盘布局影响。用户在中文输入法下按 V，`key` 可能是
 * "Process"，但 `code` 永远是 "KeyV"。
 */

/** 与 Rust 侧 `hotkey::default_spec()` 保持一致（Windows 分支）。 */
export const DEFAULT_HOTKEY = "Ctrl+Shift+V";

/** F15「粘贴为纯文本」默认热键（Windows 分支）。 */
export const DEFAULT_PLAIN_HOTKEY = "Ctrl+Shift+Alt+V";

/** `code` 到 `global-hotkey` 可解析名称的映射。只收 parse_key 认识的那些。 */
const SPECIAL_KEYS: Record<string, string> = {
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Space: "Space",
  Enter: "Enter",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
};

const NUMPAD_KEYS = new Set([
  "Numpad0",
  "Numpad1",
  "Numpad2",
  "Numpad3",
  "Numpad4",
  "Numpad5",
  "Numpad6",
  "Numpad7",
  "Numpad8",
  "Numpad9",
  "NumpadAdd",
  "NumpadDecimal",
  "NumpadDivide",
  "NumpadEnter",
  "NumpadEqual",
  "NumpadMultiply",
  "NumpadSubtract",
]);

/** 取主键名。返回 null 表示这个键不能当主键（纯修饰键、或 parse_key 不认识）。 */
export function keyFromCode(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  if (NUMPAD_KEYS.has(code)) return code;
  return SPECIAL_KEYS[code] ?? null;
}

/** 从键盘事件拼出存储用的快捷键字符串。主键不合法时返回 null。 */
export function buildHotkey(e: KeyboardEvent): string | null {
  const key = keyFromCode(e.code);
  if (!key) return null;

  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");
  if (e.metaKey) parts.push("Super");

  return [...parts, key].join("+");
}

/** 修饰键数量。少于一个修饰键的组合会和普通输入冲突，不该允许。 */
export function modifierCount(spec: string): number {
  return spec.split("+").filter((t) => {
    const u = t.trim().toUpperCase();
    return ["CTRL", "CONTROL", "ALT", "OPTION", "SHIFT", "SUPER", "CMD", "COMMAND"].includes(u);
  }).length;
}

const PRETTY: Record<string, string> = {
  CTRL: "Ctrl",
  CONTROL: "Ctrl",
  ALT: "Alt",
  OPTION: "Alt",
  SHIFT: "Shift",
  SUPER: "Win",
  CMD: "Win",
  COMMAND: "Win",
};

/** `Ctrl+Shift+V` → `Ctrl + Shift + V`，只为了好看。 */
export function formatHotkey(spec: string): string {
  return spec
    .split("+")
    .map((t) => PRETTY[t.trim().toUpperCase()] ?? t.trim())
    .join(" + ");
}
