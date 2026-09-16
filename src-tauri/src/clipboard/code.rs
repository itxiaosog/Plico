struct Profile {
    lang: &'static str,
    ci: bool,
    signals: &'static [(&'static str, u32)],
}

const MIN_SCORE: u32 = 3;

const SINGLE_LINE_MIN_SCORE: u32 = 8;

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
        if best.is_none_or(|(_, s)| score > s) {
            best = Some((p.lang, score));
        }
    }
    best.map(|(lang, _)| lang)
}

fn structural(raw: &str) -> Option<&'static str> {
    if let Some(first) = raw.lines().next() {
        if first.starts_with("#!") {
            return Some("shell");
        }
    }

    let first_char = raw.chars().next()?;

    if matches!(first_char, '{' | '[') && serde_json::from_str::<serde_json::Value>(raw).is_ok() {
        return Some("json");
    }

    if first_char == '<' {
        let opens = raw.matches('<').count();
        if opens >= 3 && (raw.contains("</") || raw.contains("/>")) {
            return Some("html");
        }
    }

    None
}

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

    #[test]
    fn 单行_sql_也能识别() {
        let code = "SELECT id, \"type\", plain_text FROM items WHERE pinned = 1 ORDER BY last_copied_at DESC LIMIT 50;";
        assert_eq!(detect(code), Some("sql"));
    }

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

    #[test]
    fn 注释里的关键词不算数() {
        let prose = "会议纪要：\n// 需要 import 一个 type 并 export interface\n// 再定义一个 function 和 const\n请张三确认。";
        assert_eq!(detect(prose), None);
    }

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
        let s = strip_comments_and_strings("#[derive(Debug)]\nstruct A;");
        assert!(s.contains("#[derive(Debug)]"));
        let s2 = strip_comments_and_strings("x = 1\n# 这是注释\nimport os");
        assert!(!s2.contains("这是注释"));
        assert!(s2.contains("import os"));
    }
}
