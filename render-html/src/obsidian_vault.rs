use std::{error::Error, path::PathBuf};

use anyhow::Context;
use serde::Deserialize;

use crate::obsidian_style_settings::{get_style_settings_css, StyleSettingsCss};

pub struct ObsidianVault(pub PathBuf);

const DEFAULT_BODY_CLASSES: [&str; 15] = [
    "mod-linux",
    "is-frameless",
    "is-hidden-frameless",
    "obsidian-app",
    "show-view-header",
    "highlightr-realistic",
    "css-settings-manager",
    "trim-cols",
    "checkbox-circle",
    "maximize-tables",
    "tabs-default",
    "tab-stack-top",
    "minimal-tab-title-visible",
    "is-maximized",
    "is-focused",
];

#[derive(PartialEq, Eq)]
pub enum ObsidianTheme {
    Light,
    Dark,
}

impl ObsidianTheme {
    pub fn classname(&self) -> &'static str {
        match self {
            ObsidianTheme::Light => "theme-light",
            ObsidianTheme::Dark => "theme-dark",
        }
    }
}

impl ObsidianVault {
    pub fn vault_of_file(file: &PathBuf) -> Result<Option<ObsidianVault>, Box<dyn Error>> {
        for folder in file.ancestors().skip(1) {
            for subfile in folder.read_dir()? {
                let subfile = subfile.with_context(|| "Reading directory entry of {folder}")?;
                if subfile.file_name() == ".obsidian" && subfile.file_type()?.is_dir() {
                    return Ok(Some(ObsidianVault(folder.join(subfile.file_name()))));
                }
            }
        }
        Ok(None)
    }

    pub fn appearance(
        &self,
    ) -> Result<Result<ObsidianAppearance, serde_json::Error>, std::io::Error> {
        let file_content = std::fs::File::open(self.0.join("appearance.json"))?;

        return Ok(serde_json::from_reader(file_content));
    }

    pub fn style_css(
        &self,
        theme_variant: &ObsidianTheme,
    ) -> Result<Option<StyleSettingsCss>, Box<dyn Error>> {
        let appearance = self.appearance()??;

        let Some(theme) = appearance.cssTheme else {
            return Ok(None);
        };

        let theme_css =
            std::fs::read_to_string(self.0.join("themes").join(theme).join("theme.css"))?;

        let mut style = get_style_settings_css(self, theme_css, theme_variant)?;

        for class in DEFAULT_BODY_CLASSES {
            style.body_classes += " ";
            style.body_classes += class;
        }

        return Ok(Some(style));
    }
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
pub struct ObsidianAppearance {
    pub baseFontSize: i32,
    pub theme: Option<String>,
    pub cssTheme: Option<String>,
    pub accentColor: Option<String>,
    pub translucency: bool,
    pub monospaceFontFamily: String,
}
