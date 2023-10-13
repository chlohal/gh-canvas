use std::collections::VecDeque;

use markdown::mdast::{
    BlockQuote, Break, Code, Delete, Emphasis, FootnoteDefinition, FootnoteReference, Heading,
    Html, Image, ImageReference, InlineCode, InlineMath, Link, List, ListItem, Node, Paragraph,
    Root, Strong, Table, TableCell, TableRow, Text, Toml, Yaml,
};
use serde::de;
use serde_yaml::Value;

macro_rules! simple_element {
    ($inner:expr, $string:expr, $definitions:expr) => {{
        for child in $inner {
            ast_to_html_gather_definitions(child, $string, $definitions);
        }
    }};
    ($inner:expr, $element:expr, $string:expr, $definitions:expr) => {{
        *$string += "<";
        *$string += $element;
        *$string += ">";
        simple_element!($inner, $string, $definitions);
        *$string += "</";
        *$string += $element;
        *$string += ">";
    }};
}

const FN_PREFIX: &'static str = "fn-link-";
const FN_REFERENCE_PREFIX: &'static str = "fn-ref-";

pub fn ast_to_html(ast: Node) -> String {
    let mut s = String::new();
    let mut definitions = Definitions::new();

    ast_to_html_gather_definitions(ast, &mut s, &mut definitions);

    s.insert_str(
        0,
        &format!(
            r#"{}<div class="non-meta-content">"#,
            definitions.yaml_meta_html()
        ),
    );
    s += &definitions.footnote_html();
    s += r#"</div>"#;

    return s;
}

