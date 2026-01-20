//! Hugging Face Hub API types.
//!
//! This module contains all data types used in the Hugging Face Hub API,
//! including enums for filtering, request/response models, and shared types
//! for models, datasets, spaces, and user operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Core Enums
// =============================================================================

/// Repository type on Hugging Face Hub.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoType {
    /// Machine learning model repository.
    #[default]
    Model,
    /// Dataset repository.
    Dataset,
    /// Spaces application repository.
    Space,
}

/// ML pipeline task type.
///
/// Represents the intended use case of a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Pipeline {
    // Text tasks
    /// Text classification (sentiment analysis, topic classification).
    TextClassification,
    /// Token-level classification (NER, POS tagging).
    TokenClassification,
    /// Question answering from context.
    QuestionAnswering,
    /// Autoregressive text generation.
    TextGeneration,
    /// Sequence-to-sequence text generation.
    Text2TextGeneration,
    /// Text summarization.
    Summarization,
    /// Language translation.
    Translation,
    /// Masked language modeling (fill-in-the-blank).
    FillMask,
    /// Feature/embedding extraction.
    FeatureExtraction,
    /// Conversational AI / chatbots.
    Conversational,
    /// Table-based question answering.
    TableQuestionAnswering,
    /// Sentence similarity scoring.
    SentenceSimilarity,
    /// Zero-shot text classification.
    ZeroShotClassification,

    // Vision tasks
    /// Image classification.
    ImageClassification,
    /// Object detection in images.
    ObjectDetection,
    /// Image segmentation.
    ImageSegmentation,
    /// Image-to-image transformation.
    ImageToImage,
    /// Depth estimation from images.
    DepthEstimation,
    /// Video classification.
    VideoClassification,
    /// Image feature extraction.
    ImageFeatureExtraction,
    /// Unconditional image generation.
    UnconditionalImageGeneration,
    /// Zero-shot image classification.
    ZeroShotImageClassification,
    /// Zero-shot object detection.
    ZeroShotObjectDetection,
    /// Mask generation for images.
    MaskGeneration,
    /// Keypoint detection in images.
    KeypointDetection,

    // Multimodal tasks
    /// Generate images from text descriptions.
    TextToImage,
    /// Generate text descriptions from images.
    ImageToText,
    /// Visual question answering.
    VisualQuestionAnswering,
    /// Document question answering.
    DocumentQuestionAnswering,
    /// Generate video from text.
    TextToVideo,
    /// Generate 3D models from images/text.
    ImageTo3d,
    /// Generate 3D from text.
    TextTo3d,
    /// Any-to-any multimodal generation.
    AnyToAny,

    // Audio tasks
    /// Transcribe speech to text.
    AutomaticSpeechRecognition,
    /// Classify audio clips.
    AudioClassification,
    /// Text-to-speech synthesis.
    TextToSpeech,
    /// Convert audio between formats/speakers.
    AudioToAudio,
    /// Generate music from text.
    TextToAudio,
    /// Voice activity detection.
    VoiceActivityDetection,

    // Tabular tasks
    /// Tabular classification.
    TabularClassification,
    /// Tabular regression.
    TabularRegression,

    // Reinforcement learning
    /// Reinforcement learning agents.
    ReinforcementLearning,
    /// Robotics applications.
    Robotics,

    // Scientific/specialized
    /// Graph machine learning.
    GraphMl,
    /// Time series forecasting.
    TimeSeriesForecasting,

    // Other
    /// Placeholder for unrecognized tasks.
    #[serde(other)]
    Other,
}

/// ML library/framework used by a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Library {
    /// Hugging Face Transformers.
    Transformers,
    /// Hugging Face Diffusers.
    Diffusers,
    /// PyTorch native.
    Pytorch,
    /// TensorFlow native.
    Tensorflow,
    /// JAX/Flax.
    Jax,
    /// spaCy NLP library.
    #[serde(rename = "spacy")]
    SpaCy,
    /// FastAI.
    Fastai,
    /// Keras.
    Keras,
    /// ONNX format.
    #[serde(rename = "onnx")]
    Onnx,
    /// Sentence Transformers.
    #[serde(rename = "sentence-transformers")]
    SentenceTransformers,
    /// Stable Baselines 3.
    #[serde(rename = "stable-baselines3")]
    StableBaselines3,
    /// Scikit-learn.
    Sklearn,
    /// TensorBoard.
    Tensorboard,
    /// Adapter Transformers.
    #[serde(rename = "adapter-transformers")]
    AdapterTransformers,
    /// PEFT (Parameter-Efficient Fine-Tuning).
    Peft,
    /// TensorFlow Lite.
    #[serde(rename = "tflite")]
    TfLite,
    /// OpenVINO.
    Openvino,
    /// Core ML.
    Coreml,
    /// Timm (PyTorch Image Models).
    Timm,
    /// GGUF format (llama.cpp compatible).
    #[serde(rename = "gguf")]
    Gguf,
    /// MLX (Apple Silicon).
    Mlx,
    /// Flair NLP.
    Flair,
    /// AllenNLP.
    Allennlp,
    /// ESPnet.
    Espnet,
    /// Asteroid.
    Asteroid,
    /// SpeechBrain.
    Speechbrain,
    /// Fairseq.
    Fairseq,
    /// Nemo.
    Nemo,
    /// PaddlePaddle.
    Paddlepaddle,
    /// Safetensors format.
    Safetensors,
    /// SetFit.
    Setfit,
    /// SpanMarker.
    Spanmarker,
    /// Keras NLP.
    #[serde(rename = "keras-nlp")]
    KerasNlp,
    /// MLX LM.
    #[serde(rename = "mlx-lm")]
    MlxLm,
    /// Unrecognized library.
    #[serde(other)]
    Other,
}

/// Sort field for listing endpoints.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortField {
    /// Sort by last modification time.
    #[default]
    LastModified,
    /// Sort by number of likes.
    Likes,
    /// Sort by number of downloads.
    Downloads,
    /// Sort by creation date.
    Created,
    /// Sort by trending score.
    Trending,
}

/// Sort direction for list queries.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    /// Ascending order (oldest/lowest first).
    #[serde(rename = "1")]
    Ascending,
    /// Descending order (newest/highest first).
    #[default]
    #[serde(rename = "-1")]
    Descending,
}

/// Repository visibility/privacy setting.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoVisibility {
    /// Publicly visible repository.
    #[default]
    Public,
    /// Private repository (owner/collaborators only).
    Private,
}

