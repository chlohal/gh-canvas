use std::{collections::HashMap, io::Error, str::Chars};

use csscolorparser::Color;
use serde::Deserialize;
use serde_json::Value;

use crate::obsidian_vault::{ObsidianTheme, ObsidianVault};

#[derive(Default)]
pub struct StyleSettingsCss {
    pub theme_css: String,
    pub style_overrides: String,
    pub body_classes: String,
}

const START_SIGIL: &'static str = "/* @settings";
const END_SIGIL: &'static str = "*/";

pub fn get_style_settings_css(
    vault: &ObsidianVault,
    theme_css: String,
    theme_variant: &ObsidianTheme,
) -> Result<StyleSettingsCss, Error> {
    let style_setting_values = style_settings_map(&vault, None, theme_variant)?.unwrap_or_default();

    let mut root_css = String::new();

    let mut body_classes = Vec::new();

    body_classes.push("theme-light".to_string());

    for comment in css_settings_comments(&theme_css)
        .filter(|x| x.starts_with(START_SIGIL) && x.ends_with(END_SIGIL))
    {
        let start = START_SIGIL.len();
        let end = comment.len() - END_SIGIL.len();
        let content = &comment[start..end].trim().replace("\t", "    ");

        let Ok(style_setting_config) = serde_yaml::from_str::<StyleSettings>(content) else {
            continue;
        };

        for setting_config in style_setting_config.settings {
            let Some(value) = style_setting_values
                .get(&(
                    style_setting_config.id.to_string(),
                    setting_config.id.to_string(),
                ))
                .map(|x| (*x).as_str())
            else {
                continue;
            };

            match setting_config.r#type {
                "class-toggle" => {
                    if value == "true" {
                        body_classes.push(setting_config.id.to_string());
                    }
                }
                "class-select" => {
                    body_classes.push(value.to_string());
                }
                "variable-select"
                | "variable-text"
                | "variable-number-slider"
                | "variable-number" => {
                    root_css = add_variable_with_value(
                        root_css,
                        &setting_config.id,
                        value.to_string() + setting_config.format.unwrap_or_default(),
                    );
                }
                "variable-color" | "variable-themed-color" => {
                    root_css = add_color_format(root_css, &setting_config, value);
                }
                _ => {}
            }
        }
    }

    Ok(StyleSettingsCss {
        body_classes: body_classes.join(" "),
        theme_css,
        style_overrides: format!("body.{} {{ {} }}", theme_variant.classname(), root_css),
    })
}

fn add_variable_with_value(mut root_css: String, id: &str, value: String) -> String {
    root_css += "--";
    root_css += id;
    root_css += ":";
    root_css += value.as_str();
    root_css += ";";

    root_css
}

fn add_color_format(
    mut root_css: String,
    setting_config: &SingleStyleSetting<'_>,
    value: &str,
) -> String {
    let color = Color::try_from(value).unwrap_or(Color::new(0., 0., 0., 0.));

    let id = setting_config.id;

    return match setting_config.format {
        Some("hex") => add_variable_with_value(root_css, id, color.to_hex_string()),
        Some("rgb") => add_variable_with_value(root_css, id, color.to_rgb_string()),
        Some("rgb-values") => add_variable_with_value(
            root_css,
            id,
            color.to_rgba8().map(|x| x.to_string()).join(" "),
        ),
        Some("hsl-values") => match color.to_hsla() {
            (h, s, l, a) => {
                let s = s * 100.;
                let l = l * 100.;
                add_variable_with_value(root_css, id, format!("{h} {s}% {l}% {a}"))
            }
        },

        Some("rgb-split") => {
            let (r, g, b, a) = color.to_linear_rgba_u8();
            root_css = add_variable_with_value(root_css, format!("{id}-r").as_str(), r.to_string());
            root_css = add_variable_with_value(root_css, format!("{id}-b").as_str(), b.to_string());
            root_css = add_variable_with_value(root_css, format!("{id}-g").as_str(), g.to_string());
            root_css = add_variable_with_value(root_css, format!("{id}-a").as_str(), a.to_string());

            root_css
        }

        Some("hsl-split") => {
            let (h, s, l, a) = color.to_hsla();
            root_css = add_variable_with_value(root_css, format!("{id}-h").as_str(), h.to_string());
            root_css = add_variable_with_value(
                root_css,
                format!("{id}-s").as_str(),
                (s * 100.).to_string() + "%",
            );
            root_css = add_variable_with_value(
                root_css,
                format!("{id}-l").as_str(),
                (l * 100.).to_string() + "%",
            );
            root_css = add_variable_with_value(root_css, format!("{id}-a").as_str(), a.to_string());

            root_css
        }
        None | Some(_) => add_variable_with_value(root_css, id, color.to_hex_string()),
    };
}

#[derive(Deserialize, Debug)]
struct StyleSettings<'a> {
    id: &'a str,
    settings: Vec<SingleStyleSetting<'a>>,
}

#[derive(Deserialize, Debug)]
struct SingleStyleSetting<'a> {
    id: &'a str,
    r#type: &'a str,
    format: Option<&'a str>,
}

struct Comments<'a>(Chars<'a>, String, i32);

impl<'a> Iterator for Comments<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let Comments(chars, buf, buffering) = self;

        while let Some(c) = chars.next() {
            if *buffering == 0 && c == '/' {
                *buffering = 1;
                buf.clear();
                buf.push(c);
            } else if *buffering > 0 {
                *buffering += 1;
                buf.push(c);
            }

            if *buffering > 0 && buf.ends_with("*/") {
                *buffering = 0;
                return Some(buf.clone());
            }
        }

        None
    }
}

fn css_settings_comments(theme_css: &String) -> Comments<'_> {
    return Comments(theme_css.chars(), String::new(), 0);
}

fn style_settings_map(
    vault: &ObsidianVault,
    category: Option<&str>,
    theme: &ObsidianTheme,
) -> Result<Option<HashMap<(String, String), String>>, Error> {
    let dir = &vault.0;

    let style_settings_file = dir.join("plugins/obsidian-style-settings/data.json");

    if !style_settings_file.exists() {
        return Ok(None);
    }

    let json =
        match serde_json::from_str(std::fs::read_to_string(style_settings_file)?.as_str()).ok() {
            Some(Value::Object(m)) => m,
            Some(_) | None => return Ok(None),
        };

    let mut result = HashMap::new();

    for (key, value) in json {
        let mut terms = key.split("@@");

        let Some(this_category_id) = terms.next() else {
            continue;
        };
        let Some(setting_id) = terms.next() else {
            continue;
        };
        let this_theme = match terms.next() {
            Some("light") => &ObsidianTheme::Light,
            Some("dark") => &ObsidianTheme::Dark,
            _ => theme,
        };

        let value = match value {
            Value::Null => "".into(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s,
            v => v.to_string(),
        };

        let this_category_matches = match category {
            Some(c) => this_category_id == c,
            None => true,
        };

        if this_theme == theme && this_category_matches {
            result.insert(
                (this_category_id.to_string(), setting_id.to_string()),
                value,
            );
        }
    }

    Ok(Some(result))
}
