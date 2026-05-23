#[path = "check_labels/counts.rs"]
mod counts;
#[path = "check_labels/run.rs"]
mod run;

pub(crate) use counts::{check_outcome_label, checked_paths_label, skipped_checks_label};
pub(crate) use run::{check_duration_label, last_run_label_with_generated_at};

#[cfg(test)]
mod check_labels_tests;
