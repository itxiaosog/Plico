export type TokenKind = "str" | "comment" | "num" | "kw" | "plain";

export interface Token {
  kind: TokenKind;
  text: string;
}

const KEYWORDS = new Set([
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

    if (rest.startsWith("//") || (c === "#" && !rest.startsWith("#!"))) {
      const end = code.indexOf("\n", i);
      flush();
      const j = end < 0 ? n : end;
      out.push({ kind: "comment", text: code.slice(i, j) });
      i = j;
      continue;
    }

    if (rest.startsWith("/*")) {
      flush();
      const end = code.indexOf("*/", i + 2);
      const j = end < 0 ? n : end + 2;
      out.push({ kind: "comment", text: code.slice(i, j) });
      i = j;
      continue;
    }

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

    if (/[0-9]/.test(c) && (i === 0 || !/[A-Za-z_$]/.test(code[i - 1]!))) {
      const m = rest.match(/^(0x[0-9a-fA-F]+|0b[01]+|[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)/);
      if (m) {
        flush();
        out.push({ kind: "num", text: m[0] });
        i += m[0].length;
        continue;
      }
    }

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
