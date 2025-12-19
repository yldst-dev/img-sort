use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const CATEGORY_KEYS: &[CategoryKey] = &[
    CategoryKey::ScreenshotDocument,
    CategoryKey::People,
    CategoryKey::FoodCafe,
    CategoryKey::NatureLandscape,
    CategoryKey::CityStreetTravel,
    CategoryKey::PetsAnimals,
    CategoryKey::ProductsObjects,
    CategoryKey::Other,
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CategoryKey {
    ScreenshotDocument,
    People,
    FoodCafe,
    NatureLandscape,
    CityStreetTravel,
    PetsAnimals,
    ProductsObjects,
    Other,
}

impl CategoryKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            CategoryKey::ScreenshotDocument => "screenshot_document",
            CategoryKey::People => "people",
            CategoryKey::FoodCafe => "food_cafe",
            CategoryKey::NatureLandscape => "nature_landscape",
            CategoryKey::CityStreetTravel => "city_street_travel",
            CategoryKey::PetsAnimals => "pets_animals",
            CategoryKey::ProductsObjects => "products_objects",
            CategoryKey::Other => "other",
        }
    }

    pub fn dir_name_ko(&self) -> &'static str {
        match self {
            CategoryKey::ScreenshotDocument => "스크린샷_문서",
            CategoryKey::People => "사람",
            CategoryKey::FoodCafe => "음식_카페",
            CategoryKey::NatureLandscape => "자연_풍경",
            CategoryKey::CityStreetTravel => "도시_여행",
            CategoryKey::PetsAnimals => "동물",
            CategoryKey::ProductsObjects => "사물",
            CategoryKey::Other => "기타",
        }
    }
}