/// Gated model access status.
///
/// Controls how access requests are handled.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GatedStatus {
    /// No gating - open access.
    #[default]
    #[serde(rename = "false")]
    False,
    /// Gated with automatic approval.
    #[serde(rename = "true")]
    True,
    /// Automatic access approval.
    Auto,
    /// Manual access approval required.
    Manual,
}

/// File type in a repository.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    /// Regular file.
    #[default]
    File,
    /// Directory/folder.
    Directory,
}

/// Discussion/pull request status.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscussionStatus {
    /// Open for discussion/review.
    #[default]
    Open,
    /// Closed without merging.
    Closed,
    /// Merged (for pull requests).
    Merged,
    /// Draft state.
    Draft,
}

/// Discussion type.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiscussionType {
    /// General discussion thread.
    #[default]
    Discussion,
    /// Pull request with code changes.
    #[serde(rename = "pull_request")]
    PullRequest,
}

/// Space runtime stage.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SpaceStage {
    /// Space is not configured.
    #[default]
    NoAppFile,
    /// Configuration error.
    ConfigError,
    /// Building the space.
    Building,
    /// Build failed.
    BuildError,
    /// Space is running.
    Running,
    /// Runtime error occurred.
    RuntimeError,
    /// Space is paused.
    Paused,
    /// Space is sleeping (inactive).
    Sleeping,
    /// Space has been deleted.
    Deleted,
}

/// Space SDK type.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpaceSdk {
    /// Gradio interface.
    #[default]
    Gradio,
    /// Streamlit app.
    Streamlit,
    /// Docker container.
    Docker,
    /// Static HTML/JS.
    Static,
}

/// Space hardware tier.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpaceHardware {
    /// Free CPU tier.
    #[default]
    #[serde(rename = "cpu-basic")]
    CpuBasic,
    /// Upgraded CPU tier.
    #[serde(rename = "cpu-upgrade")]
    CpuUpgrade,
    /// NVIDIA T4 small GPU.
    #[serde(rename = "t4-small")]
    T4Small,
    /// NVIDIA T4 medium GPU.
    #[serde(rename = "t4-medium")]
    T4Medium,
    /// NVIDIA A10G small GPU.
    #[serde(rename = "a10g-small")]
    A10gSmall,
    /// NVIDIA A10G large GPU.
    #[serde(rename = "a10g-large")]
    A10gLarge,
    /// NVIDIA A10G large x2 GPU.
    #[serde(rename = "a10g-largex2")]
    A10gLargeX2,
    /// NVIDIA A10G large x4 GPU.
    #[serde(rename = "a10g-largex4")]
    A10gLargeX4,
    /// NVIDIA A100 large GPU.
    #[serde(rename = "a100-large")]
    A100Large,
    /// Zero GPU (pay per use).
    #[serde(rename = "zero-a10g")]
    ZeroA10g,
    /// TPU v5 (1x1 pod slice).
    #[serde(rename = "v5e-1x1")]
    V5e1x1,
    /// TPU v5 (2x2 pod slice).
    #[serde(rename = "v5e-2x2")]
    V5e2x2,
    /// TPU v5 (2x4 pod slice).
    #[serde(rename = "v5e-2x4")]
    V5e2x4,
    /// Custom/unrecognized hardware.
    #[serde(other)]
    Other,
}

/// Inference API status for a model.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InferenceStatus {
    /// Inference is available.
    #[serde(rename = "warm")]
    Warm,
    /// Model needs to be loaded.
    #[serde(rename = "cold")]
    Cold,
    /// Inference not available.
    #[default]
    #[serde(rename = "off")]
    Off,
    /// Loading/warming up.
    #[serde(rename = "loading")]
    Loading,
}

/// Inference provider type.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InferenceProvider {
    /// Hugging Face hosted inference.
    #[default]
    HfInference,
    /// Serverless inference (pay per use).
    Serverless,
    /// Dedicated inference endpoints.
    DedicatedEndpoint,
    /// Third-party provider.
    ThirdParty,
}

// =============================================================================
// Common Structs
// =============================================================================

/// Repository file/sibling entry.
///
/// Represents a single file within a repository.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepoFile {
    /// Relative filename within the repository.
    pub rfilename: String,

    /// File size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,

    /// Blob ID (Git SHA).
    #[serde(rename = "blobId", skip_serializing_if = "Option::is_none")]
    pub blob_id: Option<String>,

    /// LFS pointer information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lfs: Option<LfsInfo>,
}

/// LFS (Large File Storage) information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LfsInfo {
    /// SHA256 hash of the file.
    pub sha256: String,

    /// File size in bytes.
    pub size: u64,

    /// Pointer size.
    #[serde(rename = "pointerSize", skip_serializing_if = "Option::is_none")]
    pub pointer_size: Option<u64>,
}

/// Git commit information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitInfo {
    /// Commit SHA hash.
    pub id: String,

    /// Commit title/message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Commit message body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Commit date (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// Author/user information for commits and discussions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Author {
    /// Username or full name.
    pub name: String,

    /// Email address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// User avatar URL.
    #[serde(rename = "avatarUrl", skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

/// Tag/label attached to a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    /// Tag identifier.
    pub id: String,

    /// Tag label/display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Safetensors metadata for a model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetensorsInfo {
    /// Total model size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,

    /// Parameter count by dtype.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, u64>>,

    /// Sharding information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharded: Option<bool>,
}

/// GGUF quantization file information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GgufInfo {
    /// Quantization type (e.g., "Q4_K_M").
    #[serde(rename = "quantization", skip_serializing_if = "Option::is_none")]
    pub quantization: Option<String>,

    /// File size in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,

    /// Bits per weight.
    #[serde(rename = "bitsPerWeight", skip_serializing_if = "Option::is_none")]
    pub bits_per_weight: Option<f64>,
}