pub fn ast_to_html_gather_definitions(
    ast: Node,
    string: &mut String,
    definitions: &mut Definitions,
) {
    match ast {
        Node::Yaml(Yaml { value, .. }) => {
            add_pretty_yaml(value, &mut definitions.yaml_meta);
        }
        Node::Root(Root { children, .. }) => {
            simple_element!(children, string, definitions);
        }
        Node::BlockQuote(BlockQuote { mut children, .. }) => {
            if let Some((callout_type, callout_title)) =
                find_callout_in_children_and_remove(Some(&mut children))
            {
                *string += &format!(r#"<div class="callout" data-callout="{callout_type}">"#);

                let title = callout_title.unwrap_or_else(|| capitalize(&callout_type));

                let icon_classname = find_callout_icon_classname(&callout_type);

                *string += &format!(r#"<div class="callout-title">
                    <div class="callout-icon"><i class="callout-icon-inner {icon_classname}"></i></div> 
                    <div class="callout-title-inner">{title}</div>
                </div>"#);

                *string += r#"<div class="callout-content">"#;
                simple_element!(children, string, definitions);
                *string += r#"</div></div>"#;
            } else {
                simple_element!(children, "blockquote", string, definitions);
            }
        }
        Node::FootnoteDefinition(FootnoteDefinition {
            identifier,
            children,
            ..
        }) => {
            definitions.footnotes +=
                &format!(r#"<li id="{FN_PREFIX}-{identifier}" value="{identifier}">"#);
            simple_element!(
                children,
                &mut definitions.footnotes,
                &mut Definitions::new()
            );
            definitions.footnotes +=
                &format!(r##"<a href="#{FN_REFERENCE_PREFIX}-{identifier}">↩</a>"##);
            definitions.footnotes += "</li>";
        }
        Node::List(List {
            children,
            ordered,
            start,
            ..
        }) => {
            let tag_name = if ordered { "ol" } else { "ul" };
            let start = start.unwrap_or(1);
            *string += &format!(r#"<{tag_name} start="{start}">"#);
            simple_element!(children, string, definitions);
            *string += &format!(r#"</{tag_name}>"#);
        }
        Node::Break(_) => *string += "<br>",
        Node::InlineCode(InlineCode { value, .. }) => {
            *string += "<code>";
            *string += &escape_html_str(value);
            *string += "</code>";
        }
        Node::InlineMath(InlineMath { value, .. }) => {
            *string += &format!(
                "<span>{}</span><script>var target = document.currentScript.previousElementSibling;

            katex.render(target.textContent, target, {{
                throwOnError: false,
                displayMode: false
            }});</script>",
                escape_html_str(value)
            );
        }
        Node::Paragraph(Paragraph { children, .. }) => {
            simple_element!(children, "p", string, definitions);
        }
        Node::Delete(Delete { children, .. }) => {
            simple_element!(children, "del", string, definitions);
        }
        Node::Emphasis(Emphasis { children, .. }) => {
            simple_element!(children, "em", string, definitions);
        }
        Node::FootnoteReference(FootnoteReference {
            identifier, label, ..
        }) => {
            let label = label.as_ref().unwrap_or(&identifier);
            *string += &format!(
                r##"<sup><a id="{FN_REFERENCE_PREFIX}-{identifier}" href="#{FN_PREFIX}-{identifier}">[{label}]</a></sup>"##
            );
        }
        Node::Html(Html { value, .. }) => {
            *string += &value;
        }
        Node::Image(Image {
            alt, url, title, ..
        }) => {
            let title = title.unwrap_or_default();

            *string += &format!(r#"<img src="{url}" alt="{alt}" title="{title}"/>"#);
        }
        Node::Link(Link {
            children,
            title,
            url,
            ..
        }) => {
            let title = title.unwrap_or_default();

            *string += &format!(r#"<a href="{url}" title="{title}">"#);
            simple_element!(children, string, definitions);
            *string += "</a>";
        }
        Node::Strong(Strong { children, .. }) => {
            simple_element!(children, "strong", string, definitions);
        }
        Node::Text(Text { value, .. }) => {
            *string += &escape_html_str(value);
        }
        Node::Code(Code { lang, value, .. }) => {
            let classname = if let Some(lang) = lang {
                format!("language-{lang}")
            } else {
                "".to_string()
            };

            *string += "<pre><code class=\"";
            *string += &classname;
            *string += "\">";
            *string += &escape_html_str(value);
            *string += "</code></pre>";
        }
        Node::Math(markdown::mdast::Math { value, .. }) => {
            *string += &format!(
                "<div>{}</div><script>var target = document.currentScript.previousElementSibling;

            katex.render(target.textContent, target, {{
                throwOnError: false,
                displayMode: true
            }});</script>",
                escape_html_str(value)
            );
        }
        Node::Heading(Heading {
            children, depth, ..
        }) => {
            let heading_element_type = format!("h{depth}");
            simple_element!(children, &heading_element_type, string, definitions);
        }
        Node::ThematicBreak(_) => {
            *string += "<hr>";
        }
        Node::Table(Table { children, .. }) => {
            simple_element!(children, "table", string, definitions);
        }
        Node::TableRow(TableRow { children, .. }) => {
            simple_element!(children, "tr", string, definitions);
        }
        Node::TableCell(TableCell { children, .. }) => {
            simple_element!(children, "td", string, definitions);
        }
        Node::ListItem(ListItem {
            checked, children, ..
        }) => {
            *string += "<li>";
            *string += match checked {
                Some(true) => r#"<input type="checkbox" checked>"#,
                Some(false) => r#"<input type="checkbox">"#,
                None => "",
            };
            simple_element!(children, string, definitions);
            *string += "</li>";
        }
        Node::Toml(Toml { value, .. }) => {
            eprintln!("TOML frontmatter is not supported in gh-canvas");

            *string += r#"<pre><code class="language-toml">"#;
            *string += &escape_html_str(value);
            *string += "</code></pre>"
        }
        Node::Definition(_) | Node::ImageReference(_) | Node::LinkReference(_) => {
            panic!("References and definitions are only supported for footnotes in gh-canvas");
        }
        Node::MdxJsxTextElement(_)
        | Node::MdxTextExpression(_)
        | Node::MdxFlowExpression(_)
        | Node::MdxJsxFlowElement(_)
        | Node::MdxjsEsm(_) => panic!("MDX is not supported by gh-canvas"),
    }
}

fn find_callout_icon_classname(callout_type: &String) -> &'static str {
    match callout_type.as_str() {
        "abstract" | "summary" | "tldr" => "icon-clipboard-list",
        "info" => "icon-info",
        "todo" => "icon-check-circle-2",
        "important" => "icon-flame",
        "tip" | "hint" => "icon-flame",
        "success" | "check" | "done" => "icon-check",
        "question" | "help" | "faq" => "icon-help-circle",
        "warning" | "caution" | "attention" => "icon-alert-triangle",
        "failure" | "fail" | "missing" => "icon-x",
        "danger" | "error" => "icon-zap",
        "bug" => "icon-bug",
        "example" => "icon-list",
        "quote" | "cite" => "icon-quote",
        _ => "lucide-pencil",
    }
}

fn capitalize(s: &String) -> String {
    if s.is_empty() {return String::new();}

    let first_letter = s.chars().next().unwrap().to_ascii_uppercase();
    let rest = s[1..].to_ascii_lowercase();

    let mut result = String::new();
    result.push(first_letter);
    result.extend(rest.chars());

    return result;
    
}

fn find_callout_in_children_and_remove<'a>(
    ast: Option<&'a mut Vec<Node>>,
) -> Option<(String, Option<String>)> {
    let ast = ast?;

    if ast.is_empty() {
        return None;
    }

    let first_elem = std::mem::replace(&mut ast[0], Node::Break(Break { position: None }));

    match first_elem {
        Node::Text(Text {
            mut value,
            position,
        }) => {
            if regex::Regex::new(r#"^\[![a-zA-Z_-]+\][^\n]*\n"#)
                .unwrap()
                .is_match(&value)
            {
                let nl_index = value.chars().position(|x| x == '\n').unwrap();
                let name = value[2..nl_index].to_string();
                value.drain(nl_index..);

                let (name, title) = name.split_at(name.chars().position(|x| x == ']').unwrap());
                let title = &title[1..];

                let title = if title.is_empty() {
                    None
                } else {
                    Some(title.to_string())
                };

                return Some((name.to_string(), title));
            } else {
                ast[0] = Node::Text(Text { value, position });
                return None;
            }
        }
        node => {
            ast[0] = node;
            return find_callout_in_children_and_remove(ast[0].children_mut());
        }
    }
}

fn add_pretty_yaml(value: String, string: &mut String) {
    if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&value) {
        *string += r#"<div class="properties">"#;
        add_pretty_yaml_parsed(yaml, string);
        *string += r#"</div>"#
    } else {
        *string += "<pre>";
        *string += &escape_html_str(value);
        *string += "</pre>";
    }
}

fn add_pretty_yaml_parsed(value: Value, string: &mut String) {
    match value {
        Value::Null => {}
        Value::Bool(b) => {
            *string += if b {
                r#"<input type="checkbox" checked>"#
            } else {
                r#"<input type="checkbox">"#
            };
        }
        Value::Number(n) => {
            *string += &format!("<code>{n}</code>");
        }
        Value::String(s) => {
            *string += &escape_html_str(s);
        }
        Value::Sequence(list) => {
            *string += "<ul>";
            for itm in list {
                *string += "<li>";
                add_pretty_yaml_parsed(itm, string);
                *string += "</li>";
            }
            *string += "</ul>";
        }
        Value::Mapping(map) => {
            *string += "<dl>";
            for (k, v) in map {
                *string += "<dt>";
                add_pretty_yaml_parsed(k, string);
                *string += "</dt><dd>";
                add_pretty_yaml_parsed(v, string);
                *string += "</dd>";
            }
            *string += "</dl>";
        }
        Value::Tagged(tv) => {
            *string += "<strong>";
            *string += &tv.tag.to_string();
            *string += "</strong>";

            add_pretty_yaml_parsed(tv.value, string);
        }
    }
}

fn escape_html_str(input: String) -> String {
    let mut r = String::new();
    for c in input.chars() {
        match c {
            '<' => r += "&lt;",
            '>' => r += "&gt;",
            '"' => r += "&quot;",
            '&' => r += "&amp;",
            _ => r.push(c),
        }
    }
    return r;
}

pub struct Definitions {
    footnotes: String,
    yaml_meta: String,
}

impl Definitions {
    pub fn new() -> Self {
        Definitions {
            footnotes: String::new(),
            yaml_meta: String::new(),
        }
    }
    pub fn yaml_meta_html(&self) -> &String {
        &self.yaml_meta
    }
    pub fn footnote_html(&self) -> String {
        let footnotes = &self.footnotes;
        return format!(r#"<section class="footnotes"><hr><ol>{footnotes}</ol></section>"#);
    }
}
