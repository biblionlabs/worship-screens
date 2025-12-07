use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=CHANGELOG.md");

    let changelog = fs::read_to_string("CHANGELOG.md").unwrap_or_default();
    let mut lines = changelog.lines();

    let mut collected = Vec::new();
    let mut in_section = false;

    while let Some(line) = lines.next() {
        if !in_section {
            if line.trim_start().starts_with("## [") {
                in_section = true;
                collected.push(line);
            }
        } else {
            if line.trim_start().starts_with("## [") {
                break;
            } else {
                collected.push(line);
            }
        }
    }

    let section = collected.join("\n").trim().to_string();
    let escaped = section
        .replace('\\', "\\\\")
        .replace('\r', "")
        .replace('\n', "\\n");

    println!("cargo:rustc-env=LAST_CHANGELOG={escaped}");
}
