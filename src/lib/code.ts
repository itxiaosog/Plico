/**
 * 极简易的代码着色（F20）。
 *
 * 不是真的语法高亮 —— 没有 parser，只是把常见 token 包一层颜色。
 * 分词按「字符串 / 注释 / 数字 / 关键字 / 其他」四类扫一遍，
 * 输出段序列给 React 渲染。命中关键字时颜色走 `--pl-accent`，
 * 其余走三级灰度文字，与「颜色只出现在内容里」的视觉原则一致。
 */

export type TokenKind = "str" | "comment" | "num" | "kw" | "plain";

export interface Token {
  kind: TokenKind;
  text: string;
}

const KEYWORDS = new Set([
  // 多语言公共
  "if", "else", "for", "while", "return", "break", "continue", "switch", "case", "default",
  "try", "catch", "finally", "throw", "throws", "new", "delete", "typeof", "instanceof",
  "class", "extends", "interface", "implements", "public", "private", "protected", "static",
  "import", "from", "export", "as", "in", "of", "is", "not", "and", "or", "None", "True", "False",
  // JS / TS
  "const", "let", "var", "function", "async", "await", "yield", "type", "enum", "readonly",
  "this", "super", "void", "null", "undefined", "true", "false", "get", "set",
  // Rust
  "fn", "impl", "struct", "trait", "pub", "mod", "use", "crate", "mut", "ref", "self", "Self",
  "match", "where", "dyn", "unsafe", "extern", "move", "loop", "Some", "Ok", "Err",
  // Python
  "def", "lambda", "pass", "elif", "with", "raise", "assert", "global", "nonlocal", "del",
  // SQL（大写）
  "SELECT", "INSERT", "UPDATE", "DELETE", "FROM", "WHERE", "JOIN", "LEFT", "RIGHT", "INNER",
  "OUTER", "ON", "GROUP", "BY", "ORDER", "HAVING", "LIMIT", "OFFSET", "AS", "DISTINCT", "UNION",
  "CREATE", "TABLE", "DROP", "ALTER", "INDEX", "PRIMARY", "KEY", "NOT", "NULL", "DEFAULT",
]);

const LANG_KEYWORDS: Record<string, Set<string>> = {
  sql: KEYWORDS,
};

export function tokenize(code: string, lang?: string | null): Token[] {
  const kws: Set<string> = (lang ? LANG_KEYWORDS[lang] : undefined) ?? KEYWORDS;
  const out: Token[] = [];
  const n = code.length;
  let i = 0;
  let plain = "";

  const flush = () => {
    if (plain) {
      out.push({ kind: "plain", text: plain });
      plain = "";
    }
  };

  while (i < n) {
    const c = code[i]!;
    const rest = code.slice(i);

    // 行注释 // 和 #
    if (rest.startsWith("//") || (c === "#" && !rest.startsWith("#!"))) {
      const end = code.indexOf("\n", i);
      flush();
      const j = end < 0 ? n : end;
      out.push({ kind: "comment", text: code.slice(i, j) });
      i = j;
      continue;
    }

    // 块注释 /* */
    if (rest.startsWith("/*")) {
      flush();
      const end = code.indexOf("*/", i + 2);
      const j = end < 0 ? n : end + 2;
      out.push({ kind: "comment", text: code.slice(i, j) });
      i = j;
      continue;
    }

    // 字符串（含转义），" ' ` 都算
    if (c === '"' || c === "'" || c === "`") {
      flush();
      let j = i + 1;
      while (j < n && code[j] !== c) {
        if (code[j] === "\\") j += 1;
        j += 1;
      }
      j = Math.min(j + 1, n);
      out.push({ kind: "str", text: code.slice(i, j) });
      i = j;
      continue;
    }

    // 数字（整数/浮点/0x/0b/科学计数）
    if (/[0-9]/.test(c) && (i === 0 || !/[A-Za-z_$]/.test(code[i - 1]!))) {
      const m = rest.match(/^(0x[0-9a-fA-F]+|0b[01]+|[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)/);
      if (m) {
        flush();
        out.push({ kind: "num", text: m[0] });
        i += m[0].length;
        continue;
      }
    }

    // 标识符：往关键字表里查
    if (/[A-Za-z_$]/.test(c)) {
      const m = rest.match(/^[A-Za-z_$][A-Za-z0-9_$]*/);
      if (m) {
        const word = m[0];
        if (kws.has(word)) {
          flush();
          out.push({ kind: "kw", text: word });
        } else {
          plain += word;
        }
        i += word.length;
        continue;
      }
    }

    plain += c;
    i += 1;
  }

  flush();
  return out;
}
