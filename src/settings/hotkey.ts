export const DEFAULT_HOTKEY = "Ctrl+Shift+V";

export const DEFAULT_PLAIN_HOTKEY = "Ctrl+Shift+Alt+V";

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

export function keyFromCode(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  if (NUMPAD_KEYS.has(code)) return code;
  return SPECIAL_KEYS[code] ?? null;
}

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

export function formatHotkey(spec: string): string {
  return spec
    .split("+")
    .map((t) => PRETTY[t.trim().toUpperCase()] ?? t.trim())
    .join(" + ");
}