/// Model card metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardData {
    /// License identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// License name/description.
    #[serde(rename = "license_name", skip_serializing_if = "Option::is_none")]
    pub license_name: Option<String>,

    /// License URL.
    #[serde(rename = "license_link", skip_serializing_if = "Option::is_none")]
    pub license_link: Option<String>,

    /// Supported languages (ISO codes).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub language: Vec<String>,

    /// Tags/keywords.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Datasets used for training.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub datasets: Vec<String>,

    /// Evaluation metrics.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metrics: Vec<String>,

    /// Base model this was fine-tuned from.
    #[serde(rename = "base_model", skip_serializing_if = "Option::is_none")]
    pub base_model: Option<String>,

    /// Pipeline task tag.
    #[serde(rename = "pipeline_tag", skip_serializing_if = "Option::is_none")]
    pub pipeline_tag: Option<String>,

    /// Library name.
    #[serde(rename = "library_name", skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,

    /// Model index entries.
    #[serde(rename = "model-index", skip_serializing_if = "Option::is_none")]
    pub model_index: Option<Vec<ModelIndexEntry>>,

    /// Custom widget configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget: Option<Vec<WidgetConfig>>,

    /// Co2 emissions data.
    #[serde(rename = "co2_eq_emissions", skip_serializing_if = "Option::is_none")]
    pub co2_eq_emissions: Option<Co2Emissions>,

    /// Arbitrary extra metadata.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Model index entry for evaluation results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelIndexEntry {
    /// Model name.
    pub name: String,

    /// Evaluation results.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub results: Vec<EvaluationResult>,
}

/// Evaluation result on a benchmark.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationResult {
    /// Task type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<TaskInfo>,

    /// Dataset used for evaluation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset: Option<DatasetRef>,

    /// Metrics and scores.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub metrics: Vec<MetricResult>,
}

/// Task information for evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskInfo {
    /// Task type name.
    #[serde(rename = "type")]
    pub task_type: String,

    /// Task display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Dataset reference for evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetRef {
    /// Dataset type/name.
    #[serde(rename = "type")]
    pub dataset_type: String,

    /// Dataset display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Dataset configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<String>,

    /// Dataset split.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split: Option<String>,
}

/// Metric result from evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricResult {
    /// Metric type/name.
    #[serde(rename = "type")]
    pub metric_type: String,

    /// Metric value/score.
    pub value: serde_json::Value,

    /// Display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Whether this is verified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified: Option<bool>,
}

/// Widget configuration for model demos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetConfig {
    /// Example text/input.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Example source URL (for images/audio).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,

    /// Example label/title.
    #[serde(rename = "example_title", skip_serializing_if = "Option::is_none")]
    pub example_title: Option<String>,

    /// Additional widget parameters.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// CO2 emissions information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Co2Emissions {
    /// Emissions in grams.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emissions: Option<f64>,

    /// Source of emissions data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Training type (e.g., "fine-tuning").
    #[serde(rename = "training_type", skip_serializing_if = "Option::is_none")]
    pub training_type: Option<String>,

    /// Geographical location of training.
    #[serde(rename = "geographical_location", skip_serializing_if = "Option::is_none")]
    pub geographical_location: Option<String>,

    /// Hardware used for training.
    #[serde(rename = "hardware_used", skip_serializing_if = "Option::is_none")]
    pub hardware_used: Option<String>,
}

/// Transform/processor configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformersInfo {
    /// Auto class mappings.
    #[serde(rename = "auto_map", skip_serializing_if = "Option::is_none")]
    pub auto_map: Option<HashMap<String, String>>,

    /// Custom pipeline class.
    #[serde(rename = "custom_class", skip_serializing_if = "Option::is_none")]
    pub custom_class: Option<String>,

    /// Processor class.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processor: Option<String>,
}

// =============================================================================
// Model Types
// =============================================================================

/// Complete model information from the Hub API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model identifier (e.g., "bert-base-uncased" or "org/model").
    #[serde(rename = "modelId", alias = "id")]
    pub model_id: String,

    /// SHA of the current commit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,

    /// Author/organization name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Last modified timestamp (ISO 8601).
    #[serde(rename = "lastModified", skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,

    /// Creation timestamp (ISO 8601).
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Whether the repository is private.
    #[serde(default)]
    pub private: bool,

    /// Whether the repository is disabled.
    #[serde(default)]
    pub disabled: bool,

    /// Gated access status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gated: Option<GatedStatus>,

    /// Total download count.
    #[serde(default)]
    pub downloads: u64,

    /// Downloads in the last month.
    #[serde(rename = "downloadsAllTime", skip_serializing_if = "Option::is_none")]
    pub downloads_all_time: Option<u64>,

    /// Number of likes.
    #[serde(default)]
    pub likes: u64,

    /// Pipeline task tag.
    #[serde(rename = "pipeline_tag", skip_serializing_if = "Option::is_none")]
    pub pipeline_tag: Option<String>,

    /// Library/framework name.
    #[serde(rename = "library_name", skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,

    /// All tags attached to the model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Files in the repository.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub siblings: Vec<RepoFile>,

    /// Spaces using this model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spaces: Vec<String>,

    /// Model card data (parsed YAML frontmatter).
    #[serde(rename = "cardData", skip_serializing_if = "Option::is_none")]
    pub card_data: Option<CardData>,

    /// Safetensors metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safetensors: Option<SafetensorsInfo>,

    /// Transformers configuration.
    #[serde(rename = "transformersInfo", skip_serializing_if = "Option::is_none")]
    pub transformers_info: Option<TransformersInfo>,

    /// config.json contents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HashMap<String, serde_json::Value>>,

    /// Trending score.
    #[serde(rename = "trendingScore", skip_serializing_if = "Option::is_none")]
    pub trending_score: Option<f64>,

    /// Inference API status.
    #[serde(rename = "inference", skip_serializing_if = "Option::is_none")]
    pub inference: Option<InferenceStatus>,

    /// Mask token for masked LM models.
    #[serde(rename = "mask_token", skip_serializing_if = "Option::is_none")]
    pub mask_token: Option<String>,

    /// Widget data for the model.
    #[serde(rename = "widgetData", skip_serializing_if = "Option::is_none")]
    pub widget_data: Option<Vec<WidgetConfig>>,

    /// Model index with evaluation results.
    #[serde(rename = "model-index", skip_serializing_if = "Option::is_none")]
    pub model_index: Option<Vec<ModelIndexEntry>>,
}

/// Summary model info for list endpoints (fewer fields).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelSummary {
    /// Model identifier.
    #[serde(rename = "modelId", alias = "_id", alias = "id")]
    pub model_id: String,

    /// Author/organization name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Last modified timestamp.
    #[serde(rename = "lastModified", skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,

    /// Download count.
    #[serde(default)]
    pub downloads: u64,

    /// Like count.
    #[serde(default)]
    pub likes: u64,

    /// Pipeline task.
    #[serde(rename = "pipeline_tag", skip_serializing_if = "Option::is_none")]
    pub pipeline_tag: Option<String>,

    /// Library name.
    #[serde(rename = "library_name", skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,

    /// Tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Whether private.
    #[serde(default)]
    pub private: bool,
}

