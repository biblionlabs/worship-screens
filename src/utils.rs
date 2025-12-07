use std::collections::HashSet;

use slint::SharedString;
use tracing::debug;

pub fn list_system_fonts() -> Vec<SharedString> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let mut db = db
        .faces()
        .map(|f| {
            f.families
                .iter()
                .map(|f| SharedString::from(&f.0))
                .collect::<HashSet<_>>()
        })
        .flatten()
        .collect::<Vec<_>>();
    debug!("Loaded {} fonts from system", db.len());
    db.sort();
    db
}

pub fn parse_last_changelog_to_markdown_lines(raw_env: &str) -> Vec<ui::MarkdownLine> {
    let raw = raw_env.replace("\\n", "\n").replace("\\\\", "\\");
    let mut out: Vec<ui::MarkdownLine> = Vec::new();

    let mut in_code = false;
    let mut code_acc = String::new();

    for line in raw.lines() {
        let trimmed = line.trim_end();

        if in_code {
            if trimmed.trim_start().starts_with("```") {
                out.push(ui::MarkdownLine {
                    kind: ui::MarkdownLineKind::Code,
                    text: SharedString::from(code_acc.clone()),
                    level: 0,
                    bullet: SharedString::from(""),
                });
                code_acc.clear();
                in_code = false;
            } else {
                if !code_acc.is_empty() {
                    code_acc.push('\n');
                }
                code_acc.push_str(trimmed);
            }
            continue;
        }

        if trimmed.trim_start().starts_with("```") {
            in_code = true;
            code_acc.clear();
            continue;
        }

        if trimmed.trim().is_empty() {
            continue;
        }

        let t = trimmed.trim_start();
        if t.starts_with("## ") {
            let content = t[3..].trim();
            out.push(ui::MarkdownLine {
                kind: ui::MarkdownLineKind::H2,
                text: SharedString::from(content),
                level: 2,
                bullet: SharedString::from(""),
            });
        } else if t.starts_with("### ") {
            let content = t[4..].trim();
            out.push(ui::MarkdownLine {
                kind: ui::MarkdownLineKind::H3,
                text: SharedString::from(content),
                level: 3,
                bullet: SharedString::from(""),
            });
        } else if t.starts_with("- ") {
            let content = t[2..].trim();
            out.push(ui::MarkdownLine {
                kind: ui::MarkdownLineKind::Li,
                text: SharedString::from(content),
                level: 0,
                bullet: SharedString::from("-"),
            });
        } else if t.starts_with('*') && t.len() > 1 && t.chars().nth(1) == Some(' ') {
            let content = t[2..].trim();
            out.push(ui::MarkdownLine {
                kind: ui::MarkdownLineKind::Li,
                text: SharedString::from(content),
                level: 0,
                bullet: SharedString::from("•"),
            });
        } else {
            out.push(ui::MarkdownLine {
                kind: ui::MarkdownLineKind::P,
                text: SharedString::from(trimmed),
                level: 0,
                bullet: SharedString::from(""),
            });
        }
    }

    if in_code && !code_acc.is_empty() {
        out.push(ui::MarkdownLine {
            kind: ui::MarkdownLineKind::Code,
            text: SharedString::from(code_acc),
            level: 0,
            bullet: SharedString::from(""),
        });
    }

    out
}
