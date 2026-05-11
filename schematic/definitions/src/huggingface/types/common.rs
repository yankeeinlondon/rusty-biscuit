use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RepoType {
    #[default]
    Model,
    Dataset,
    Space,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Pipeline {
    TextClassification,
    TokenClassification,
    QuestionAnswering,
    TextGeneration,
    Text2TextGeneration,
    Summarization,
    Translation,
    FillMask,
    FeatureExtraction,
    Conversational,
    TableQuestionAnswering,
    SentenceSimilarity,
    ZeroShotClassification,
    ImageClassification,
    ObjectDetection,
    ImageSegmentation,
    ImageToImage,
    DepthEstimation,
    VideoClassification,
    ImageFeatureExtraction,
    UnconditionalImageGeneration,
    ZeroShotImageClassification,
    ZeroShotObjectDetection,
    MaskGeneration,
    KeypointDetection,
    TextToImage,
    ImageToText,
    VisualQuestionAnswering,
    DocumentQuestionAnswering,
    TextToVideo,
    ImageTo3d,
    TextTo3d,
    AnyToAny,
    AutomaticSpeechRecognition,
    AudioClassification,
    TextToSpeech,
    AudioToAudio,
    TextToAudio,
    VoiceActivityDetection,
    TabularClassification,
    TabularRegression,
    ReinforcementLearning,
    Robotics,
    GraphMl,
    TimeSeriesForecasting,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Library {
    Transformers,
    Diffusers,
    Pytorch,
    Tensorflow,
    Jax,
    #[serde(rename = "spacy")]
    SpaCy,
    Fastai,
    Keras,
    #[serde(rename = "onnx")]
    Onnx,
    #[serde(rename = "sentence-transformers")]
    SentenceTransformers,
    #[serde(rename = "stable-baselines3")]
    StableBaselines3,
    Sklearn,
    Tensorboard,
    #[serde(rename = "adapter-transformers")]
    AdapterTransformers,
    Peft,
    #[serde(rename = "tflite")]
    TfLite,
    Openvino,
    Coreml,
    Timm,
    #[serde(rename = "gguf")]
    Gguf,
    Mlx,
    Flair,
    Allennlp,
    Espnet,
    Asteroid,
    Speechbrain,
    Fairseq,
    Nemo,
    Paddlepaddle,
    Safetensors,
    Setfit,
    Spanmarker,
    #[serde(rename = "keras-nlp")]
    KerasNlp,
    #[serde(rename = "mlx-lm")]
    MlxLm,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SortField {
    #[default]
    LastModified,
    Likes,
    Downloads,
    Created,
    Trending,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum SortDirection {
    #[serde(rename = "1")]
    Ascending,
    #[default]
    #[serde(rename = "-1")]
    Descending,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RepoVisibility {
    #[default]
    Public,
    Private,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum GatedStatus {
    #[default]
    #[serde(rename = "false")]
    False,
    #[serde(rename = "true")]
    True,
    Auto,
    Manual,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    #[default]
    File,
    Directory,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DiscussionStatus {
    #[default]
    Open,
    Closed,
    Merged,
    Draft,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DiscussionType {
    #[default]
    Discussion,
    #[serde(rename = "pull_request")]
    PullRequest,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpaceStage {
    #[default]
    NoAppFile,
    ConfigError,
    Building,
    BuildError,
    Running,
    RuntimeError,
    Paused,
    Sleeping,
    Deleted,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SpaceSdk {
    #[default]
    Gradio,
    Streamlit,
    Docker,
    Static,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum SpaceHardware {
    #[default]
    #[serde(rename = "cpu-basic")]
    CpuBasic,
    #[serde(rename = "cpu-upgrade")]
    CpuUpgrade,
    #[serde(rename = "t4-small")]
    T4Small,
    #[serde(rename = "t4-medium")]
    T4Medium,
    #[serde(rename = "a10g-small")]
    A10gSmall,
    #[serde(rename = "a10g-large")]
    A10gLarge,
    #[serde(rename = "a10g-largex2")]
    A10gLargeX2,
    #[serde(rename = "a10g-largex4")]
    A10gLargeX4,
    #[serde(rename = "a100-large")]
    A100Large,
    #[serde(rename = "zero-a10g")]
    ZeroA10g,
    #[serde(rename = "v5e-1x1")]
    V5e1x1,
    #[serde(rename = "v5e-2x2")]
    V5e2x2,
    #[serde(rename = "v5e-2x4")]
    V5e2x4,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum InferenceStatus {
    #[serde(rename = "warm")]
    Warm,
    #[serde(rename = "cold")]
    Cold,
    #[default]
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "loading")]
    Loading,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum InferenceProvider {
    #[default]
    HfInference,
    Serverless,
    DedicatedEndpoint,
    ThirdParty,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RepoFile {
    pub rfilename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(rename = "blobId", skip_serializing_if = "Option::is_none")]
    pub blob_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lfs: Option<LfsInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LfsInfo {
    pub sha256: String,
    pub size: u64,
    #[serde(rename = "pointerSize", skip_serializing_if = "Option::is_none")]
    pub pointer_size: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CommitInfo {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Author {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "avatarUrl", skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Tag {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SafetensorsInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharded: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GgufInfo {
    #[serde(rename = "quantization", skip_serializing_if = "Option::is_none")]
    pub quantization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(rename = "bitsPerWeight", skip_serializing_if = "Option::is_none")]
    pub bits_per_weight: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CardData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(rename = "license_name", skip_serializing_if = "Option::is_none")]
    pub license_name: Option<String>,
    #[serde(rename = "license_link", skip_serializing_if = "Option::is_none")]
    pub license_link: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub language: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub datasets: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metrics: Vec<String>,
    #[serde(rename = "base_model", skip_serializing_if = "Option::is_none")]
    pub base_model: Option<String>,
    #[serde(rename = "pipeline_tag", skip_serializing_if = "Option::is_none")]
    pub pipeline_tag: Option<String>,
    #[serde(rename = "library_name", skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,
    #[serde(rename = "model-index", skip_serializing_if = "Option::is_none")]
    pub model_index: Option<Vec<ModelIndexEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget: Option<Vec<WidgetConfig>>,
    #[serde(rename = "co2_eq_emissions", skip_serializing_if = "Option::is_none")]
    pub co2_eq_emissions: Option<Co2Emissions>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModelIndexEntry {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub results: Vec<EvaluationResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct EvaluationResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset: Option<DatasetRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metrics: Vec<MetricResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TaskInfo {
    #[serde(rename = "type")]
    pub task_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DatasetRef {
    #[serde(rename = "type")]
    pub dataset_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MetricResult {
    #[serde(rename = "type")]
    pub metric_type: String,
    pub value: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WidgetConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(rename = "example_title", skip_serializing_if = "Option::is_none")]
    pub example_title: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Co2Emissions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emissions: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(rename = "training_type", skip_serializing_if = "Option::is_none")]
    pub training_type: Option<String>,
    #[serde(
        rename = "geographical_location",
        skip_serializing_if = "Option::is_none"
    )]
    pub geographical_location: Option<String>,
    #[serde(rename = "hardware_used", skip_serializing_if = "Option::is_none")]
    pub hardware_used: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TransformersInfo {
    #[serde(rename = "auto_map", skip_serializing_if = "Option::is_none")]
    pub auto_map: Option<HashMap<String, String>>,
    #[serde(rename = "custom_class", skip_serializing_if = "Option::is_none")]
    pub custom_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processor: Option<String>,
}