// =============================================================================
// Dataset Types
// =============================================================================

/// Complete dataset information from the Hub API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetInfo {
    /// Dataset identifier (e.g., "squad" or "org/dataset").
    #[serde(rename = "id", alias = "datasetId")]
    pub id: String,

    /// SHA of the current commit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,

    /// Author/organization name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Last modified timestamp.
    #[serde(rename = "lastModified", skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,

    /// Creation timestamp.
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Whether private.
    #[serde(default)]
    pub private: bool,

    /// Whether disabled.
    #[serde(default)]
    pub disabled: bool,

    /// Gated access status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gated: Option<GatedStatus>,

    /// Download count.
    #[serde(default)]
    pub downloads: u64,

    /// Like count.
    #[serde(default)]
    pub likes: u64,

    /// All tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Files in the repository.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub siblings: Vec<RepoFile>,

    /// Card data (parsed frontmatter).
    #[serde(rename = "cardData", skip_serializing_if = "Option::is_none")]
    pub card_data: Option<DatasetCardData>,

    /// Description text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Citation text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation: Option<String>,

    /// Trending score.
    #[serde(rename = "trendingScore", skip_serializing_if = "Option::is_none")]
    pub trending_score: Option<f64>,
}

/// Dataset-specific card data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetCardData {
    /// License identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Supported languages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub language: Vec<String>,

    /// Tags/keywords.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Task categories.
    #[serde(rename = "task_categories", default, skip_serializing_if = "Vec::is_empty")]
    pub task_categories: Vec<String>,

    /// Task IDs.
    #[serde(rename = "task_ids", default, skip_serializing_if = "Vec::is_empty")]
    pub task_ids: Vec<String>,

    /// Size category.
    #[serde(rename = "size_categories", default, skip_serializing_if = "Vec::is_empty")]
    pub size_categories: Vec<String>,

    /// Pretty name.
    #[serde(rename = "pretty_name", skip_serializing_if = "Option::is_none")]
    pub pretty_name: Option<String>,

    /// Dataset features/configs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub configs: Vec<DatasetConfig>,

    /// Arbitrary extra metadata.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Dataset configuration/split information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetConfig {
    /// Configuration name.
    #[serde(rename = "config_name", skip_serializing_if = "Option::is_none")]
    pub config_name: Option<String>,

    /// Data files patterns.
    #[serde(rename = "data_files", skip_serializing_if = "Option::is_none")]
    pub data_files: Option<serde_json::Value>,

    /// Default split.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub splits: Vec<DatasetSplit>,
}

/// Dataset split information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetSplit {
    /// Split name (train, test, validation).
    pub name: String,

    /// Number of examples.
    #[serde(rename = "num_examples", skip_serializing_if = "Option::is_none")]
    pub num_examples: Option<u64>,

    /// Number of bytes.
    #[serde(rename = "num_bytes", skip_serializing_if = "Option::is_none")]
    pub num_bytes: Option<u64>,
}

/// Summary dataset info for list endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatasetSummary {
    /// Dataset identifier.
    #[serde(rename = "id", alias = "_id")]
    pub id: String,

    /// Author/organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Last modified timestamp.
    #[serde(rename = "lastModified", skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,

    /// Download count.
    #[serde(default)]
    pub downloads: u64,

    /// Like count.
    #[serde(default)]
    pub likes: u64,

    /// Tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Whether private.
    #[serde(default)]
    pub private: bool,
}

// =============================================================================
// Space Types
// =============================================================================

/// Complete Space information from the Hub API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpaceInfo {
    /// Space identifier (e.g., "org/space-name").
    #[serde(rename = "id", alias = "spaceId")]
    pub id: String,

    /// SHA of the current commit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,

    /// Author/organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Last modified timestamp.
    #[serde(rename = "lastModified", skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,

    /// Creation timestamp.
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Whether private.
    #[serde(default)]
    pub private: bool,

    /// Whether disabled.
    #[serde(default)]
    pub disabled: bool,

    /// Like count.
    #[serde(default)]
    pub likes: u64,

    /// All tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// SDK type (gradio, streamlit, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk: Option<SpaceSdk>,

    /// SDK version.
    #[serde(rename = "sdk_version", skip_serializing_if = "Option::is_none")]
    pub sdk_version: Option<String>,

    /// Runtime information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<SpaceRuntime>,

    /// Models used by this space.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,

    /// Datasets used by this space.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub datasets: Vec<String>,

    /// Card data (parsed frontmatter).
    #[serde(rename = "cardData", skip_serializing_if = "Option::is_none")]
    pub card_data: Option<SpaceCardData>,

    /// Files in the repository.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub siblings: Vec<RepoFile>,

    /// Emoji icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,

    /// Color theme.
    #[serde(rename = "colorFrom", skip_serializing_if = "Option::is_none")]
    pub color_from: Option<String>,

    /// Color theme (end).
    #[serde(rename = "colorTo", skip_serializing_if = "Option::is_none")]
    pub color_to: Option<String>,

    /// Pinned status.
    #[serde(default)]
    pub pinned: bool,

    /// Trending score.
    #[serde(rename = "trendingScore", skip_serializing_if = "Option::is_none")]
    pub trending_score: Option<f64>,
}

/// Space runtime status and configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpaceRuntime {
    /// Current stage/status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<SpaceStage>,

    /// Hardware tier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware: Option<SpaceHardware>,

    /// Requested hardware.
    #[serde(rename = "requestedHardware", skip_serializing_if = "Option::is_none")]
    pub requested_hardware: Option<SpaceHardware>,

    /// Sleep time in seconds (0 = never).
    #[serde(rename = "gcTimeout", skip_serializing_if = "Option::is_none")]
    pub gc_timeout: Option<u64>,

    /// Error message if any.
    #[serde(rename = "errorMessage", skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,

    /// Storage mount path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,

    /// Number of replicas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicas: Option<u32>,
}

