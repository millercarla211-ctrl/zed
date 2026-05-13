use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub enum ForgeAssetKind {
    Code,
    Audio,
    Video,
    Image,
    Model3d,
    Document,
    Dataset,
    ProjectFile,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerdeSerialize,
    SerdeDeserialize,
    Archive,
    RkyvSerialize,
    RkyvDeserialize,
)]
pub enum ForgeRemoteKind {
    Github,
    Gitlab,
    Bitbucket,
    Youtube,
    Sketchfab,
    Soundcloud,
    Dropbox,
    Gdrive,
    Mega,
    R2,
    S3Compatible,
}

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
pub struct ForgeSyncPlan {
    pub assets: Vec<ForgeAssetKind>,
    pub remotes: Vec<ForgeRemoteKind>,
    pub multi_remote: bool,
    pub notes: Vec<String>,
}

pub struct ForgeBridge;

impl ForgeBridge {
    pub fn for_dx_media_pipeline() -> ForgeSyncPlan {
        ForgeSyncPlan {
            assets: vec![
                ForgeAssetKind::Code,
                ForgeAssetKind::Audio,
                ForgeAssetKind::Video,
                ForgeAssetKind::Image,
                ForgeAssetKind::Model3d,
                ForgeAssetKind::ProjectFile,
            ],
            remotes: vec![
                ForgeRemoteKind::Github,
                ForgeRemoteKind::Gitlab,
                ForgeRemoteKind::Bitbucket,
                ForgeRemoteKind::Youtube,
                ForgeRemoteKind::Sketchfab,
                ForgeRemoteKind::Soundcloud,
                ForgeRemoteKind::R2,
            ],
            multi_remote: true,
            notes: vec![
                "Forge should version both source code and large media assets from the same project graph."
                    .to_string(),
                "A single push should be able to fan out to multiple remotes when project policy allows it."
                    .to_string(),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dx_media_plan_is_multi_remote() {
        let plan = ForgeBridge::for_dx_media_pipeline();
        assert!(plan.multi_remote);
        assert!(plan.remotes.contains(&ForgeRemoteKind::Github));
        assert!(plan.remotes.contains(&ForgeRemoteKind::Youtube));
    }
}
