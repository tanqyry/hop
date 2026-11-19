//! More languages, requested by people who tried hop. Same table format as syntax.rs.
//! Extra classes used here: 11 added line (diff), 12 removed line (diff).
use crate::syntax::*;

fn blk(s: &'static str) -> Vec<Rule> { vec![rn(r"\*/", 'c', s), r(r"[^*]+|\*", 'c')] }

/// the C family: line and block comments, strings, keywords, types, calls
fn clike(name: &'static str, exts: &'static [&'static str], kws: &[&str], types: &[&str], extra: Vec<Rule>) -> Lang {
    let mut main = vec![r("//.*", 'c'), rn(r"/\*", 'c', "blk")];
    main.extend(extra);
    main.push(r(&format!("{DQ}|{SQ}"), 's'));
    main.push(r(&kw(kws), 'k'));
    main.push(r(&kw(&["true", "false", "null", "nil", "NULL", "nullptr", "None"]), 'n'));
    main.push(r(NUM, 'n'));
    if !types.is_empty() { main.push(r(&kw(types), 't')); }
    main.push(r(r"[A-Z]\w*", 't'));
    main.push(r(r"[A-Za-z_]\w*(?=\s*[(<])", 'f'));
    main.push(r(r"[{}()\[\];,.:?<>=!+\-*/%&|^~]+", 'p'));
    lang(name, exts, &[], Some("//"), vec![("main", main), ("blk", blk("main"))])
}

fn ci(words: &[&str]) -> String { format!(r"(?i:\b(?:{})\b)", words.join("|")) }

pub fn more() -> Vec<Lang> {
    let c_kw = ["if", "else", "for", "while", "do", "switch", "case", "break", "continue", "return", "goto", "default", "struct", "union", "enum", "typedef", "static", "extern", "const", "volatile", "sizeof", "inline", "register"];
    let c_ty = ["int", "char", "float", "double", "void", "long", "short", "unsigned", "signed", "size_t", "bool", "uint8_t", "uint16_t", "uint32_t", "uint64_t", "int8_t", "int16_t", "int32_t", "int64_t"];
    let pre = || r(r"^\s*#\s*\w+(?:\s*<[^>]*>)?", 'm');
    let mut cpp_kw = c_kw.to_vec();
    cpp_kw.extend(["class", "namespace", "template", "typename", "public", "private", "protected", "virtual", "override", "new", "delete", "this", "using", "try", "catch", "throw", "constexpr", "auto", "operator", "friend", "explicit", "noexcept"]);
    let mut cs_kw = vec!["namespace", "using", "class", "struct", "interface", "enum", "public", "private", "protected", "internal", "static", "readonly", "const", "void", "new", "this", "base", "return", "if", "else", "for", "foreach", "while", "do", "switch", "case", "break", "continue", "try", "catch", "finally", "throw", "async", "await", "var", "get", "set", "override", "virtual", "abstract", "sealed", "in", "out", "ref", "is", "as", "null", "partial", "yield", "record"];
    cs_kw.dedup();

    vec![
        lang("PowerShell", &["ps1", "psm1", "psd1"], &[], Some("#"), vec![
            ("main", vec![rn(r"<#", 'c', "blk"), r("#.*", 'c'), r(&format!(r#"{DQ}|{SQ}"#), 's'), r(r"\$\{[^}]*\}|\$\w+(?::\w+)?", 'm'),
                r(&ci(&["if", "else", "elseif", "foreach", "for", "while", "do", "until", "switch", "function", "filter", "param", "return", "try", "catch", "finally", "throw", "begin", "process", "end", "class", "using", "break", "continue", "in", "exit"]), 'k'),
                r(r"\b[A-Z][a-z]+-[A-Z]\w+", 'f'), r(r"(?<![\w$])-[A-Za-z]\w*", 'a'), r(r"\[[\w.,\[\]\s]+\]", 't'), r(NUM, 'n'), r(r"[{}()\[\];,.|=<>!+*/%&]+", 'p')]),
            ("blk", vec![rn(r"#>", 'c', "main"), r(r"[^#]+|#", 'c')]),
        ]),
        clike("C", &["c", "h"], &c_kw, &c_ty, vec![pre()]),
        clike("C++", &["cpp", "cc", "cxx", "hpp", "hh", "hxx", "ino"], &cpp_kw, &c_ty, vec![pre()]),
        clike("C#", &["cs", "csx"], &cs_kw, &["int", "string", "bool", "double", "float", "long", "char", "object", "decimal", "byte", "short", "var", "dynamic"], vec![r(r"^\s*#\s*\w+", 'm'), r(r"\[[A-Z]\w*(?:\([^\]]*\))?\]", 'm')]),
        clike("Java", &["java"], &["package", "import", "class", "interface", "enum", "extends", "implements", "public", "private", "protected", "static", "final", "abstract", "void", "new", "this", "super", "return", "if", "else", "for", "while", "do", "switch", "case", "break", "continue", "try", "catch", "finally", "throw", "throws", "instanceof", "synchronized", "volatile", "default", "var", "record"],
            &["int", "long", "double", "float", "boolean", "char", "byte", "short", "String"], vec![r(r"@\w+", 'm')]),
        {
            let mut g = clike("Go", &["go"], &["package", "import", "func", "var", "const", "type", "struct", "interface", "map", "chan", "go", "defer", "return", "if", "else", "for", "range", "switch", "case", "default", "break", "continue", "select", "fallthrough", "goto"],
                &["int", "int8", "int16", "int32", "int64", "uint", "uint8", "uint16", "uint32", "uint64", "float32", "float64", "string", "bool", "byte", "rune", "error", "any"], vec![r(r"`[^`]*`", 's')]);
            g.names = &["go.mod", "go.sum"];
            g
        },
        lang("Lua", &["lua"], &[], Some("--"), vec![
            ("main", vec![rn(r"--\[\[", 'c', "lblk"), r("--.*", 'c'), rn(r"\[\[", 's', "lstr"), r(&format!("{DQ}|{SQ}"), 's'),
                r(&kw(&["and", "break", "do", "else", "elseif", "end", "for", "function", "goto", "if", "in", "local", "not", "or", "repeat", "return", "then", "until", "while"]), 'k'),
                r(&kw(&["true", "false", "nil"]), 'n'), r(NUM, 'n'), r(&kw(&["self", "print", "pairs", "ipairs", "require", "type", "tostring", "tonumber", "table", "string", "math", "os", "io"]), 't'),
                r(r"[A-Za-z_]\w*(?=\s*[(\x22'{])", 'f'), r(r"[{}()\[\];,.:<>=~+\-*/%^#]+", 'p')]),
            ("lblk", vec![rn(r"\]\]", 'c', "main"), r(r"[^\]]+|\]", 'c')]),
            ("lstr", vec![rn(r"\]\]", 's', "main"), r(r"[^\]]+|\]", 's')]),
        ]),
        lang("SQL", &["sql", "ddl", "dml"], &[], Some("--"), vec![
            ("main", vec![r("--.*", 'c'), rn(r"/\*", 'c', "blk"), r(r"'(?:[^']|'')*'", 's'), r(r#""[^"]*"|`[^`]*`|\[[^\]]*\]"#, 'a'),
                r(&ci(&["select", "from", "where", "and", "or", "not", "in", "is", "null", "like", "between", "join", "inner", "left", "right", "full", "outer", "cross", "on", "as", "group", "by", "order", "having", "limit", "offset", "union", "all", "distinct", "insert", "into", "values", "update", "set", "delete", "create", "alter", "drop", "table", "index", "view", "database", "primary", "key", "foreign", "references", "default", "constraint", "unique", "exists", "case", "when", "then", "else", "end", "asc", "desc", "with", "begin", "commit", "rollback", "transaction", "returning", "if"]), 'k'),
                r(&ci(&["int", "integer", "bigint", "smallint", "varchar", "char", "text", "boolean", "bool", "date", "datetime", "timestamp", "float", "double", "decimal", "numeric", "serial", "uuid", "json", "blob"]), 't'),
                r(NUM, 'n'), r(r"\w+(?=\s*\()", 'f'), r(r"[();,.=<>!+*/%]+", 'p')]),
            ("blk", blk("main")),
        ]),
        lang("PHP", &["php", "phtml"], &[], Some("//"), vec![
            ("main", vec![r(r"<\?php|<\?=|\?>", 'm'), r("//.*|#(?!\\[).*", 'c'), rn(r"/\*", 'c', "blk"), r(&format!("{DQ}|{SQ}"), 's'), r(r"\$\w+", 'm'),
                r(&ci(&["function", "class", "interface", "trait", "extends", "implements", "public", "private", "protected", "static", "abstract", "final", "const", "namespace", "use", "new", "return", "if", "else", "elseif", "foreach", "for", "while", "do", "switch", "case", "break", "continue", "try", "catch", "finally", "throw", "echo", "print", "require", "require_once", "include", "include_once", "isset", "unset", "empty", "as", "fn", "match", "global", "array"]), 'k'),
                r(&ci(&["true", "false", "null"]), 'n'), r(NUM, 'n'), r(r"[A-Z]\w*", 't'), r(r"[A-Za-z_]\w*(?=\s*\()", 'f'), r(r"[{}()\[\];,.:?<>=!+\-*/%&|^~]+", 'p')]),
            ("blk", blk("main")),
        ]),
        lang("Ruby", &["rb", "rake", "gemspec", "ru"], &["gemfile", "rakefile"], Some("#"), vec![
            ("main", vec![rn(r"^=begin", 'c', "rdoc"), r("#.*", 'c'), r(&format!("{DQ}|{SQ}"), 's'), r(r":\w+[?!]?|:\x22[^\x22]*\x22", 'm'), r(r"@@?\w+|\$\w+", 'a'),
                r(&kw(&["def", "end", "class", "module", "if", "elsif", "else", "unless", "while", "until", "for", "in", "do", "begin", "rescue", "ensure", "return", "yield", "self", "super", "then", "case", "when", "and", "or", "not", "require", "require_relative", "include", "extend", "attr_reader", "attr_writer", "attr_accessor", "private", "protected", "public", "lambda", "proc", "puts", "raise"]), 'k'),
                r(&kw(&["true", "false", "nil"]), 'n'), r(NUM, 'n'), r(r"[A-Z]\w*", 't'), r(r"[a-z_]\w*[?!]?(?=\s*[(])", 'f'), r(r"[{}()\[\];,.<>=!+\-*/%&|^~?]+", 'p')]),
            ("rdoc", vec![rn(r"^=end.*", 'c', "main"), r(r".+", 'c')]),
        ]),
        clike("Kotlin", &["kt", "kts"], &["package", "import", "class", "interface", "object", "fun", "val", "var", "return", "if", "else", "for", "while", "do", "when", "is", "in", "as", "try", "catch", "finally", "throw", "override", "open", "abstract", "sealed", "data", "enum", "companion", "private", "public", "protected", "internal", "suspend", "lateinit", "by", "init", "this", "super", "typealias", "inline"],
            &["Int", "Long", "Double", "Float", "Boolean", "String", "Char", "Unit", "Any", "Nothing"], vec![r(r"@\w+", 'm')]),
        clike("Swift", &["swift"], &["import", "class", "struct", "enum", "protocol", "extension", "func", "var", "let", "return", "if", "else", "guard", "for", "while", "repeat", "switch", "case", "default", "break", "continue", "in", "is", "as", "try", "catch", "throw", "throws", "init", "deinit", "self", "super", "static", "public", "private", "internal", "fileprivate", "open", "override", "final", "lazy", "weak", "mutating", "async", "await", "defer", "where", "some", "any"],
            &["Int", "Double", "Float", "Bool", "String", "Character", "Void", "Any", "Array", "Dictionary", "Set", "Optional"], vec![r(r"@\w+|#\w+", 'm')]),
        {
            let jsx = r(r"</?[A-Za-z][\w.]*|/?>", 't');
            let mut main = vec![r("//.*", 'c'), rn(r"/\*", 'c', "blk"), r(&format!("{DQ}|{SQ}"), 's'), rn("`", 's', "tpl"), jsx,
                r(&kw(&["const", "let", "var", "function", "return", "if", "else", "for", "while", "do", "switch", "case", "break", "continue", "new", "class", "extends", "import", "export", "from", "default", "async", "await", "try", "catch", "finally", "throw", "typeof", "instanceof", "in", "of", "this", "super", "static", "interface", "type", "enum", "as", "readonly", "public", "private", "protected"]), 'k'),
                r(&kw(&["true", "false", "null", "undefined"]), 'n'), r(NUM, 'n'), r(&kw(&["string", "number", "boolean", "void", "any", "unknown", "never"]), 't'), r(r"[A-Z][\w$]*", 't'),
                r(r"[A-Za-z_$][\w$]*(?=\s*[(<])", 'f'), r(r"[\w-]+(?==)", 'a'), r(r"[{}()\[\];,.:?=!+\-*/%&|^~]+", 'p')];
            let _ = &mut main;
            lang("JSX / TSX", &["jsx", "tsx"], &[], Some("//"), vec![
                ("main", main), ("blk", blk("main")),
                ("tpl", vec![r(r"\\.", 'm'), rn("`", 's', "main"), r(r"\$\{", 'm'), r(r"[^`\\$]+|\$", 's')]),
            ])
        },
        lang("Vue / Svelte", &["vue", "svelte"], &[], None, vec![
            ("main", vec![rn("<!--", 'c', "hc"), rn(r"</?[\w:.-]+", 't', "tag"), r(r"\{\{[^}]*\}\}|\{[#/:@]?[^}]*\}", 'f'), r(r"&#?\w+;", 'm')]),
            ("tag", vec![rn(r"/?>", 't', "main"), r(r"[:@#v][\w:.@-]*(?=\s*=)|[\w:@.-]+(?=\s*=)", 'a'), r(&format!("{DQ}|{SQ}"), 's'), r(r"\{[^}]*\}", 'f'), r("=", 'p')]),
            ("hc", vec![rn("-->", 'c', "main"), r(r"[^-]+|-", 'c')]),
        ]),
        lang("SCSS", &["scss", "sass", "less"], &[], Some("//"), vec![
            ("main", vec![r("//.*", 'c'), rn(r"/\*", 'c', "blk"), r(&format!("{DQ}|{SQ}"), 's'), r(r"#[\da-fA-F]{3,8}\b", 'n'), r(r"@[\w-]+", 'k'), r(r"\$[\w-]+|--[\w-]+", 'm'), r(r"&", 'k'), r(r"[\w-]+(?=\s*:)", 'a'),
                r(r"-?\d*\.?\d+(?:px|em|rem|%|vh|vw|s|ms|deg|fr|ch)?", 'n'), r(r"[.#%][\w-]+", 't'), r(r"[\w-]+(?=\()", 'f'), r(r"[{}();:,>+~*]", 'p')]),
            ("blk", blk("main")),
        ]),
        lang("GraphQL", &["graphql", "gql", "graphqls"], &[], Some("#"), vec![
            ("main", vec![r("#.*", 'c'), r(r#""""[^"]*(?:"{1,2}[^"]+)*""""#, 's'), r(DQ, 's'), r(&kw(&["query", "mutation", "subscription", "fragment", "on", "type", "input", "enum", "interface", "union", "scalar", "schema", "extend", "directive", "implements", "repeatable"]), 'k'),
                r(&kw(&["true", "false", "null"]), 'n'), r(NUM, 'n'), r(r"\$\w+", 'm'), r(r"@\w+", 'm'), r(r"\b(?:Int|Float|String|Boolean|ID)\b|[A-Z]\w*", 't'), r(r"\w+(?=\s*[:(])", 'a'), r(r"[{}()\[\]:!=|&,]", 'p')]),
        ]),
        lang("Dockerfile", &["dockerfile"], &["dockerfile", "containerfile"], Some("#"), vec![
            ("main", vec![r("#.*", 'c'), r(&format!("{DQ}|{SQ}"), 's'), r(r"\$\{?\w+\}?", 'm'), r(r"(?i:^\s*(?:FROM|RUN|CMD|LABEL|MAINTAINER|EXPOSE|ENV|ADD|COPY|ENTRYPOINT|VOLUME|USER|WORKDIR|ARG|ONBUILD|STOPSIGNAL|HEALTHCHECK|SHELL)\b)", 'k'), r(r"(?i:\bAS\b)", 'k'), r(r"--[\w-]+", 'a'), r(NUM, 'n')]),
        ]),
        lang("ENV", &["env"], &[".env", ".env.local", ".env.example", ".env.production", ".env.development"], Some("#"), vec![
            ("main", vec![r("#.*", 'c'), r(r"^\s*(?:export\s+)?[A-Za-z_][\w.]*(?==)", 'a'), r(&format!("{DQ}|{SQ}"), 's'), r(r"\$\{?\w+\}?", 'm'), r(NUM, 'n')]),
        ]),
        lang("Gitignore", &["gitignore", "dockerignore", "npmignore"], &[".gitignore", ".dockerignore", ".npmignore", ".gitattributes"], Some("#"), vec![
            ("main", vec![r("^#.*", 'c'), r(r"^!", 'k'), r(r"\*\*|\*|\?", 'm'), r(r"\[[^\]]*\]", 't'), r(r"/", 'p')]),
        ]),
        lang("Makefile", &["mk", "mak"], &["makefile", "gnumakefile"], Some("#"), vec![
            ("main", vec![r("#.*", 'c'), r(&format!("{DQ}|{SQ}"), 's'), r(r"\$[({][^)}]*[)}]|\$[@<^?*%]", 'm'), r(r"^\s*(?:ifeq|ifneq|ifdef|ifndef|else|endif|include|-include|define|endef|export|override|vpath)\b", 'k'),
                r(r"^[A-Za-z_][\w.]*(?=\s*[:+?!]?=)", 'a'), r(r"^[\w.%/$()-]+(?:\s+[\w.%/-]+)*(?=\s*:(?!=))", 't'), r(r"[:=|]", 'p')]),
        ]),
        lang("Diff / Patch", &["diff", "patch"], &[], None, vec![
            ("main", vec![r(r"^(?:\+\+\+|---) .*", 'k'), r(r"^@@.*", 'm'), r(r"^(?:diff|index|new file|deleted file|similarity|rename|old mode|new mode).*", 'c'), r(r"^\+.*", '\u{1}'), r(r"^-.*", '\u{2}'), r(r"^[!<>].*", 't')]),
        ]),
        lang("CSV", &["csv", "tsv"], &[], None, vec![
            ("main", vec![r(DQ, 's'), r(r"-?\d[\d.]*(?=[,;\t]|$)", 'n'), r(r"[,;\t|]", 'p')]),
        ]),
        lang("Regex", &["regex", "re"], &[], None, vec![
            ("main", vec![r(r"\\.", 'm'), r(r"\(\?(?:[:=!]|<[=!]|<\w+>|P<\w+>)", 'k'), r(r"\[\^?(?:\\.|[^\]\\])*\]", 's'), r(r"\{\d+(?:,\d*)?\}", 'n'), r(r"[*+?]\??", 'f'), r(r"[()|^$.]", 'p')]),
        ]),
        lang("XML", &["xml", "svg", "xaml", "xsd", "xsl", "xslt", "plist", "csproj", "props", "targets", "rss", "atom", "config", "resx"], &[], None, vec![
            ("main", vec![rn("<!--", 'c', "hc"), rn(r"<!\[CDATA\[", 's', "cdata"), r(r"<\?[\s\S]*?\?>|<\?[^?]*", 'm'), r(r"<![^>]*>", 'm'), rn(r"</?[\w:.-]+", 't', "tag"), r(r"&#?\w+;", 'm')]),
            ("tag", vec![rn(r"/?>", 't', "main"), r(r"[\w:@.-]+(?=\s*=)", 'a'), r(&format!("{DQ}|{SQ}"), 's'), r("=", 'p')]),
            ("hc", vec![rn("-->", 'c', "main"), r(r"[^-]+|-", 'c')]),
            ("cdata", vec![rn(r"\]\]>", 's', "main"), r(r"[^\]]+|\]", 's')]),
        ]),
    ]
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_language_compiles_and_tokenises() {
        for l in crate::syntax::all() {
            let (t, _) = crate::syntax::line(l, "let x = \"s\" // c 42 @@ -1 +1 @@", "main");
            assert!(!t.is_empty(), "{}", l.name);
        }
    }
    #[test]
    fn detects_lua_in_a_txt() {
        let lua: Vec<String> = "local t = {}\nfor i, v in ipairs(t) do\n  if v ~= nil then\n    print(v)\n  end\nend".lines().map(String::from).collect();
        let d = crate::detect::detect(&lua, 0).map(|i| crate::syntax::all()[i].name);
        assert_eq!(d, Some("Lua"));
        let py: Vec<String> = "import os\n\ndef main():\n    print(os.getcwd())\n\nif __name__ == '__main__':\n    main()".lines().map(String::from).collect();
        let py_i = crate::syntax::for_file("a.py");
        assert_eq!(crate::detect::detect(&py, py_i), None);
        let lua_in_py = crate::detect::detect(&lua, py_i).map(|i| crate::syntax::all()[i].name);
        assert_eq!(lua_in_py, Some("Lua"));
    }
}