/// Space-specific card data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpaceCardData {
    /// Title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Emoji icon.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,

    /// Color theme start.
    #[serde(rename = "colorFrom", skip_serializing_if = "Option::is_none")]
    pub color_from: Option<String>,

    /// Color theme end.
    #[serde(rename = "colorTo", skip_serializing_if = "Option::is_none")]
    pub color_to: Option<String>,

    /// SDK type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk: Option<SpaceSdk>,

    /// SDK version.
    #[serde(rename = "sdk_version", skip_serializing_if = "Option::is_none")]
    pub sdk_version: Option<String>,

    /// Application file path.
    #[serde(rename = "app_file", skip_serializing_if = "Option::is_none")]
    pub app_file: Option<String>,

    /// Application port.
    #[serde(rename = "app_port", skip_serializing_if = "Option::is_none")]
    pub app_port: Option<u16>,

    /// Pinned spaces.
    #[serde(rename = "pinned", default)]
    pub pinned: bool,

    /// License.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Suggested hardware.
    #[serde(rename = "suggested_hardware", skip_serializing_if = "Option::is_none")]
    pub suggested_hardware: Option<SpaceHardware>,

    /// Suggested storage.
    #[serde(rename = "suggested_storage", skip_serializing_if = "Option::is_none")]
    pub suggested_storage: Option<String>,

    /// Models used.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,

    /// Datasets used.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub datasets: Vec<String>,

    /// Tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Short description.
    #[serde(rename = "short_description", skip_serializing_if = "Option::is_none")]
    pub short_description: Option<String>,

    /// Arbitrary extra metadata.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Summary Space info for list endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpaceSummary {
    /// Space identifier.
    #[serde(rename = "id", alias = "_id")]
    pub id: String,

    /// Author/organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Last modified timestamp.
    #[serde(rename = "lastModified", skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,

    /// Like count.
    #[serde(default)]
    pub likes: u64,

    /// SDK type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk: Option<SpaceSdk>,

    /// Tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Whether private.
    #[serde(default)]
    pub private: bool,
}

// =============================================================================
// User & Organization Types
// =============================================================================

/// User information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInfo {
    /// Username.
    #[serde(rename = "name", alias = "user")]
    pub name: String,

    /// Full display name.
    #[serde(rename = "fullname", skip_serializing_if = "Option::is_none")]
    pub fullname: Option<String>,

    /// User type (user or org).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub user_type: Option<String>,

    /// Avatar URL.
    #[serde(rename = "avatarUrl", skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,

    /// Whether this is a pro user.
    #[serde(rename = "isPro", default)]
    pub is_pro: bool,

    /// Number of models.
    #[serde(rename = "numModels", skip_serializing_if = "Option::is_none")]
    pub num_models: Option<u64>,

    /// Number of datasets.
    #[serde(rename = "numDatasets", skip_serializing_if = "Option::is_none")]
    pub num_datasets: Option<u64>,

    /// Number of spaces.
    #[serde(rename = "numSpaces", skip_serializing_if = "Option::is_none")]
    pub num_spaces: Option<u64>,

    /// Number of likes given.
    #[serde(rename = "numLikes", skip_serializing_if = "Option::is_none")]
    pub num_likes: Option<u64>,

    /// Number of followers.
    #[serde(rename = "numFollowers", skip_serializing_if = "Option::is_none")]
    pub num_followers: Option<u64>,

    /// Number following.
    #[serde(rename = "numFollowing", skip_serializing_if = "Option::is_none")]
    pub num_following: Option<u64>,
}

/// Response from the whoami endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhoAmIResponse {
    /// Account type ("user" or "org").
    #[serde(rename = "type")]
    pub account_type: String,

    /// Username.
    pub name: String,

    /// Full display name.
    #[serde(rename = "fullname", skip_serializing_if = "Option::is_none")]
    pub fullname: Option<String>,

    /// Email address (if permitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// Whether email is verified.
    #[serde(rename = "emailVerified", default)]
    pub email_verified: bool,

    /// Whether user can pay.
    #[serde(rename = "canPay", default)]
    pub can_pay: bool,

    /// Whether this is a pro user.
    #[serde(rename = "isPro", default)]
    pub is_pro: bool,

    /// Periodic account data (models, datasets, etc.).
    #[serde(rename = "periodicalAccountData", skip_serializing_if = "Option::is_none")]
    pub periodical_account_data: Option<PeriodicalAccountData>,

    /// Avatar URL.
    #[serde(rename = "avatarUrl", skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,

    /// Organizations the user belongs to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub orgs: Vec<OrganizationRef>,

    /// Token permissions/scopes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthInfo>,
}

/// Periodical account usage data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeriodicalAccountData {
    /// Number of models.
    #[serde(rename = "numModels", skip_serializing_if = "Option::is_none")]
    pub num_models: Option<u64>,

    /// Number of datasets.
    #[serde(rename = "numDatasets", skip_serializing_if = "Option::is_none")]
    pub num_datasets: Option<u64>,

    /// Number of spaces.
    #[serde(rename = "numSpaces", skip_serializing_if = "Option::is_none")]
    pub num_spaces: Option<u64>,

    /// Number of discussions.
    #[serde(rename = "numDiscussions", skip_serializing_if = "Option::is_none")]
    pub num_discussions: Option<u64>,

    /// Number of papers.
    #[serde(rename = "numPapers", skip_serializing_if = "Option::is_none")]
    pub num_papers: Option<u64>,

    /// Number of upvotes received.
    #[serde(rename = "numUpvotes", skip_serializing_if = "Option::is_none")]
    pub num_upvotes: Option<u64>,
}

/// Organization reference (minimal info).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationRef {
    /// Organization name/slug.
    pub name: String,

    /// Full display name.
    #[serde(rename = "fullname", skip_serializing_if = "Option::is_none")]
    pub fullname: Option<String>,

    /// Avatar URL.
    #[serde(rename = "avatarUrl", skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,

    /// Whether this is a pro org.
    #[serde(rename = "isPro", default)]
    pub is_pro: bool,

    /// User's role in the org.
    #[serde(rename = "roleInOrg", skip_serializing_if = "Option::is_none")]
    pub role_in_org: Option<String>,

    /// Whether user is admin.
    #[serde(rename = "isEnterprise", default)]
    pub is_enterprise: bool,
}

/// Organization information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organization {
    /// Organization name/slug.
    pub name: String,

    /// Full display name.
    #[serde(rename = "fullname", skip_serializing_if = "Option::is_none")]
    pub fullname: Option<String>,

    /// Avatar URL.
    #[serde(rename = "avatarUrl", skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,

    /// Whether this is a pro org.
    #[serde(rename = "isPro", default)]
    pub is_pro: bool,

    /// Whether this is an enterprise org.
    #[serde(rename = "isEnterprise", default)]
    pub is_enterprise: bool,

    /// Number of members.
    #[serde(rename = "numMembers", skip_serializing_if = "Option::is_none")]
    pub num_members: Option<u64>,

    /// Number of models.
    #[serde(rename = "numModels", skip_serializing_if = "Option::is_none")]
    pub num_models: Option<u64>,

    /// Number of datasets.
    #[serde(rename = "numDatasets", skip_serializing_if = "Option::is_none")]
    pub num_datasets: Option<u64>,

    /// Number of spaces.
    #[serde(rename = "numSpaces", skip_serializing_if = "Option::is_none")]
    pub num_spaces: Option<u64>,
}

