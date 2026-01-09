use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::hash::Hash;
use std::{collections::HashMap, fs, path::Path};

use crate::primitives::runnable::Runnable;
use crate::primitives::state::PipelineState;
use crate::{models::model_capability::ModelCapability, utils::datetime::Epoch};

/// Errors that can occur when creating a Prompt from input data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptError {
    /// File exists but is not a valid text file (e.g., binary content)
    InvalidFileReference(String),
    /// File reference points to a non-existent file
    MissingFileReference(String),
    /// Bare URL without context text (use `image://` or `audio://` for remote resources)
    InvalidUrlReference(String),
    /// Binary content is not a recognized audio or image format
    InvalidBinaryPrompt,
    /// JSON value could not be deserialized into a Prompt
    InvalidJson(String),
}

impl std::fmt::Display for PromptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFileReference(path) => {
                write!(f, "file exists but is not a text file: {path}")
            }
            Self::MissingFileReference(path) => write!(f, "file not found: {path}"),
            Self::InvalidUrlReference(url) => {
                write!(f, "bare URL not allowed; use image:// or audio:// prefix: {url}")
            }
            Self::InvalidBinaryPrompt => {
                write!(f, "binary content is not a recognized audio or image format")
            }
            Self::InvalidJson(msg) => write!(f, "invalid JSON: {msg}"),
        }
    }
}

impl std::error::Error for PromptError {}

/// Detected content type from binary magic bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryContentType {
    Image,
    Audio,
}

