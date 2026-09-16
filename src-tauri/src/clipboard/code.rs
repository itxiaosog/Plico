//! F20 代码语言识别。
//!
//! 用途只有一个：决定预览区要不要把这段文本按等宽 + 关键字高亮展示。
//! 判据的标准因此是「宁可返回 None，也不要判错」—— 把一封邮件渲染成代码
//! 比不渲染更糟。不进库、不参与去重。
//!
//! 三层判据，从强到弱：
//!   1. **结构性**：合法 JSON / shebang / 成片的 HTML 标签。命中即定性。
//!   2. **代码骨架上的加权关键词**：先把注释和字符串剥掉，再统计关键词。
//!      不剥的话，`// 这里要 import 一个 type` 这种中文注释正好含有关键词。
//!   3. **门槛**：太短或只有一行的文本直接放弃。

/// 一门语言的判据。
struct Profile {
    lang: &'static str,
    /// 关键词表是否按小写匹配 —— SQL 里 `select` 和 `SELECT` 一样常见。
    ci: bool,
    /// `(信号串, 权重)`。权重表达「这门语言有多独占这个串」。
    signals: &'static [(&'static str, u32)],
}

/// 低于这个分数不认。定成 3 是因为单条高权重信号（`impl `、`console.log`）
/// 就足够定性，而两三条低权重信号（`::` + `use `）凑起来也算数。
const MIN_SCORE: u32 = 3;

/// 单行文本的门槛。一行散文里偶尔也会撞上一两个关键词（"export the data
/// from the table"），所以单行要拿到更高的分数才认 —— 多行本身是代码的强信号，
/// 散文很少有多行结构。一条 `SELECT ... WHERE ... ORDER BY ...` 能到 15 分，
/// 一句英文句子通常到不了 8 分。
const SINGLE_LINE_MIN_SCORE: u32 = 8;

/// 最短长度。比 1.0 的 40 字符宽松：关键词现在带权重，`impl ` + `let mut `
/// 两条就能定性，没必要再凑长度。但下限仍然要有 —— 一个 `if x` 不该被当代码。
const MIN_CHARS: usize = 30;

pub fn detect(text: &str) -> Option<&'static str> {
    let raw = text.trim();
    if raw.is_empty() {
        return None;
    }

    if let Some(lang) = structural(raw) {
        return Some(lang);
    }

    if raw.chars().count() < MIN_CHARS {
        return None;
    }

    // 关键词只在骨架上匹配：注释和字符串里的词不算数
    let skeleton = strip_comments_and_strings(raw);
    let lowered = skeleton.to_lowercase();
    let threshold = if raw.lines().count() >= 2 {
        MIN_SCORE
    } else {
        SINGLE_LINE_MIN_SCORE
    };

    let mut best: Option<(&'static str, u32)> = None;
    for p in PROFILES {
        let hay = if p.ci { lowered.as_str() } else { skeleton.as_str() };
        let score: u32 = p
            .signals
            .iter()
            .filter(|(needle, _)| hay.contains(needle))
            .map(|(_, weight)| *weight)
            .sum();
        if score < threshold {
            continue;
        }
        // 同分时保留先出现的（PROFILES 的顺序即优先级），保证结果稳定
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((p.lang, score));
        }
    }
    best.map(|(lang, _)| lang)
}

/// 结构性信号：命中一条就能定语言，不必凑关键词。
fn structural(raw: &str) -> Option<&'static str> {
    // shebang —— 几乎是唯一解
    if let Some(first) = raw.lines().next() {
        if first.starts_with("#!") {
            return Some("shell");
        }
    }

    let first_char = raw.chars().next()?;

    // 合法 JSON 只可能长这样：以 `{` / `[` 开头，且整体能被解析。
    // 这一条是判定性的 —— `{"a":1}` 只有 7 个字符也该被认出来，
    // 不该卡在 MIN_CHARS 上。
    if matches!(first_char, '{' | '[') && serde_json::from_str::<serde_json::Value>(raw).is_ok() {
        return Some("json");
    }

    // 成片的 HTML 标签。门槛是「≥3 个 `<` 且有闭合标签」：
    // 单独一个 `<` 可能是比较运算符或 C++ 模板参数，凑不满这个数。
    if first_char == '<' {
        let opens = raw.matches('<').count();
        if opens >= 3 && (raw.contains("</") || raw.contains("/>")) {
            return Some("html");
        }
    }

    None
}

