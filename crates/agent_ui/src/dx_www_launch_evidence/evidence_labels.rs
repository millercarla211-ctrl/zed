pub(crate) fn evidence_score_label(score: Option<u64>, schema: &str) -> String {
    match score {
        Some(score) if score <= 100 => format!("{score}/100"),
        Some(score) => format!("invalid score {score}"),
        None => schema_label(schema),
    }
}

fn schema_label(schema: &str) -> String {
    let schema = schema.trim();
    if schema.is_empty() {
        "unknown".to_string()
    } else {
        schema.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_label_prefers_valid_scores() {
        assert_eq!(evidence_score_label(Some(97), "launch.schema.v1"), "97/100");
    }

    #[test]
    fn score_label_rejects_scores_above_100() {
        assert_eq!(
            evidence_score_label(Some(101), "launch.schema.v1"),
            "invalid score 101"
        );
        assert_eq!(evidence_score_label(Some(999), ""), "invalid score 999");
    }

    #[test]
    fn score_label_trims_blank_schema() {
        assert_eq!(evidence_score_label(None, "  "), "unknown");
        assert_eq!(
            evidence_score_label(None, " launch.schema.v1 "),
            "launch.schema.v1"
        );
    }
}
