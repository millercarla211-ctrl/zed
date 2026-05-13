use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::{Deserialize as SerdeDeserialize, Serialize as SerdeSerialize};

use crate::{
    provider_catalog::ProviderCatalogPlan, runtime::DeviceProfile, workspace::DxProjectStatus,
};

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
pub enum HostSurface {
    Dx,
    FlowApp,
    ZeroclawFork,
    CodexFork,
    ZedFork,
    Desktop,
    AndroidNative,
    IosNative,
    Tauri,
    Flutter,
    BrowserWasm,
    Vps,
    RaspberryPi,
    Watch,
    Tv,
    Tablet,
    CustomRustHost,
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
pub enum IntegrationMode {
    FullRuntime,
    FeatureSlice,
    RustCrate,
    FfiBridge,
    WasmBridge,
    SidecarService,
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
pub enum FlowSubsystem {
    RuntimeBroker,
    DeviceProfiling,
    LocalInference,
    WakeWords,
    VoiceDictation,
    Grammar,
    ProviderRouting,
    RemoteAuth,
    Metasearch,
    Serializer,
    PromptCache,
    LongContext,
    DeviceControl,
    Conversion,
    CommunityIndex,
    BrowserRuntime,
    ForgeSync,
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
pub enum AdjacentProjectKind {
    Metasearch,
    Serializer,
    Rlm,
    Providers,
    Forge,
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
pub enum ProviderAuthKind {
    ApiKey,
    OAuth,
    BrowserCookie,
    LiteLlm,
    ModelsDevCatalog,
    UserPremium,
    FreeOfferPool,
    LocalFallback,
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
pub struct AdjacentProject {
    pub key: String,
    pub kind: AdjacentProjectKind,
    pub root_dir: String,
    pub detected: bool,
    pub purpose: String,
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
pub struct ProviderStrategy {
    pub folder_present: bool,
    pub auth_kinds: Vec<ProviderAuthKind>,
    pub auto_switch_local_and_remote: bool,
    pub notes: Vec<String>,
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
pub struct SerializerStrategy {
    pub folder_present: bool,
    pub format_name: String,
    pub uses_rkyv: bool,
    pub uses_memmap: bool,
    pub notes: Vec<String>,
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
pub struct SearchStrategy {
    pub folder_present: bool,
    pub notes: Vec<String>,
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
pub struct LongContextStrategy {
    pub folder_present: bool,
    pub notes: Vec<String>,
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
pub struct ForgeStrategy {
    pub folder_present: bool,
    pub supports_multi_remote: bool,
    pub notes: Vec<String>,
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
pub struct FlowLibraryBlueprint {
    pub host: HostSurface,
    pub integration_mode: IntegrationMode,
    pub device_profile: DeviceProfile,
    pub workspace_projects: Vec<DxProjectStatus>,
    pub core_subsystems: Vec<FlowSubsystem>,
    pub optional_subsystems: Vec<FlowSubsystem>,
    pub adjacent_projects: Vec<AdjacentProject>,
    pub provider_strategy: ProviderStrategy,
    pub provider_catalog_plan: ProviderCatalogPlan,
    pub serializer_strategy: SerializerStrategy,
    pub search_strategy: SearchStrategy,
    pub long_context_strategy: LongContextStrategy,
    pub forge_strategy: ForgeStrategy,
    pub notes: Vec<String>,
}