/// Token authentication/permission info.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthInfo {
    /// Token type.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub auth_type: Option<String>,

    /// Access token (partially hidden).
    #[serde(rename = "accessToken", skip_serializing_if = "Option::is_none")]
    pub access_token: Option<TokenInfo>,
}

/// Token information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Token display name.
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// Token role/permission level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,

    /// Creation timestamp.
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}

// =============================================================================
// Discussion Types
// =============================================================================

/// Discussion thread on a repository.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Discussion {
    /// Discussion number.
    pub num: u64,

    /// Discussion title.
    pub title: String,

    /// Discussion status.
    pub status: DiscussionStatus,

    /// Discussion type (discussion or pull_request).
    #[serde(rename = "isPullRequest", default)]
    pub is_pull_request: bool,

    /// Author information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Author>,

    /// Creation timestamp.
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Number of comments.
    #[serde(rename = "numComments", skip_serializing_if = "Option::is_none")]
    pub num_comments: Option<u64>,

    /// Whether pinned.
    #[serde(default)]
    pub pinned: bool,

    /// Labels/tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
}

/// Discussion comment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscussionComment {
    /// Comment ID.
    pub id: String,

    /// Author information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Author>,

    /// Comment content (markdown).
    pub content: String,

    /// Creation timestamp.
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Last edit timestamp.
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,

    /// Whether this is hidden.
    #[serde(default)]
    pub hidden: bool,
}

/// List of discussions response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiscussionList {
    /// List of discussions.
    pub discussions: Vec<Discussion>,

    /// Total count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
}

// =============================================================================
// Filter Types (for query construction)
// =============================================================================

/// Filter parameters for model search.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelFilter {
    /// Filter by author/organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Filter by library/framework.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub library: Option<String>,

    /// Filter by language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Filter by pipeline task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,

    /// Filter by tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Search query string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,

    /// Filter by trained dataset.
    #[serde(rename = "trained_dataset", skip_serializing_if = "Option::is_none")]
    pub trained_dataset: Option<String>,

    /// Full text search in model card.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full: Option<String>,
}

/// Filter parameters for dataset search.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetFilter {
    /// Filter by author/organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Filter by language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    /// Filter by task category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,

    /// Filter by size category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,

    /// Filter by tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Search query string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

/// Filter parameters for space search.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceFilter {
    /// Filter by author/organization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Filter by SDK.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk: Option<SpaceSdk>,

    /// Filter by tags.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Filter by models used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<String>,

    /// Filter by datasets used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datasets: Option<String>,

    /// Search query string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
}

// =============================================================================
// Request/Response Types
// =============================================================================

/// Request to create a new repository.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateRepoBody {
    /// Repository name (without org prefix).
    pub name: String,

    /// Repository type.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub repo_type: Option<RepoType>,

    /// Organization to create in (optional, uses user namespace otherwise).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,

    /// Whether the repo should be private.
    #[serde(default)]
    pub private: bool,

    /// Whether to create an SDK-specific repo (for Spaces).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk: Option<SpaceSdk>,

    /// Hardware tier for Spaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware: Option<SpaceHardware>,

    /// Storage tier for Spaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,

    /// Sleep timeout in seconds (Spaces).
    #[serde(rename = "sleepTimeSeconds", skip_serializing_if = "Option::is_none")]
    pub sleep_time_seconds: Option<u64>,

    /// Secrets for Spaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Vec<SpaceSecret>>,

    /// Environment variables for Spaces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<SpaceVariable>>,
}

/// Space secret configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceSecret {
    /// Secret key name.
    pub key: String,

    /// Secret value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,

    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Space environment variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceVariable {
    /// Variable key name.
    pub key: String,

    /// Variable value.
    pub value: String,

    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Response from create repository endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateRepoResponse {
    /// Full repository URL.
    pub url: String,

    /// Repository ID (e.g., "org/repo").
    #[serde(rename = "repoId", skip_serializing_if = "Option::is_none")]
    pub repo_id: Option<String>,
}

/// Request to delete a repository.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteRepoBody {
    /// Repository ID to delete.
    #[serde(rename = "repoId")]
    pub repo_id: String,

    /// Repository type.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub repo_type: Option<RepoType>,

    /// Organization (if deleting org repo).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
}

/// Request to move/rename a repository.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveRepoBody {
    /// Source repository ID.
    #[serde(rename = "fromRepo")]
    pub from_repo: String,

    /// Destination repository ID.
    #[serde(rename = "toRepo")]
    pub to_repo: String,

    /// Repository type.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub repo_type: Option<RepoType>,
}

/// Request to update repository settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateRepoSettingsBody {
    /// Change gated status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gated: Option<GatedStatus>,

    /// Change private status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,
}

/// Request to update Space settings.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateSpaceSettingsRequest {
    /// Change hardware tier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware: Option<SpaceHardware>,

    /// Change storage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,

    /// Change sleep timeout.
    #[serde(rename = "sleepTimeSeconds", skip_serializing_if = "Option::is_none")]
    pub sleep_time_seconds: Option<u64>,

    /// Add or update secrets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Vec<SpaceSecret>>,

    /// Add or update variables.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<SpaceVariable>>,
}

/// Request to upload a file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadFileRequest {
    /// Path within the repository.
    pub path: String,

    /// Commit message.
    #[serde(rename = "commitMessage", skip_serializing_if = "Option::is_none")]
    pub commit_message: Option<String>,

    /// Commit description.
    #[serde(rename = "commitDescription", skip_serializing_if = "Option::is_none")]
    pub commit_description: Option<String>,

    /// Branch to commit to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Create branch if missing.
    #[serde(rename = "createBranch", default)]
    pub create_branch: bool,
}

/// Response from file upload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadFileResponse {
    /// Commit info.
    #[serde(rename = "commitOid", skip_serializing_if = "Option::is_none")]
    pub commit_oid: Option<String>,

    /// Commit URL.
    #[serde(rename = "commitUrl", skip_serializing_if = "Option::is_none")]
    pub commit_url: Option<String>,
}

