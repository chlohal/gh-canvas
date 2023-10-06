mod obsidian_style_settings;
mod obsidian_vault;

use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;

use crate::{
    obsidian_style_settings::StyleSettingsCss,
    obsidian_vault::{ObsidianTheme::Light, ObsidianVault},
};

const DEFAULT_FONT_SIZE: i32 = 18;
const DEFAULT_ZOOM_FACTOR: f64 = 1.;//0.9128709291752769;
const DEFAULT_MONO_FONT: &'static str = "Fira Code Retina";
const DEFAULT_H1_WEIGHT: u32 = 800;
const DEFAULT_H2_WEIGHT: u32 = 800;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    let font_size = args.font_size.unwrap_or(DEFAULT_FONT_SIZE);
    let zoom_factor = args.zoom_factor.unwrap_or(DEFAULT_ZOOM_FACTOR);
    let mono_font = args.mono_font.unwrap_or(DEFAULT_MONO_FONT.into());
    let h1_weight = args.h1_weight.unwrap_or(DEFAULT_H1_WEIGHT);
    let h2_weight = args.h2_weight.unwrap_or(DEFAULT_H2_WEIGHT);

    let file = std::fs::canonicalize(args.file)?;

    let input_md = std::fs::read_to_string(&file)
        .with_context(|| format!("Couldn't read Markdown from {}", file.to_string_lossy()))?;

    let body = html_body_of_md(&input_md);

    let vault =
        ObsidianVault::vault_of_file(&file)?.ok_or("Couldn't find Obsidian vault folder")?;

    let app_css = include_str!("./asset/app.css");

    let StyleSettingsCss {
        theme_css,
        style_overrides,
        body_classes,
    } = vault.style_css(&Light)?.unwrap_or_default();

    let html_total = format!(
        r#"<!DOCTYPE html>
    <html>
    <head>
        <meta charset="UTF-8">
        <style>
            {app_css}
        </style>
        <style>
            {theme_css}
        </style>
        <style>
            {style_overrides}
        </style>

        <style>
            :root {{
                overflow: unset;
            }}
            .markdown-preview-view {{
                overflow: unset;
            }}
            body {{
                overflow: unset;
                --file-margins: 0;
                --background-primary: #fff !important;
            }}
            body.theme-light {{
                --h1-weight: {h1_weight};
                --h2-weight: {h2_weight};
            }}
        </style>

        <style>
            @page {{
                margin: 0.65in;
                padding: 0;
                size: 8.5in 11in;
            }}   
        </style>

        <link rel='preconnect' href='https://fonts.googleapis.com'>
        <link rel='preconnect' href='https://fonts.gstatic.com' crossorigin>
        <link href='https://fonts.googleapis.com/css2?family=Inter:wght@100;200;300;400;500;600;700;800;900&display=block' rel='stylesheet'>
    </head>
    <body class='{body_classes}' style="--font-text-size: {font_size}px; --zoom-factor: {zoom_factor}; --font-monospace-override: &quot;{mono_font}&quot;;">
        <div class="print">
            <div class="markdown-rendered markdown-preview-view show-properties">
                {body}
            </main>
        </div>
    </body>
    </html>
    "#
    );

    println!("{}", html_total);

    Ok(())
}

#[derive(Parser, Debug)]
struct CliArgs {
    file: PathBuf,
    #[arg(long)]
    font_size: Option<i32>,
    #[arg(long)]
    zoom_factor: Option<f64>,
    #[arg(long)]
    mono_font: Option<String>,
    #[arg(long)]
    h1_weight: Option<u32>,
    #[arg(long)]
    h2_weight: Option<u32>,
}

fn html_body_of_md(input: &String) -> String {
    markdown::to_html_with_options(
        input,
        &markdown::Options {
            parse: markdown::ParseOptions {
                constructs: markdown::Constructs::gfm(),
                gfm_strikethrough_single_tilde: false,
                math_text_single_dollar: true,
                mdx_expression_parse: None,
                mdx_esm_parse: None,
            },
            compile: markdown::CompileOptions {
                allow_dangerous_html: true,
                allow_dangerous_protocol: true,
                default_line_ending: markdown::LineEnding::LineFeed,
                gfm_footnote_label: Some("".to_string()),
                gfm_footnote_label_tag_name: Some("hr".to_string()),
                gfm_footnote_label_attributes: Some("".to_string()),
                gfm_footnote_back_label: None,
                gfm_footnote_clobber_prefix: None,
                gfm_task_list_item_checkable: true,
                gfm_tagfilter: true,
            },
        },
    )
    .unwrap()
}
