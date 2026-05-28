const MAX_BOUNDED_ITEM_CHARS: usize = 120;
const BOUNDED_ITEM_TRUNCATION_MARKER: &str = "...";

pub(crate) fn bounded_items(items: &[String], max: usize, empty: &'static str) -> String {
    let present = items
        .iter()
        .map(|value| compact_bounded_item(value))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();

    if present.is_empty() {
        return empty.to_string();
    }

    let hidden = present.len().saturating_sub(max);
    let mut values = present.into_iter().take(max).collect::<Vec<_>>();
    if hidden > 0 {
        values.push(format!("+{hidden} more"));
    }
    values.join(", ")
}

fn compact_bounded_item(value: &str) -> String {
    let compacted = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compacted.chars().count() <= MAX_BOUNDED_ITEM_CHARS {
        return compacted;
    }

    let marker_chars = BOUNDED_ITEM_TRUNCATION_MARKER.chars().count();
    let prefix = compacted
        .chars()
        .take(MAX_BOUNDED_ITEM_CHARS - marker_chars)
        .collect::<String>();
    format!("{prefix}{BOUNDED_ITEM_TRUNCATION_MARKER}")
}

pub(crate) fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn bounded_items_ignores_blank_values() {
        assert_eq!(bounded_items(&[], 3, "No rows"), "No rows");
        assert_eq!(
            bounded_items(&strings(&["", "  "]), 3, "No rows"),
            "No rows"
        );
        assert_eq!(
            bounded_items(&strings(&[" first ", "", "second"]), 3, "No rows"),
            "first, second"
        );
    }

    #[test]
    fn bounded_items_compacts_and_caps_individual_values() {
        let long = "x".repeat(MAX_BOUNDED_ITEM_CHARS + 20);
        let items = vec!["  alpha \t beta\n".into(), long, "gamma   delta".into()];
        let expected_long = format!(
            "{}{}",
            "x".repeat(MAX_BOUNDED_ITEM_CHARS - BOUNDED_ITEM_TRUNCATION_MARKER.len()),
            BOUNDED_ITEM_TRUNCATION_MARKER
        );

        assert_eq!(
            bounded_items(&items, 2, "No rows"),
            format!("alpha beta, {expected_long}, +1 more")
        );
    }

    #[test]
    fn bounded_items_counts_overflow_after_blank_values_are_removed() {
        let items = strings(&["alpha", "", " beta   two ", "gamma"]);
        assert_eq!(
            bounded_items(&items, 2, "No rows"),
            "alpha, beta two, +1 more"
        );
        assert_eq!(
            bounded_items(&strings(&["alpha", "beta"]), 0, "No rows"),
            "+2 more"
        );
    }

    #[test]
    fn yes_no_labels_boolean_values() {
        assert_eq!(yes_no(true), "yes");
        assert_eq!(yes_no(false), "no");
    }
}