/// Inference API request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InferenceRequest {
    /// Input text/data.
    pub inputs: serde_json::Value,

    /// Model-specific parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<InferenceParameters>,

    /// Options for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<InferenceOptions>,
}

/// Parameters for inference.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InferenceParameters {
    /// Maximum new tokens to generate.
    #[serde(rename = "max_new_tokens", skip_serializing_if = "Option::is_none")]
    pub max_new_tokens: Option<u32>,

    /// Sampling temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Top-p nucleus sampling.
    #[serde(rename = "top_p", skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Top-k sampling.
    #[serde(rename = "top_k", skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    /// Repetition penalty.
    #[serde(rename = "repetition_penalty", skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f64>,

    /// Whether to sample.
    #[serde(rename = "do_sample", skip_serializing_if = "Option::is_none")]
    pub do_sample: Option<bool>,

    /// Return full text including prompt.
    #[serde(rename = "return_full_text", skip_serializing_if = "Option::is_none")]
    pub return_full_text: Option<bool>,

    /// Random seed for reproducibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,

    /// Stop sequences.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,

    /// Candidate labels (for zero-shot classification).
    #[serde(rename = "candidate_labels", skip_serializing_if = "Option::is_none")]
    pub candidate_labels: Option<Vec<String>>,

    /// Additional model-specific parameters.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Options for inference request.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InferenceOptions {
    /// Wait for model to load.
    #[serde(rename = "wait_for_model", default)]
    pub wait_for_model: bool,

    /// Use cached results.
    #[serde(rename = "use_cache", default = "default_true")]
    pub use_cache: bool,
}

fn default_true() -> bool {
    true
}

/// Generic inference response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum InferenceResponse {
    /// Text generation response.
    TextGeneration(Vec<TextGenerationResult>),

    /// Classification response.
    Classification(Vec<ClassificationResult>),

    /// Token classification (NER) response.
    TokenClassification(Vec<TokenClassificationResult>),

    /// Zero-shot classification response.
    ZeroShotClassification(ZeroShotResult),

    /// Question answering response.
    QuestionAnswering(QuestionAnsweringResult),

    /// Feature extraction response.
    FeatureExtraction(Vec<Vec<f64>>),

    /// Raw JSON response.
    Raw(serde_json::Value),
}

/// Text generation result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextGenerationResult {
    /// Generated text.
    #[serde(rename = "generated_text")]
    pub generated_text: String,
}

/// Classification result (sentiment, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// Classification label.
    pub label: String,

    /// Confidence score.
    pub score: f64,
}

/// Token classification result (NER, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenClassificationResult {
    /// Entity type/label.
    #[serde(rename = "entity_group", alias = "entity")]
    pub entity_group: String,

    /// Confidence score.
    pub score: f64,

    /// The word/token.
    pub word: String,

    /// Start character position.
    pub start: u64,

    /// End character position.
    pub end: u64,
}

/// Zero-shot classification result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ZeroShotResult {
    /// Input sequence.
    pub sequence: String,

    /// Labels in order of score.
    pub labels: Vec<String>,

    /// Scores for each label.
    pub scores: Vec<f64>,
}

/// Question answering result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestionAnsweringResult {
    /// Answer text.
    pub answer: String,

    /// Confidence score.
    pub score: f64,

    /// Start character position in context.
    pub start: u64,

    /// End character position in context.
    pub end: u64,
}

/// Error response from the API.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiError {
    /// Error message.
    pub error: String,

    /// Error type/code.
    #[serde(rename = "error_type", skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,

    /// Estimated time for model loading (503 errors).
    #[serde(rename = "estimated_time", skip_serializing_if = "Option::is_none")]
    pub estimated_time: Option<f64>,

    /// Additional error details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

// =============================================================================
// Webhook Types
// =============================================================================

/// Webhook event payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// Event type.
    pub event: WebhookEvent,

    /// Repository information.
    pub repo: WebhookRepo,

    /// Discussion info (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discussion: Option<WebhookDiscussion>,

    /// Comment info (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<WebhookComment>,

    /// Webhook configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook: Option<WebhookConfig>,
}

/// Webhook event types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Event action.
    pub action: String,

    /// Event scope.
    pub scope: String,
}

/// Repository info in webhook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookRepo {
    /// Repository type.
    #[serde(rename = "type")]
    pub repo_type: RepoType,

    /// Repository name.
    pub name: String,

    /// Repository ID (org/name).
    pub id: String,

    /// Whether private.
    pub private: bool,

    /// Headsha.
    #[serde(rename = "headSha", skip_serializing_if = "Option::is_none")]
    pub head_sha: Option<String>,
}

/// Discussion info in webhook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookDiscussion {
    /// Discussion number.
    pub num: u64,

    /// Discussion title.
    pub title: String,

    /// Discussion status.
    pub status: DiscussionStatus,
}

/// Comment info in webhook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookComment {
    /// Comment ID.
    pub id: String,

    /// Author name.
    pub author: String,

    /// Whether hidden.
    pub hidden: bool,

    /// Comment content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Webhook configuration info.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook ID.
    pub id: String,
}

// =============================================================================
// Metrics and Collection Types
// =============================================================================

/// Collection (curated list of repos).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    /// Collection slug/ID.
    pub slug: String,

    /// Collection title.
    pub title: String,

    /// Collection description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Owner/author.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    /// Theme/emoji.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,

    /// Whether private.
    #[serde(default)]
    pub private: bool,

    /// Items in the collection.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<CollectionItem>,

    /// Number of upvotes.
    #[serde(default)]
    pub upvotes: u64,

    /// Creation timestamp.
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,

    /// Last updated timestamp.
    #[serde(rename = "lastUpdated", skip_serializing_if = "Option::is_none")]
    pub last_updated: Option<String>,

    /// Position/order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u64>,
}

/// Item in a collection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CollectionItem {
    /// Item type.
    #[serde(rename = "type")]
    pub item_type: RepoType,

    /// Item ID (repo ID).
    pub id: String,

    /// Display title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Note/description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,

    /// Position in collection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u64>,
}

