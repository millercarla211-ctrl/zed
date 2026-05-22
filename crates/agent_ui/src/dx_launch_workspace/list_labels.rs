pub(crate) fn bounded_items(items: &[String], max: usize, empty: &'static str) -> String {
    let present = items
        .iter()
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim())
        .collect::<Vec<_>>();

    if present.is_empty() {
        return empty.to_string();
    }

    let mut values = present
        .iter()
        .take(max)
        .map(|value| value.to_string())
        .collect::<Vec<_>>();
    if present.len() > max {
        values.push(format!("+{} more", present.len() - max));
    }
    values.join(", ")
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
    fn bounded_items_counts_overflow_after_blank_values_are_removed() {
        assert_eq!(
            bounded_items(&strings(&["alpha", "", " beta ", "gamma"]), 2, "No rows"),
            "alpha, beta, +1 more"
        );
        assert_eq!(
            bounded_items(&strings(&["alpha", "beta"]), 0, "No rows"),
            "+2 more"
        );
    }
}
