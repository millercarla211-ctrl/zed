#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofingGoal {
    Grammar,
    Clarity,
    Tone,
    Concision,
    CitationSupport,
    FactCheck,
    PlagiarismScreen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofingSeverity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofingIssue {
    pub goal: ProofingGoal,
    pub severity: ProofingSeverity,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlowProofingPlanner {
    pub profile_name: &'static str,
    pub goals: Vec<ProofingGoal>,
    pub strict_mode: bool,
}

impl FlowProofingPlanner {
    pub fn business_default() -> Self {
        Self {
            profile_name: "business-default",
            goals: vec![
                ProofingGoal::Grammar,
                ProofingGoal::Clarity,
                ProofingGoal::Tone,
                ProofingGoal::Concision,
            ],
            strict_mode: false,
        }
    }

    pub fn academic_default() -> Self {
        Self {
            profile_name: "academic-default",
            goals: vec![
                ProofingGoal::Grammar,
                ProofingGoal::Clarity,
                ProofingGoal::CitationSupport,
                ProofingGoal::FactCheck,
                ProofingGoal::PlagiarismScreen,
            ],
            strict_mode: true,
        }
    }

    pub fn inspect(&self, text: &str) -> Vec<ProofingIssue> {
        let mut issues = Vec::new();

        if text.contains("  ") {
            issues.push(ProofingIssue {
                goal: ProofingGoal::Grammar,
                severity: ProofingSeverity::Low,
                message: "Repeated spaces detected.".to_string(),
                suggestion: Some("Collapse consecutive spaces into one.".to_string()),
            });
        }

        if text.contains(" i ") {
            issues.push(ProofingIssue {
                goal: ProofingGoal::Grammar,
                severity: ProofingSeverity::Medium,
                message: "Standalone lowercase 'i' detected.".to_string(),
                suggestion: Some("Replace it with uppercase 'I'.".to_string()),
            });
        }

        if max_sentence_words(text) > 34 {
            issues.push(ProofingIssue {
                goal: ProofingGoal::Clarity,
                severity: ProofingSeverity::Medium,
                message: "At least one sentence is likely too long for instant reading."
                    .to_string(),
                suggestion: Some("Split the sentence into two shorter sentences.".to_string()),
            });
        }

        if missing_terminal_punctuation(text) {
            issues.push(ProofingIssue {
                goal: ProofingGoal::Grammar,
                severity: ProofingSeverity::Low,
                message: "The draft ends without terminal punctuation.".to_string(),
                suggestion: Some("Add a period, question mark, or exclamation mark.".to_string()),
            });
        }

        if self.goals.contains(&ProofingGoal::CitationSupport) && mentions_claim_language(text) {
            issues.push(ProofingIssue {
                goal: ProofingGoal::CitationSupport,
                severity: ProofingSeverity::Medium,
                message: "The draft contains claim language that may need a source citation."
                    .to_string(),
                suggestion: Some("Attach a source or quote before publishing.".to_string()),
            });
        }

        if self.goals.contains(&ProofingGoal::FactCheck) && mentions_absolute_language(text) {
            issues.push(ProofingIssue {
                goal: ProofingGoal::FactCheck,
                severity: ProofingSeverity::High,
                message: "Absolute phrasing detected; verify the claim before sending.".to_string(),
                suggestion: Some("Replace absolutes with evidence-backed wording.".to_string()),
            });
        }

        issues
    }
}

fn max_sentence_words(text: &str) -> usize {
    text.split(['.', '!', '?'])
        .map(|sentence| sentence.split_whitespace().count())
        .max()
        .unwrap_or(0)
}

fn missing_terminal_punctuation(text: &str) -> bool {
    text.chars()
        .rev()
        .find(|ch| !ch.is_whitespace())
        .map(|ch| !matches!(ch, '.' | '!' | '?'))
        .unwrap_or(false)
}

fn mentions_claim_language(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "according to",
        "studies show",
        "research proves",
        "reported that",
    ]
    .iter()
    .any(|pattern| lower.contains(pattern))
}

fn mentions_absolute_language(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    ["always", "never", "everyone", "nobody", "guaranteed"]
        .iter()
        .any(|pattern| lower.contains(pattern))
}