/// 剥掉注释与字符串字面量，留下「代码骨架」。
///
/// 刻意**不**处理单引号：Rust 的 `&'a str` 生命周期、Python 英文撇号（`don't`）
/// 都会被误当成字符串开头，一剥就吃掉半段真代码。而单引号字符串里含有关键词的
/// 概率很低 —— 这个取舍换来的是「绝不剥错」。
fn strip_comments_and_strings(src: &str) -> String {
    #[derive(PartialEq)]
    enum State {
        Code,
        Line,
        Block,
        Str,
        Template,
    }

    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut state = State::Code;
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];
        let n = chars.get(i + 1).copied();

        match state {
            State::Code => {
                if c == '/' && n == Some('/') {
                    state = State::Line;
                    i += 2;
                    continue;
                }
                if c == '/' && n == Some('*') {
                    state = State::Block;
                    i += 2;
                    continue;
                }
                if c == '#' {
                    // `#[` 是 Rust 属性、`#!` 是 shebang、`#include` 一类是 C 预处理
                    // 指令 —— 三者都不是注释，先排除掉。
                    //
                    // 剩下的还要再判一次位置：只有「这一行到目前全是空白」的 `#`
                    // 才是注释，否则 CSS 的 `#id` 选择器会被吃掉。
                    let tail: String = chars[i + 1..].iter().take(8).collect();
                    let is_directive = tail.starts_with('[')
                        || tail.starts_with('!')
                        || ["include", "define", "ifdef", "ifndef", "endif", "pragma"]
                            .iter()
                            .any(|d| tail.starts_with(d));
                    if !is_directive {
                        let line_so_far = out.rsplit('\n').next().unwrap_or("");
                        if line_so_far.trim().is_empty() {
                            state = State::Line;
                            i += 1;
                            continue;
                        }
                    }
                }
                if c == '"' {
                    state = State::Str;
                    out.push(' ');
                    i += 1;
                    continue;
                }
                if c == '`' {
                    state = State::Template;
                    out.push(' ');
                    i += 1;
                    continue;
                }
                out.push(c);
                i += 1;
            }
            State::Line => {
                if c == '\n' {
                    state = State::Code;
                    out.push('\n');
                }
                i += 1;
            }
            State::Block => {
                if c == '*' && n == Some('/') {
                    state = State::Code;
                    out.push(' ');
                    i += 2;
                    continue;
                }
                if c == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            State::Str => {
                if c == '\\' {
                    i += 2;
                    continue;
                }
                // 换行说明这个引号根本没闭合（单引号没处理，`'` 开头的行很常见），
                // 认输回到代码态，别把后面整段都吃掉
                if c == '\n' {
                    state = State::Code;
                    out.push('\n');
                } else if c == '"' {
                    state = State::Code;
                }
                i += 1;
            }
            State::Template => {
                if c == '\\' {
                    i += 2;
                    continue;
                }
                // 模板串可以跨行，换行不算认输
                if c == '\n' {
                    out.push('\n');
                } else if c == '`' {
                    state = State::Code;
                }
                i += 1;
            }
        }
    }

    out
}