/// Detects content type from binary data using magic bytes.
fn detect_binary_content_type(data: &[u8]) -> Option<BinaryContentType> {
    if data.len() < 4 {
        return None;
    }

    // Image formats
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        // PNG
        return Some(BinaryContentType::Image);
    }
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        // JPEG
        return Some(BinaryContentType::Image);
    }
    if data.starts_with(b"GIF8") {
        // GIF87a or GIF89a
        return Some(BinaryContentType::Image);
    }
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        // WebP
        return Some(BinaryContentType::Image);
    }
    if data.starts_with(b"<svg") || data.starts_with(b"<?xml") {
        // SVG (may start with XML declaration)
        return Some(BinaryContentType::Image);
    }

    // Audio formats
    if data.starts_with(&[0xFF, 0xFB]) || data.starts_with(&[0xFF, 0xFA]) {
        // MP3 frame sync
        return Some(BinaryContentType::Audio);
    }
    if data.starts_with(b"ID3") {
        // MP3 with ID3 tag
        return Some(BinaryContentType::Audio);
    }
    if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WAVE" {
        // WAV
        return Some(BinaryContentType::Audio);
    }
    if data.starts_with(b"OggS") {
        // OGG (could be Vorbis, Opus, etc.)
        return Some(BinaryContentType::Audio);
    }
    if data.starts_with(b"fLaC") {
        // FLAC
        return Some(BinaryContentType::Audio);
    }

    None
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PromptCapability {
    TextOnly(ModelCapability),
    MultiModal(ModelCapability),
    TextImageDocument(ModelCapability, ModelCapability, ModelCapability),
    TextDocument(ModelCapability),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SystemPromptStrategy {
    Append,
    Replace,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PromptImage {
    Raster(Vec<u8>),
    Svg(String),
    FileRef(String),
    Url(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptAudio(pub Vec<u8>);

pub enum StructuredValue {
    String(String),
    Integer(i32),
    Float(f32),
}

/// the **Prompt** struct represents a
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound(serialize = "V: Serialize", deserialize = "V: DeserializeOwned"))]
pub struct Prompt<V: Serialize + Hash + Eq> {
    /// The abstracted model/models you want to use to
    /// provide modalities to
    pub capability: Option<PromptCapability>,

    /// A bespoke system prompt
    pub system_prompt: Option<String>,
    /// How the bespoke system provided (if provided)
    /// should interact with the default system prompt
    /// for the given model.
    pub system_prompt_strategy: Option<SystemPromptStrategy>,

    /// text prompts
    pub text: Option<String>,
    /// local binary images
    pub images: Option<Vec<PromptImage>>,
    /// local audio content
    pub audio: Option<Vec<PromptAudio>>,

    pub prefer_multi_modal_model: bool,
    pub structured_response: Option<HashMap<String, V>>,

    ///
    pub has_external_resources: bool,
    /// whether URI based resources were all available
    /// at the check.
    pub missing_resources: bool,
    /// the last time resources were checked
    pub missing_resources_checked: Option<Epoch>,
}

impl<V: Serialize + Hash + Eq> Default for Prompt<V> {
    fn default() -> Self {
        Self {
            capability: None,
            system_prompt: None,
            system_prompt_strategy: None,
            text: None,
            images: None,
            audio: None,
            prefer_multi_modal_model: false,
            structured_response: None,
            has_external_resources: false,
            missing_resources: false,
            missing_resources_checked: None,
        }
    }
}

/*
TryFrom Business Logic

- `String` | `&str` | CowStr | Into<String>
            - if the string starts with `file://`:
                - local file check for `file://${filepath}`
                    - If file exists *and* is a text file it will be _eagerly_ loaded into the `text` property
                    - if file exists but is NOT a text file then return a `InvalidFileReference` error
                    - if the file is missing then return a `MissingFileReference` error
            - for `image://` and `audio://`
                - set the `has_external_resources` to `true`
                - HEAD check with `reqwest` crate (both `audio://`, and `image://` represent HTTPS resources)
                    - set `missing_resources` based on whether this
            - if the string starts and ends with a `http://${domain}/${path}` or `https://${domain}/${path}` this is clearly an unintentional mistake so return a `InvalidUrlReference`.
                - If the text content has more prose text _after_ an initial http(s) reference then this will be treated as text prompt rather than an error
            - all other textual prompts will be treated as text and:
                - will create a default prompt with the `text` property set
        - `Vec<u8>`
            - the binary content will evaluated,  and:
                - pushed onto the `audio` property if the binary content is a recognizable audio format
                - pushed onto the `images` property if the binary content is a recognized raster image
                - if not recognized it will return a `InvalidBinaryPrompt`
                -
*/

/// Checks if a string is a bare URL (just a URL with no prose after it).
fn is_bare_url(s: &str) -> bool {
    let trimmed = s.trim();
    if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
        return false;
    }
    // If there's no whitespace, it's just a URL
    // If whitespace exists, check if there's meaningful text after the URL
    match trimmed.find(char::is_whitespace) {
        None => true, // No whitespace = just a URL
        Some(idx) => {
            // Check if text after URL is meaningful (not just whitespace)
            trimmed[idx..].trim().is_empty()
        }
    }
}

impl<V: Serialize + Hash + Eq> TryFrom<String> for Prompt<V> {
    type Error = PromptError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut prompt = Self::default();

        // Handle file:// protocol - eagerly load text files
        if let Some(path) = value.strip_prefix("file://") {
            let file_path = Path::new(path);
            if !file_path.exists() {
                return Err(PromptError::MissingFileReference(path.to_string()));
            }

            // Attempt to read as UTF-8 text
            match fs::read_to_string(file_path) {
                Ok(content) => {
                    prompt.text = Some(content);
                    return Ok(prompt);
                }
                Err(_) => {
                    // File exists but couldn't be read as text
                    return Err(PromptError::InvalidFileReference(path.to_string()));
                }
            }
        }

        // Handle image:// protocol - external HTTPS image resource
        if let Some(url_path) = value.strip_prefix("image://") {
            prompt.has_external_resources = true;
            prompt.images = Some(vec![PromptImage::Url(format!("https://{url_path}"))]);
            // Note: HEAD check would require async; resource availability
            // should be verified separately or lazily
            return Ok(prompt);
        }

        // Handle audio:// protocol - external HTTPS audio resource
        if let Some(_url_path) = value.strip_prefix("audio://") {
            prompt.has_external_resources = true;
            // Audio URLs need to be fetched before we can create PromptAudio
            // Mark as having external resources; actual fetching is deferred
            // TODO: Consider adding PromptAudio::Url variant for lazy loading
            return Ok(prompt);
        }

        // Check for bare http/https URLs (error case)
        if is_bare_url(&value) {
            return Err(PromptError::InvalidUrlReference(value.trim().to_string()));
        }

        // All other text content
        prompt.text = Some(value);
        Ok(prompt)
    }
}

impl<V: Serialize + Hash + Eq> TryFrom<&str> for Prompt<V> {
    type Error = PromptError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
    }
}

impl<V: Serialize + Hash + Eq> TryFrom<&Vec<u8>> for Prompt<V> {
    type Error = PromptError;