/// Daily download metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DownloadMetrics {
    /// Date string (YYYY-MM-DD).
    pub date: String,

    /// Download count for that day.
    pub downloads: u64,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_type_serialization() {
        assert_eq!(serde_json::to_string(&RepoType::Model).unwrap(), "\"model\"");
        assert_eq!(
            serde_json::to_string(&RepoType::Dataset).unwrap(),
            "\"dataset\""
        );
        assert_eq!(serde_json::to_string(&RepoType::Space).unwrap(), "\"space\"");
    }

    #[test]
    fn repo_type_deserialization() {
        let model: RepoType = serde_json::from_str("\"model\"").unwrap();
        assert_eq!(model, RepoType::Model);

        let dataset: RepoType = serde_json::from_str("\"dataset\"").unwrap();
        assert_eq!(dataset, RepoType::Dataset);
    }

    #[test]
    fn pipeline_serialization() {
        assert_eq!(
            serde_json::to_string(&Pipeline::TextGeneration).unwrap(),
            "\"text-generation\""
        );
        assert_eq!(
            serde_json::to_string(&Pipeline::ImageClassification).unwrap(),
            "\"image-classification\""
        );
    }

    #[test]
    fn library_serialization() {
        assert_eq!(
            serde_json::to_string(&Library::Transformers).unwrap(),
            "\"transformers\""
        );
        assert_eq!(
            serde_json::to_string(&Library::SentenceTransformers).unwrap(),
            "\"sentence-transformers\""
        );
        assert_eq!(serde_json::to_string(&Library::Gguf).unwrap(), "\"gguf\"");
    }

    #[test]
    fn sort_direction_serialization() {
        assert_eq!(
            serde_json::to_string(&SortDirection::Ascending).unwrap(),
            "\"1\""
        );
        assert_eq!(
            serde_json::to_string(&SortDirection::Descending).unwrap(),
            "\"-1\""
        );
    }

    #[test]
    fn gated_status_serialization() {
        assert_eq!(
            serde_json::to_string(&GatedStatus::False).unwrap(),
            "\"false\""
        );
        assert_eq!(
            serde_json::to_string(&GatedStatus::Manual).unwrap(),
            "\"manual\""
        );
    }

    #[test]
    fn space_hardware_serialization() {
        assert_eq!(
            serde_json::to_string(&SpaceHardware::CpuBasic).unwrap(),
            "\"cpu-basic\""
        );
        assert_eq!(
            serde_json::to_string(&SpaceHardware::A10gSmall).unwrap(),
            "\"a10g-small\""
        );
    }

    #[test]
    fn model_info_deserialization() {
        let json = r#"{
            "modelId": "bert-base-uncased",
            "sha": "abc123",
            "author": "google",
            "downloads": 1000000,
            "likes": 5000,
            "pipeline_tag": "fill-mask",
            "library_name": "transformers",
            "tags": ["pytorch", "bert"],
            "private": false
        }"#;

        let model: ModelInfo = serde_json::from_str(json).unwrap();
        assert_eq!(model.model_id, "bert-base-uncased");
        assert_eq!(model.author, Some("google".to_string()));
        assert_eq!(model.downloads, 1000000);
        assert_eq!(model.tags, vec!["pytorch", "bert"]);
    }

    #[test]
    fn model_info_with_id_alias() {
        let json = r#"{
            "id": "org/model-name",
            "downloads": 500
        }"#;

        let model: ModelInfo = serde_json::from_str(json).unwrap();
        assert_eq!(model.model_id, "org/model-name");
    }

    #[test]
    fn repo_file_deserialization() {
        let json = r#"{
            "rfilename": "model.safetensors",
            "size": 1073741824,
            "blobId": "abc123"
        }"#;

        let file: RepoFile = serde_json::from_str(json).unwrap();
        assert_eq!(file.rfilename, "model.safetensors");
        assert_eq!(file.size, Some(1073741824));
    }

    #[test]
    fn safetensors_info_deserialization() {
        let json = r#"{
            "total": 2147483648,
            "parameters": {
                "F16": 1073741824,
                "BF16": 536870912
            },
            "sharded": true
        }"#;

        let info: SafetensorsInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.total, Some(2147483648));
        assert_eq!(info.sharded, Some(true));
    }

    #[test]
    fn inference_parameters_serialization() {
        let params = InferenceParameters {
            max_new_tokens: Some(100),
            temperature: Some(0.7),
            top_p: Some(0.95),
            do_sample: Some(true),
            ..Default::default()
        };

        let json = serde_json::to_string(&params).unwrap();
        assert!(json.contains("\"max_new_tokens\":100"));
        assert!(json.contains("\"temperature\":0.7"));
    }

    #[test]
    fn classification_result_deserialization() {
        let json = r#"{"label": "POSITIVE", "score": 0.9998}"#;

        let result: ClassificationResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.label, "POSITIVE");
        assert!(result.score > 0.99);
    }

    #[test]
    fn who_am_i_deserialization() {
        let json = r#"{
            "type": "user",
            "name": "testuser",
            "fullname": "Test User",
            "emailVerified": true,
            "canPay": false,
            "isPro": true,
            "orgs": [{"name": "test-org", "isPro": false}]
        }"#;

        let response: WhoAmIResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.account_type, "user");
        assert_eq!(response.name, "testuser");
        assert!(response.is_pro);
        assert_eq!(response.orgs.len(), 1);
    }

    #[test]
    fn space_runtime_deserialization() {
        let json = r#"{
            "stage": "RUNNING",
            "hardware": "cpu-basic",
            "gcTimeout": 3600
        }"#;

        let runtime: SpaceRuntime = serde_json::from_str(json).unwrap();
        assert_eq!(runtime.stage, Some(SpaceStage::Running));
        assert_eq!(runtime.hardware, Some(SpaceHardware::CpuBasic));
        assert_eq!(runtime.gc_timeout, Some(3600));
    }

    #[test]
    fn api_error_deserialization() {
        let json = r#"{
            "error": "Model too busy",
            "estimated_time": 20.5
        }"#;

        let error: ApiError = serde_json::from_str(json).unwrap();
        assert_eq!(error.error, "Model too busy");
        assert_eq!(error.estimated_time, Some(20.5));
    }

    #[test]
    fn create_repo_request_serialization() {
        let request = CreateRepoBody {
            name: "my-model".to_string(),
            repo_type: Some(RepoType::Model),
            private: true,
            ..Default::default()
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"name\":\"my-model\""));
        assert!(json.contains("\"type\":\"model\""));
        assert!(json.contains("\"private\":true"));
    }

    #[test]
    fn default_values() {
        assert_eq!(RepoType::default(), RepoType::Model);
        assert_eq!(SortField::default(), SortField::LastModified);
        assert_eq!(SortDirection::default(), SortDirection::Descending);
        assert_eq!(GatedStatus::default(), GatedStatus::False);
        assert_eq!(SpaceHardware::default(), SpaceHardware::CpuBasic);
    }
}
