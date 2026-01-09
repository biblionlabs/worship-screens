use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};
use ui::{AutoCompleteManager, MainWindow};

#[derive(Clone)]
struct FuzzyMatch {
    text: String,
    score: i32,
    is_prefix: bool,
}

fn fuzzy_score(pattern: &str, text: &str) -> Option<(i32, bool)> {
    let pattern_lower = pattern.to_lowercase();
    let text_lower = text.to_lowercase();

    if text_lower.starts_with(&pattern_lower) {
        return Some((1000, text.starts_with(pattern)));
    }

    let mut pattern_chars = pattern_lower.chars().peekable();
    let mut text_chars = text_lower.chars().enumerate();
    let mut score = 0;
    let mut last_match_idx = 0;
    let mut consecutive = 0;

    while let Some(p_char) = pattern_chars.peek() {
        let mut found = false;

        while let Some((idx, t_char)) = text_chars.next() {
            if *p_char == t_char {
                pattern_chars.next();

                if idx == last_match_idx + 1 {
                    consecutive += 1;
                    score += 5 + consecutive * 2;
                } else {
                    consecutive = 0;
                    score += 1;
                }

                if idx > 0 {
                    let prev = text_lower.chars().nth(idx - 1).unwrap();
                    if prev == ' ' || prev == '-' || prev == '_' {
                        score += 10;
                    }
                }

                last_match_idx = idx;
                found = true;
                break;
            }
        }

        if !found {
            return None;
        }
    }

    score -= text.len() as i32 / 2;

    Some((score, false))
}

fn fuzzy_filter(all_suggestions: &[String], text: &str) -> Vec<FuzzyMatch> {
    if text.is_empty() {
        return vec![];
    }

    let mut matches: Vec<FuzzyMatch> = all_suggestions
        .iter()
        .filter_map(|s| {
            fuzzy_score(text, s).map(|(score, is_prefix)| FuzzyMatch {
                text: s.clone(),
                score,
                is_prefix,
            })
        })
        .collect();

    matches.sort_by(|a, b| b.score.cmp(&a.score));

    matches
}

fn get_smart_preview(input_text: &str, match_text: &str, is_prefix: bool) -> String {
    if !is_prefix {
        return String::new();
    }

    let input_lower = input_text.to_lowercase();
    let match_lower = match_text.to_lowercase();

    if match_lower.starts_with(&input_lower) {
        match_text.to_string()
    } else {
        String::new()
    }
}

pub fn setup_autocomplete(ui: &MainWindow) {
    {
        let ui_weak = ui.as_weak();

        ui.global::<AutoCompleteManager>()
            .on_text_changed(move |text, all_suggestions| {
                let ui = match ui_weak.upgrade() {
                    Some(ui) => ui,
                    None => return,
                };

                let all_sug: Vec<String> = all_suggestions.iter().map(|s| s.to_string()).collect();

                let matches = fuzzy_filter(&all_sug, text.as_str());

                let manager = ui.global::<AutoCompleteManager>();

                if !matches.is_empty() {
                    let suggestions: Vec<SharedString> = matches
                        .iter()
                        .map(|m| SharedString::from(m.text.as_str()))
                        .collect();

                    manager.set_active_suggestions(ModelRc::new(VecModel::from(suggestions)));
                    manager.set_active_selected_index(-1);

                    // Preview inteligente: solo si el primer match es prefijo
                    let preview =
                        get_smart_preview(text.as_str(), &matches[0].text, matches[0].is_prefix);
                    manager.set_active_preview_text(preview.into());
                } else {
                    manager.set_active_suggestions(ModelRc::new(VecModel::from(vec![])));
                    manager.set_active_selected_index(-1);
                    manager.set_active_preview_text("".into());
                }
            });
    }

    {
        let ui_weak = ui.as_weak();

        ui.global::<AutoCompleteManager>()
            .on_item_selected(move |selected_text| {
                let ui = match ui_weak.upgrade() {
                    Some(ui) => ui,
                    None => return,
                };

                let manager = ui.global::<AutoCompleteManager>();
                manager.set_active_text_to_apply(selected_text);
            });
    }
}