impl From<&str> for CategoryKey {
    fn from(value: &str) -> Self {
        match value {
            "screenshot_document" => CategoryKey::ScreenshotDocument,
            "people" => CategoryKey::People,
            "food_cafe" => CategoryKey::FoodCafe,
            "nature_landscape" => CategoryKey::NatureLandscape,
            "city_street_travel" => CategoryKey::CityStreetTravel,
            "pets_animals" => CategoryKey::PetsAnimals,
            "products_objects" => CategoryKey::ProductsObjects,
            _ => CategoryKey::Other,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct Scores {
    pub screenshot_document: f32,
    pub people: f32,
    pub food_cafe: f32,
    pub nature_landscape: f32,
    pub city_street_travel: f32,
    pub pets_animals: f32,
    pub products_objects: f32,
    pub other: f32,
}

impl Scores {
    pub fn from_map(map: &HashMap<String, f32>) -> Self {
        let mut s = Scores::default();
        for (k, v) in map {
            match k.as_str() {
                "screenshot_document" => s.screenshot_document = *v,
                "people" => s.people = *v,
                "food_cafe" => s.food_cafe = *v,
                "nature_landscape" => s.nature_landscape = *v,
                "city_street_travel" => s.city_street_travel = *v,
                "pets_animals" => s.pets_animals = *v,
                "products_objects" => s.products_objects = *v,
                "other" => s.other = *v,
                _ => {}
            }
        }
        s.normalize()
    }

    pub fn to_map(&self) -> HashMap<String, f32> {
        HashMap::from([
            ("screenshot_document".into(), self.screenshot_document),
            ("people".into(), self.people),
            ("food_cafe".into(), self.food_cafe),
            ("nature_landscape".into(), self.nature_landscape),
            ("city_street_travel".into(), self.city_street_travel),
            ("pets_animals".into(), self.pets_animals),
            ("products_objects".into(), self.products_objects),
            ("other".into(), self.other),
        ])
    }

    pub fn normalize(mut self) -> Self {
        let sum: f32 = self
            .to_map()
            .values()
            .copied()
            .fold(0.0f32, |acc, v| acc + v);
        let denom = if sum <= 0.0 { 1.0 } else { sum };
        self.screenshot_document /= denom;
        self.people /= denom;
        self.food_cafe /= denom;
        self.nature_landscape /= denom;
        self.city_street_travel /= denom;
        self.pets_animals /= denom;
        self.products_objects /= denom;
        self.other /= denom;
        self
    }

    pub fn top(&self) -> (CategoryKey, f32) {
        let map = self.to_map();
        map.into_iter()
            .fold((CategoryKey::ScreenshotDocument, -1.0f32), |acc, (k, v)| {
                if v > acc.1 {
                    (CategoryKey::from(k.as_str()), v)
                } else {
                    acc
                }
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "default_base_url")]
    pub ollama_base_url: String,
    #[serde(default = "default_model")]
    pub ollama_model: String,
    #[serde(default)]
    pub ollama_think: bool,
    #[serde(default)]
    pub ollama_stream: bool,
    #[serde(default = "default_analysis_resize_enabled")]
    pub analysis_resize_enabled: bool,
    #[serde(default = "default_analysis_max_edge")]
    pub analysis_max_edge: u32,
    #[serde(default = "default_analysis_jpeg_quality")]
    pub analysis_jpeg_quality: u8,
    #[serde(default)]
    pub analysis_value_enabled: bool,
    #[serde(default = "default_analysis_concurrency")]
    pub analysis_concurrency: u32,
    #[serde(default = "default_analysis_engine")]
    pub analysis_engine: AnalysisEngine,
    #[serde(default)]
    pub clip_model_dir: Option<String>,
    #[serde(default = "default_clip_model_file")]
    pub clip_model_file: String,
    #[serde(default = "default_clip_fallback_to_ollama")]
    pub clip_fallback_to_ollama: bool,
    #[serde(default = "default_clip_ep_auto")]
    pub clip_ep_auto: bool,
    #[serde(default = "default_clip_ep_coreml")]
    pub clip_ep_coreml: bool,
    #[serde(default)]
    pub clip_ep_cuda: bool,
    #[serde(default)]
    pub clip_ep_rocm: bool,
    #[serde(default)]
    pub clip_ep_directml: bool,
    #[serde(default)]
    pub clip_ep_openvino: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisEngine {
    Clip,
    Ollama,
}

pub fn default_base_url() -> String {
    "http://127.0.0.1:11434".to_string()
}

pub fn default_model() -> String {
    "qwen2.5vl:7b".to_string()
}

pub fn default_analysis_resize_enabled() -> bool {
    true
}

pub fn default_analysis_max_edge() -> u32 {
    768
}

pub fn default_analysis_jpeg_quality() -> u8 {
    60
}

pub fn default_analysis_concurrency() -> u32 {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
        .max(1);
    cores.min(4)
}

pub fn default_analysis_engine() -> AnalysisEngine {
    AnalysisEngine::Clip
}

pub fn default_clip_fallback_to_ollama() -> bool {
    false
}

pub fn default_clip_model_file() -> String {
    "onnx/model_q4f16.onnx".to_string()
}

pub fn default_clip_ep_auto() -> bool {
    true
}

pub fn default_clip_ep_coreml() -> bool {
    cfg!(target_vendor = "apple")
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            ollama_base_url: default_base_url(),
            ollama_model: default_model(),
            ollama_think: false,
            ollama_stream: false,
            analysis_resize_enabled: default_analysis_resize_enabled(),
            analysis_max_edge: default_analysis_max_edge(),
            analysis_jpeg_quality: default_analysis_jpeg_quality(),
            analysis_value_enabled: false,
            analysis_concurrency: default_analysis_concurrency(),
            analysis_engine: default_analysis_engine(),
            clip_model_dir: None,
            clip_model_file: default_clip_model_file(),
            clip_fallback_to_ollama: default_clip_fallback_to_ollama(),
            clip_ep_auto: default_clip_ep_auto(),
            clip_ep_coreml: default_clip_ep_coreml(),
            clip_ep_cuda: false,
            clip_ep_rocm: false,
            clip_ep_directml: false,
            clip_ep_openvino: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipProviderCapability {
    pub supported: bool,
    pub available: bool,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipAccelCapabilities {
    pub cpu: ClipProviderCapability,
    pub coreml: ClipProviderCapability,
    pub cuda: ClipProviderCapability,
    pub rocm: ClipProviderCapability,
    pub directml: ClipProviderCapability,
    pub openvino: ClipProviderCapability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StreamChunk {
    pub job_id: String,
    pub file_name: String,
    pub delta: String,
    pub done: bool,
    #[serde(default)]
    pub reset: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelOut {
    pub category: CategoryKey,
    pub scores: Scores,
    pub tags_ko: Vec<String>,
    pub caption_ko: String,
    pub text_in_image_ko: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportStatus {
    Pending,
    Success,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoRow {
    pub id: String,
    pub file_name: String,
    pub path: String,
    pub category: CategoryKey,
    pub top_score: f32,
    pub scores: Scores,
    pub tags: Vec<String>,
    pub export_status: ExportStatus,
    pub error_message: Option<String>,
    pub analysis_duration_ms: Option<i64>,
    pub model: Option<String>,
    pub is_valuable: Option<bool>,
    pub valuable_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhotoDetail {
    pub id: String,
    pub file_name: String,
    pub path: String,
    pub category: CategoryKey,
    pub top_score: f32,
    pub scores: Scores,
    pub tags: Vec<String>,
    pub export_status: ExportStatus,
    pub error_message: Option<String>,
    pub analysis_log: Option<String>,
    pub analysis_duration_ms: Option<i64>,
    pub caption: Option<String>,
    pub text_in_image: Option<String>,
    pub model: Option<String>,
    pub is_valuable: Option<bool>,
    pub valuable_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueStats {
    pub valuable: usize,
    pub not_valuable: usize,
    pub unknown: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Idle,
    Running,
    Completed,
    Canceled,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub job_id: String,
    pub status: JobStatus,
    pub current_file: Option<String>,
    pub processed: usize,
    pub total: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DistributionMode {
    AvgScore,
    CountRatio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Distribution {
    pub mode: DistributionMode,
    pub by_category: HashMap<String, f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartAnalysisInput {
    pub source_root: String,
    pub export_root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartAnalysisResult {
    pub job_id: String,
}
