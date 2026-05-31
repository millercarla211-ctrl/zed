pub(super) struct GroupingEfficiency {
    pub(super) raw_atomic_bytes: Option<usize>,
    pub(super) grouped_reference_bytes: Option<usize>,
    pub(super) grouping_savings_bytes: Option<isize>,
    pub(super) recommended_representation: Option<&'static str>,
}

pub(super) fn grouping_efficiency(
    alias: Option<&str>,
    utilities: &[String],
    candidate_token_count: Option<usize>,
) -> GroupingEfficiency {
    if utilities.is_empty() {
        return GroupingEfficiency {
            raw_atomic_bytes: None,
            grouped_reference_bytes: None,
            grouping_savings_bytes: None,
            recommended_representation: None,
        };
    }

    let raw_atomic_bytes = utilities.iter().map(|utility| utility.len()).sum::<usize>()
        + utilities.len().saturating_sub(1);
    let grouped_reference_bytes = alias.map(|alias| alias.len() + 2);
    let grouping_savings_bytes =
        grouped_reference_bytes.map(|grouped| raw_atomic_bytes as isize - grouped as isize);
    let recommended_representation = match (alias, grouping_savings_bytes) {
        (Some(_), Some(savings)) if savings > 0 => Some("grouped_reference"),
        (Some(_), Some(_)) => Some("atomic_utilities"),
        (None, _) if candidate_token_count.is_some() => Some("group_candidate_needs_alias"),
        _ => Some("atomic_utilities"),
    };

    GroupingEfficiency {
        raw_atomic_bytes: Some(raw_atomic_bytes),
        grouped_reference_bytes,
        grouping_savings_bytes,
        recommended_representation,
    }
}