    fn try_from(value: &Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl<V: Serialize + Hash + Eq> TryFrom<&[u8]> for Prompt<V> {
    type Error = PromptError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut prompt = Self::default();

        match detect_binary_content_type(value) {
            Some(BinaryContentType::Image) => {
                // Check if it's SVG (text-based) vs raster
                if value.starts_with(b"<svg") || value.starts_with(b"<?xml") {
                    let svg_content =
                        String::from_utf8(value.to_vec()).map_err(|_| PromptError::InvalidBinaryPrompt)?;
                    prompt.images = Some(vec![PromptImage::Svg(svg_content)]);
                } else {
                    prompt.images = Some(vec![PromptImage::Raster(value.to_vec())]);
                }
                Ok(prompt)
            }
            Some(BinaryContentType::Audio) => {
                prompt.audio = Some(vec![PromptAudio(value.to_vec())]);
                Ok(prompt)
            }
            None => Err(PromptError::InvalidBinaryPrompt),
        }
    }
}

impl<V: Serialize + Hash + Eq> TryFrom<Vec<u8>> for Prompt<V> {
    type Error = PromptError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.as_slice())
    }
}

impl<V: Serialize + DeserializeOwned + Hash + Eq> TryFrom<serde_json::Value> for Prompt<V> {
    type Error = PromptError;

    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value).map_err(|e| PromptError::InvalidJson(e.to_string()))
    }
}

impl<V> Runnable for Prompt<V>
where
    V: Serialize + Hash + Eq + Send + Sync + Clone + 'static,
{
    /// The output type is `String` representing the LLM's text response.
    ///
    /// For structured responses, use the `structured_response` field to define
    /// a schema and parse the response accordingly.
    type Output = String;

    /// Executing a `Prompt` will:
    ///
    /// 1. Identify the concrete `ProviderModel` to use
    /// 2. Call the LLM model with the prompt; providing tools
    ///    a system prompt, and structured output instructions
    ///    if they were added.
    fn execute(&self, _state: &mut PipelineState) -> Self::Output {
        todo!("LLM execution not yet implemented")
    }

    fn execute_readonly(&self, _state: &PipelineState) -> Self::Output {
        // Prompts can execute in read-only mode since they don't need to write state
        todo!("LLM execution not yet implemented")
    }

    fn name(&self) -> &str {
        "Prompt"
    }

    fn supports_readonly(&self) -> bool {
        // Prompts can run in parallel since they just call external LLMs
        true
    }
}

impl<V: Serialize + Hash + Eq> Prompt<V> {
    /// create a new prompt with text, an image, or audio
    pub fn new<T>(init: T) -> Self
    where
        Self: TryFrom<T>,
        <Self as TryFrom<T>>::Error: std::fmt::Debug,
    {
        Self::try_from(init).expect("Prompt creation failed")
    }

    // BUILDER PATTERN

    pub fn using_model(mut self, model: ModelCapability) -> Self {
        self.capability = Some(PromptCapability::TextOnly(model));
        self
    }

    pub fn with_image(mut self, image: PromptImage) -> Self {
        let mut images = self.images.unwrap_or_default();
        images.push(image);
        self.images = Some(images);
        self
    }

    pub fn with_audio(mut self, audio: PromptAudio) -> Self {
        let mut audios = self.audio.unwrap_or_default();
        audios.push(audio);
        self.audio = Some(audios);
        self
    }

    pub fn prefer_multi_modal(mut self) -> Self {
        self.prefer_multi_modal_model = true;
        self
    }

    pub fn with_structured_response(mut self, structure: HashMap<String, V>) -> Self {
        self.structured_response = Some(structure);
        self
    }

    // END BUILDER PATTERN

    /// Validates external resources by performing HEAD requests.
    ///
    /// Updates `missing_resources` to `true` if any external URL is unreachable,
    /// and sets `missing_resources_checked` to the current timestamp.
    ///
    /// This is a no-op if `has_external_resources` is `false`.
    pub async fn validate(&mut self) {
        if !self.has_external_resources {
            return;
        }

        let client = reqwest::Client::new();
        let mut any_missing = false;

        // Check image URLs
        if let Some(ref images) = self.images {
            for image in images {
                if let PromptImage::Url(url) = image {
                    if !Self::check_url_accessible(&client, url).await {
                        any_missing = true;
                    }
                }
            }
        }

        self.missing_resources = any_missing;
        self.missing_resources_checked = Some(Epoch::now());
    }

    /// Performs a HEAD request to check if a URL is accessible.
    async fn check_url_accessible(client: &reqwest::Client, url: &str) -> bool {
        match client.head(url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}
