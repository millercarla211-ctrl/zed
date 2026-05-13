use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub struct DxProjectStatus {
    pub key: String,
    pub role: String,
    pub completeness_score: u8,
}

pub fn dx_project_statuses() -> Vec<DxProjectStatus> {
    vec![
        DxProjectStatus {
            key: "metasearch".to_string(),
            role: "agent retrieval and search".to_string(),
            completeness_score: 90,
        },
        DxProjectStatus {
            key: "rlm".to_string(),
            role: "long-context preprocessing and recursive reasoning".to_string(),
            completeness_score: 80,
        },
        DxProjectStatus {
            key: "serializer".to_string(),
            role: "token-efficient prompt and context serialization".to_string(),
            completeness_score: 70,
        },
        DxProjectStatus {
            key: "providers".to_string(),
            role: "remote AI provider auth and routing".to_string(),
            completeness_score: 60,
        },
        DxProjectStatus {
            key: "forge".to_string(),
            role: "multi-remote version control for code and media".to_string(),
            completeness_score: 50,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dx_stack_contains_five_major_projects() {
        let projects = dx_project_statuses();
        assert_eq!(projects.len(), 5);
        assert!(projects.iter().any(|project| project.key == "forge"));
    }
}
