use fuzzy::StringMatch;
use gpui::SharedString;

mod dropdown;
mod font_picker;
mod icon_theme_picker;
mod input_field;
mod number_field;
mod ollama_model_picker;
mod section_items;
mod theme_picker;

pub use dropdown::*;
pub use font_picker::font_picker;
pub use icon_theme_picker::icon_theme_picker;
pub use input_field::*;
pub use number_field::*;
pub use ollama_model_picker::render_ollama_model_picker;
pub use section_items::*;
pub use theme_picker::theme_picker;

pub(super) const MAX_SETTINGS_PICKER_OPTIONS: usize = 2048;
pub(super) const MAX_SETTINGS_PICKER_MATCHES: usize = 512;

pub(super) fn bounded_picker_options(
    mut options: Vec<SharedString>,
    current: &SharedString,
) -> Vec<SharedString> {
    if options.len() <= MAX_SETTINGS_PICKER_OPTIONS {
        return options;
    }

    let current_in_prefix = options
        .iter()
        .take(MAX_SETTINGS_PICKER_OPTIONS)
        .any(|option| option == current);

    options.truncate(MAX_SETTINGS_PICKER_OPTIONS);

    if !current.is_empty()
        && !current_in_prefix
        && !options.iter().any(|option| option == current)
        && let Some(last_option) = options.last_mut()
    {
        *last_option = current.clone();
    }

    options
}

pub(super) fn bounded_picker_matches<'a>(
    options: impl Iterator<Item = (usize, &'a SharedString)>,
) -> Vec<StringMatch> {
    options
        .take(MAX_SETTINGS_PICKER_MATCHES)
        .map(|(index, option)| StringMatch {
            candidate_id: index,
            string: option.to_string(),
            positions: Vec::new(),
            score: 0.0,
        })
        .collect()
}
