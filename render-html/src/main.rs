mod ast_to_html;
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
const DEFAULT_ZOOM_FACTOR: f64 = 1.; //0.9128709291752769;
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
    let properties_css = include_str!("./asset/properties.css");

    let prism_js = include_str!("./asset/prism.js");

    let StyleSettingsCss {
        theme_css,
        style_overrides,
        body_classes,
    } = vault.style_css(&Light)?.unwrap_or_default();

    let html_total = format!(
        r#"<!DOCTYPE html>
    <html>
    <head>
        <meta charset="UTF-8"/>
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
            {properties_css}
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
                margin: 0;
                margin-bottom: 0.65in;
                margin-top: 0.65in;
                padding: 0;
                size: 8.5in 11in;
            }}
            @page:first {{
                margin-top: 0
            }}
            .non-meta-content {{
                margin: 0;
                margin-left: 0.65in;
                margin-right: 0.65in;
                margin-top: 0.65in;
            }}
            .properties ~ .non-meta-content {{
                margin-top: var(--spacing-p);
            }}
        </style>

        <!-- KaTeX (for math)! -->
        <link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.css" integrity="sha384-n8MVd4RsNIU0tAv4ct0nTaAbDJwPJzDEaqSD1odI+WdtXRGWt2kTvGFasHpSy3SV" crossorigin="anonymous">
        <script src="https://cdn.jsdelivr.net/npm/katex@0.16.9/dist/katex.min.js" integrity="sha384-XjKyOOlGwcjNTAIQHIpgOno0Hl1YQqzUOEleOLALmuqehneUG+vnGctmUb0ZY0l8" crossorigin="anonymous"></script>

        <!-- Prism (for syntax highlighting)! -->
        <script>{prism_js}</script>

        <!-- Lucide (for icons)! -->
        <style>
        @font-face {{
            font-family: 'LucideIcons';
            src: url(https://unpkg.com/lucide-static@latest/font/Lucide.ttf) format('truetype');
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
    let ast = markdown::to_mdast(
        input,
        &markdown::ParseOptions {
            constructs: markdown::Constructs {
                frontmatter: true,
                ..markdown::Constructs::gfm()
            },
            gfm_strikethrough_single_tilde: false,
            math_text_single_dollar: true,
            mdx_expression_parse: None,
            mdx_esm_parse: None,
        },
    )
    .unwrap();

    return ast_to_html::ast_to_html(ast);
}