/// 关键词表。顺序即优先级（同分时靠前的胜出）。
///
/// TypeScript 只收**独占**信号：`const` / `=>` / `import` 这些 JS 也有的
/// 放在 javascript 里。否则一段普通 JS 会被判成 TS（TS 是 JS 的超集，
/// 反过来放就会这样）。真含类型标注的 TS 靠 `: string` 这类信号胜出。
const PROFILES: &[Profile] = &[
    Profile {
        lang: "rust",
        ci: false,
        signals: &[
            ("fn ", 3),
            ("impl ", 3),
            ("pub ", 2),
            ("let mut ", 3),
            ("&str", 3),
            ("&mut ", 3),
            ("println!", 4),
            ("#[", 3),
            ("-> ", 2),
            ("crate::", 3),
            ("unwrap()", 3),
            ("Option<", 3),
            ("Result<", 3),
            ("vec![", 3),
            ("match ", 2),
            ("::", 1),
            ("use ", 1),
        ],
    },
    Profile {
        lang: "typescript",
        ci: false,
        signals: &[
            (": string", 4),
            (": number", 4),
            (": boolean", 4),
            (": void", 4),
            ("interface ", 4),
            ("implements ", 3),
            ("readonly ", 3),
            ("export type ", 4),
            (" as const", 3),
            ("<T>", 3),
            ("?: ", 2),
            ("): ", 2),
            ("enum ", 2),
            ("| null", 2),
            ("| undefined", 2),
        ],
    },
    Profile {
        lang: "javascript",
        ci: false,
        signals: &[
            ("console.log", 4),
            ("require(", 4),
            ("module.exports", 4),
            ("function ", 3),
            ("var ", 3),
            ("document.", 3),
            ("JSON.", 3),
            ("const ", 2),
            ("=> ", 2),
            ("async ", 2),
            ("await ", 2),
            ("window.", 2),
            ("let ", 1),
            ("export ", 1),
            ("import ", 1),
        ],
    },
    Profile {
        lang: "python",
        ci: false,
        signals: &[
            ("def ", 4),
            ("elif ", 4),
            ("self.", 4),
            ("__init__", 4),
            ("print(", 3),
            ("lambda ", 3),
            ("try:", 3),
            ("except ", 3),
            ("f\"", 3),
            ("None", 2),
            ("True", 2),
            ("False", 2),
            ("range(", 2),
            ("import ", 1),
            ("from ", 1),
        ],
    },
    Profile {
        lang: "css",
        ci: true,
        signals: &[
            ("display:", 4),
            ("font-size:", 4),
            ("border-radius:", 4),
            ("grid-template", 4),
            ("!important", 4),
            ("@media", 4),
            ("background:", 3),
            ("color:", 3),
            ("margin:", 3),
            ("padding:", 3),
            ("var(--", 3),
            ("flex", 2),
            ("rem;", 2),
            ("px;", 1),
        ],
    },
    Profile {
        lang: "sql",
        ci: true,
        signals: &[
            ("create table", 5),
            ("insert into ", 5),
            ("delete from ", 5),
            ("select ", 4),
            ("group by", 4),
            ("order by", 4),
            ("primary key", 4),
            ("where ", 3),
            ("join ", 3),
            ("values (", 3),
            ("update ", 3),
            ("from ", 2),
            ("limit ", 2),
        ],
    },
    Profile {
        lang: "shell",
        ci: false,
        signals: &[
            ("echo ", 4),
            ("sudo ", 4),
            ("apt-get", 4),
            ("chmod ", 4),
            ("$(", 3),
            ("${", 3),
            ("mkdir ", 3),
            ("&& ", 2),
            ("|| ", 2),
            ("export ", 2),
            ("npm ", 2),
            ("git ", 2),
            ("then\n", 3),
            ("fi\n", 4),
        ],
    },
    Profile {
        lang: "html",
        ci: true,
        signals: &[
            ("<!doctype", 5),
            ("<div", 4),
            ("<span", 4),
            ("<p>", 4),
            ("<img", 4),
            ("<head", 4),
            ("<body", 4),
            ("<meta", 4),
            ("class=", 3),
            ("href=", 3),
            ("<a ", 3),
            ("<ul", 3),
            ("<li", 3),
            ("</", 2),
        ],
    },
    Profile {
        lang: "json",
        ci: false,
        signals: &[
            ("\": {", 4),
            ("\": [", 4),
            ("\": true", 4),
            ("\": false", 4),
            ("\": null", 4),
            ("\": \"", 3),
            ("\"},\n", 3),
            ("[\n  {", 3),
            ("\": 0", 2),
        ],
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 识别_rust() {
        let code = "fn main() {\n    let mut x = 1;\n    println!(\"{}\", x);\n}";
        assert_eq!(detect(code), Some("rust"));
    }

    #[test]
    fn 识别_rust_带生命周期() {
        // 单引号不该被当成字符串开头，否则 `&'a str` 会把后面整段吃掉
        let code = "pub fn parse<'a>(input: &'a str) -> &'a str {\n    let s = input.trim();\n    s\n}";
        assert_eq!(detect(code), Some("rust"));
    }

    #[test]
    fn 识别_python() {
        let code = "def hello(name):\n    if name is None:\n        print(\"x\")\n    return self.x";
        assert_eq!(detect(code), Some("python"));
    }

    #[test]
    fn 识别_typescript() {
        let code = "interface User { id: number; name: string }\nexport function greet(u: User): string {\n  return u.name;\n}";
        assert_eq!(detect(code), Some("typescript"));
    }

    /// TS 是 JS 的超集。没有类型标注的 JS 不该被判成 TS。
    #[test]
    fn 识别_javascript() {
        let code = "const add = (a, b) => a + b;\nconsole.log(add(1, 2));\nmodule.exports = { add };";
        assert_eq!(detect(code), Some("javascript"));
    }

    #[test]
    fn 识别_sql() {
        let code = "SELECT id, name\n  FROM users\n WHERE pinned = 1\n ORDER BY created_at DESC;";
        assert_eq!(detect(code), Some("sql"));
    }

    #[test]
    fn 识别小写_sql() {
        let code = "select id, name\nfrom users\nwhere pinned = 1\norder by created_at desc;";
        assert_eq!(detect(code), Some("sql"));
    }

    #[test]
    fn 识别_css() {
        let code = ".card {\n  display: flex;\n  border-radius: 10px;\n  font-size: 13px;\n}";
        assert_eq!(detect(code), Some("css"));
    }

    #[test]
    fn 识别_shell() {
        let code = "echo \"building\"\nsudo apt-get update\nmkdir -p dist";
        assert_eq!(detect(code), Some("shell"));
    }

    /// 一条合法 JSON 只有 7 个字符，不该卡在长度门槛上。
    #[test]
    fn 合法_json_不看长度() {
        assert_eq!(detect("{\"a\": 1}"), Some("json"));
        assert_eq!(detect("[1, 2, 3]"), Some("json"));
    }

    #[test]
    fn 合法_json_多行() {
        let code = "{\n  \"name\": \"plico\",\n  \"version\": \"0.1.0\",\n  \"private\": true\n}";
        assert_eq!(detect(code), Some("json"));
    }

    #[test]
    fn 识别_html() {
        let code = "<div class=\"card\">\n  <span>hi</span>\n  <img src=\"a.png\" />\n</div>";
        assert_eq!(detect(code), Some("html"));
    }

    #[test]
    fn 短文本不算代码() {
        assert_eq!(detect("if x"), None);
        assert_eq!(detect("https://a.com"), None);
    }

    /// 单行 SQL 是剪贴板里最常见的一类代码，不能因为「只有一行」就漏掉。
    #[test]
    fn 单行_sql_也能识别() {
        let code = "SELECT id, \"type\", plain_text FROM items WHERE pinned = 1 ORDER BY last_copied_at DESC LIMIT 50;";
        assert_eq!(detect(code), Some("sql"));
    }

    /// 反过来，一行英文散文里的零散关键词不该凑够单行门槛。
    #[test]
    fn 单行英文散文不算代码() {
        let prose = "Please export the data from the table and import it into the new system before Friday.";
        assert_eq!(detect(prose), None);
    }

    #[test]
    fn 中文邮件不算代码() {
        let mail = "小宋，关于 Q4 预算的回复，重点是研发投入要拆成人力与设备两块。下周给你详细版。";
        assert_eq!(detect(mail), None);
    }

    /// 注释里的关键词不该把散文判成代码 —— 这是 1.0 最容易误报的场景。
    #[test]
    fn 注释里的关键词不算数() {
        let prose = "会议纪要：\n// 需要 import 一个 type 并 export interface\n// 再定义一个 function 和 const\n请张三确认。";
        assert_eq!(detect(prose), None);
    }

    /// 字符串里的关键词同理。
    #[test]
    fn 字符串里的关键词不算数() {
        let text = "错误提示原文如下：\n\"Cannot find interface User, did you forget to import it? Please check your const declaration.\"\n请联系后端同学。";
        assert_eq!(detect(text), None);
    }

    #[test]
    fn 骨架保留代码剥掉注释() {
        let s = strip_comments_and_strings("let a = \"fn main\"; // impl Foo\nlet b = 1;");
        assert!(s.contains("let a ="));
        assert!(s.contains("let b = 1;"));
        assert!(!s.contains("fn main"), "字符串内容应被剥掉");
        assert!(!s.contains("impl Foo"), "行注释应被剥掉");
    }

    #[test]
    fn 井号注释只在行首生效() {
        // Rust 属性不该被当成注释吃掉
        let s = strip_comments_and_strings("#[derive(Debug)]\nstruct A;");
        assert!(s.contains("#[derive(Debug)]"));
        // 行首的井号才是注释
        let s2 = strip_comments_and_strings("x = 1\n# 这是注释\nimport os");
        assert!(!s2.contains("这是注释"));
        assert!(s2.contains("import os"));
    }
}
