# ElevenLabs Creative Platform API - Technical Specification

> **Base URL:** `https://api.elevenlabs.io`
> **Authentication:** `xi-api-key` header (required for all endpoints)
> **Content-Type:** `application/json` (unless otherwise specified)

## Table of Contents

- [ElevenLabs Creative Platform API - Technical Specification](#elevenlabs-creative-platform-api---technical-specification)
    - [Table of Contents](#table-of-contents)
    - [Authentication](#authentication)
    - [Regional Endpoints](#regional-endpoints)
        - [REST API](#rest-api)
        - [WebSocket API](#websocket-api)
    - [Text-to-Speech](#text-to-speech)
        - [HTTP Endpoints](#http-endpoints)
            - [Create Speech](#create-speech)
            - [Stream Speech](#stream-speech)
            - [Create Speech with Timestamps](#create-speech-with-timestamps)
            - [Stream Speech with Timestamps](#stream-speech-with-timestamps)
        - [WebSocket Endpoints](#websocket-endpoints)
            - [Text-to-Speech WebSocket](#text-to-speech-websocket)
            - [Multi-Context WebSocket](#multi-context-websocket)
    - [Voices](#voices)
        - [Voice Management](#voice-management)
            - [List Voices](#list-voices)
            - [Get Voice](#get-voice)
            - [Edit Voice](#edit-voice)
            - [Delete Voice](#delete-voice)
            - [Find Similar Voices](#find-similar-voices)
        - [Voice Settings](#voice-settings)
            - [Get Default Voice Settings](#get-default-voice-settings)
            - [Get Voice Settings](#get-voice-settings)
            - [Update Voice Settings](#update-voice-settings)
        - [Voice Samples](#voice-samples)
            - [Get Voice Sample Audio](#get-voice-sample-audio)
            - [Delete Voice Sample](#delete-voice-sample)
        - [Voice Library](#voice-library)
            - [List Shared Voices](#list-shared-voices)
            - [Add Shared Voice](#add-shared-voice)
        - [Professional Voice Cloning (PVC)](#professional-voice-cloning-pvc)
            - [Create PVC Voice](#create-pvc-voice)
            - [Update PVC Voice](#update-pvc-voice)
            - [Train PVC Voice](#train-pvc-voice)
            - [Add Samples to PVC Voice](#add-samples-to-pvc-voice)
            - [Other PVC Endpoints](#other-pvc-endpoints)
    - [Sound Effects](#sound-effects)
        - [Create Sound Effect](#create-sound-effect)
    - [Core Resources](#core-resources)
        - [Models](#models)
            - [List Models](#list-models)
        - [Single-Use Tokens](#single-use-tokens)
            - [Create Single-Use Token](#create-single-use-token)
        - [History](#history)
            - [Get Generated Items](#get-generated-items)
            - [Get History Item](#get-history-item)
            - [Delete History Item](#delete-history-item)
            - [Get History Item Audio](#get-history-item-audio)
            - [Download History Items](#download-history-items)
    - [Workspace](#workspace)
        - [Usage Statistics](#usage-statistics)
            - [Get Usage Stats](#get-usage-stats)
        - [User Information](#user-information)
            - [Get User](#get-user)
            - [Get User Subscription](#get-user-subscription)
        - [Resources](#resources)
            - [Get Resource](#get-resource)
            - [Share Resource](#share-resource)
            - [Unshare Resource](#unshare-resource)
            - [Copy Resource to Workspace](#copy-resource-to-workspace)
    - [Service Accounts](#service-accounts)
        - [List Service Accounts](#list-service-accounts)
        - [List Service Account API Keys](#list-service-account-api-keys)
        - [Create API Key](#create-api-key)
        - [Update API Key](#update-api-key)
        - [Delete API Key](#delete-api-key)
    - [Webhooks](#webhooks)
        - [List Webhooks](#list-webhooks)
        - [Create Webhook](#create-webhook)
        - [Update Webhook](#update-webhook)
        - [Delete Webhook](#delete-webhook)
    - [Common Types](#common-types)
        - [Alignment Types](#alignment-types)
        - [HTTP Alignment Object](#http-alignment-object)
        - [WebSocket Alignment Object](#websocket-alignment-object)
        - [Output Formats](#output-formats)
        - [Resource Types](#resource-types)
        - [Voice Categories](#voice-categories)
        - [Subscription Statuses](#subscription-statuses)
    - [Error Handling](#error-handling)
    - [SDK Support](#sdk-support)

---

## Authentication

All API requests require authentication via the `xi-api-key` header:

```http
xi-api-key: your_api_key_here
```

For WebSocket connections, authentication can be provided via:

- Header: `xi-api-key`
- Query parameter: `authorization` or `single_use_token`

---

## Regional Endpoints

### REST API

| Region  | Base URL                    |
|---------|-----------------------------|
| Default | `https://api.elevenlabs.io` |

### WebSocket API

| Region  | Base URL                               |
|---------|----------------------------------------|
| Default | `wss://api.elevenlabs.io`              |
| US      | `wss://api.us.elevenlabs.io`           |
| EU      | `wss://api.eu.residency.elevenlabs.io` |
| India   | `wss://api.in.residency.elevenlabs.io` |

---

## Text-to-Speech

### HTTP Endpoints

#### Create Speech

Converts text into speech and returns audio.

```http
POST /v1/text-to-speech/{voice_id}
```

**Path Parameters:**

| Parameter  | Type   | Required | Description      |
|------------|--------|----------|------------------|
| `voice_id` | string | Yes      | Voice identifier |

**Query Parameters:**

| Parameter                    | Type    | Default         | Description                                            |
|------------------------------|---------|-----------------|--------------------------------------------------------|
| `enable_logging`             | boolean | `true`          | Disable retention when `false` (enterprise only)       |
| `optimize_streaming_latency` | integer | `0`             | Latency optimization (0-4, higher = more optimization) |
| `output_format`              | string  | `mp3_44100_128` | Audio codec format                                     |

**Request Body:**

| Field                               | Type    | Required | Default                  | Description                             |
|-------------------------------------|---------|----------|--------------------------|-----------------------------------------|
| `text`                              | string  | Yes      | -                        | Text to convert to speech               |
| `model_id`                          | string  | No       | `eleven_multilingual_v2` | TTS model                               |
| `language_code`                     | string  | No       | -                        | ISO 639-1 language code                 |
| `voice_settings`                    | object  | No       | -                        | Voice configuration                     |
| `pronunciation_dictionary_locators` | array   | No       | -                        | Up to 3 dictionaries                    |
| `seed`                              | integer | No       | -                        | Deterministic generation (0-4294967295) |
| `previous_text`                     | string  | No       | -                        | Context for continuity                  |
| `next_text`                         | string  | No       | -                        | Context for continuity                  |
| `previous_request_ids`              | array   | No       | -                        | Previous request IDs for chaining       |
| `next_request_ids`                  | array   | No       | -                        | Next request IDs for chaining           |
| `apply_text_normalization`          | string  | No       | `auto`                   | `auto`, `on`, or `off`                  |
| `apply_language_text_normalization` | boolean | No       | -                        | Language-specific normalization         |

**Voice Settings Object:**

| Field               | Type    | Default | Range   | Description                      |
|---------------------|---------|---------|---------|----------------------------------|
| `stability`         | number  | `0.5`   | 0-1     | Voice stability/randomness       |
| `similarity_boost`  | number  | `0.75`  | 0-1     | Adherence to original voice      |
| `style`             | number  | `0`     | -       | Style exaggeration (V2+ models)  |
| `speed`             | number  | `1.0`   | 0.7-1.2 | Speech rate                      |
| `use_speaker_boost` | boolean | `true`  | -       | Enhanced similarity (V2+ models) |

**Response:** Binary audio (`application/octet-stream`)

---

#### Stream Speech

Streams audio as it's generated.

```http
POST /v1/text-to-speech/{voice_id}/stream
```

Parameters identical to Create Speech endpoint.

**Response:** Streaming audio (`text/event-stream`)

---

#### Create Speech with Timestamps

Returns audio with character-level timing information.

```http
POST /v1/text-to-speech/{voice_id}/with-timestamps
```

Parameters identical to Create Speech endpoint.

**Response:**

```json
{
  "audioBase64": "base64_encoded_audio",
  "alignment": {
    "characters": ["H", "e", "l", "l", "o"],
    "characterStartTimesSeconds": [0.0, 0.1, 0.15, 0.2, 0.25],
    "characterEndTimesSeconds": [0.1, 0.15, 0.2, 0.25, 0.35]
  },
  "normalizedAlignment": {
    "characters": ["H", "e", "l", "l", "o"],
    "characterStartTimesSeconds": [0.0, 0.1, 0.15, 0.2, 0.25],
    "characterEndTimesSeconds": [0.1, 0.15, 0.2, 0.25, 0.35]
  }
}
```

See [HTTP Alignment Object](#http-alignment-object) for type details.

---

#### Stream Speech with Timestamps

Streams audio chunks with timing information.

```http
POST /v1/text-to-speech/{voice_id}/stream/with-timestamps
```

Parameters identical to Create Speech endpoint.

**Response:** Streaming JSON chunks with `audioBase64` and `alignment` data (same format as above).

---

### WebSocket Endpoints

#### Text-to-Speech WebSocket

Real-time streaming TTS with word-to-audio alignment.

```text
GET /v1/text-to-speech/{voice_id}/stream-input
```

**Connection Parameters (Query String):**

| Parameter                  | Type    | Default         | Description           |
|----------------------------|---------|-----------------|-----------------------|
| `model_id`                 | string  | -               | TTS model             |
| `language_code`            | string  | -               | Target language       |
| `enable_logging`           | boolean | `true`          | Enable logging        |
| `enable_ssml_parsing`      | boolean | `false`         | Parse SSML            |
| `output_format`            | string  | `mp3_44100_128` | Audio codec           |
| `inactivity_timeout`       | integer | `20`            | Timeout in seconds    |
| `sync_alignment`           | boolean | `false`         | Synchronize alignment |
| `auto_mode`                | boolean | `false`         | Auto mode             |
| `apply_text_normalization` | string  | `auto`          | Text normalization    |
| `seed`                     | integer | -               | Reproducibility seed  |

**Client Messages:**

*Initialize Connection (First Message):*

```json
{
  "text": " ",
  "voice_settings": {
    "stability": 0.5,
    "similarity_boost": 0.75,
    "style": 0,
    "use_speaker_boost": true,
    "speed": 1
  },
  "generation_config": {
    "chunk_length_schedule": [120, 160, 250, 290]
  },
  "pronunciation_dictionary_locators": [
    { "pronunciation_dictionary_id": "...", "version_id": "..." }
  ],
  "xi-api-key": "optional_inline_key"
}
```

*Send Text:*

```json
{
  "text": "Your text here ",
  "try_trigger_generation": false,
  "flush": false
}
```

> **Note:** Text should always end with a single space character.

*Close Connection:*

```json
{
  "text": ""
}
```

**Server Messages:**

*Audio Output:*

```json
{
  "audio": "base64_encoded_audio_chunk",
  "alignment": {
    "charStartTimesMs": [0, 3, 7, 12],
    "charDurationsMs": [3, 4, 5, 3],
    "chars": ["H", "e", "l", "l"]
  },
  "normalizedAlignment": {
    "charStartTimesMs": [0, 3, 7, 12],
    "charDurationsMs": [3, 4, 5, 3],
    "chars": ["H", "e", "l", "l"]
  }
}
```

See [WebSocket Alignment Object](#websocket-alignment-object) for type details.

*Final Output:*

```json
{
  "isFinal": true
}
```

**Chunk Length Schedule:**
The `chunk_length_schedule` array controls buffering behavior. Default `[120, 160, 250, 290]` means:

- First chunk: wait for 120+ characters
- Second chunk: wait for 160+ characters
- Third chunk: wait for 250+ characters
- Fourth+ chunks: wait for 290+ characters

---

#### Multi-Context WebSocket

Manages multiple independent audio streams over a single connection.

```text
GET /v1/text-to-speech/{voice_id}/multi-stream-input
```

**Limits:** Maximum 5 concurrent contexts per connection.

**Client Messages:**

*Initialize Connection:*

```json
{
  "text": " ",
  "context_id": "context_1",
  "voice_settings": { ... },
  "generation_config": { ... }
}
```

*Initialize Additional Context:*

```json
{
  "text": " ",
  "context_id": "context_2",
  "voice_settings": { ... }
}
```

*Send Text to Context:*

```json
{
  "text": "Hello world ",
  "context_id": "context_1",
  "flush": false
}
```

*Flush Context:*

```json
{
  "context_id": "context_1",
  "flush": true
}
```

*Close Context:*

```json
{
  "context_id": "context_1",
  "close_context": true
}
```

*Close Socket:*

```json
{
  "close_socket": true
}
```

*Keep Context Alive:*

```json
{
  "context_id": "context_1",
  "text": ""
}
```

**Server Messages:**

*Audio Output:*

```json
{
  "audio": "base64_encoded_audio",
  "contextId": "context_1",
  "alignment": {
    "charStartTimesMs": [0, 3, 7, 12],
    "charDurationsMs": [3, 4, 5, 3],
    "chars": ["H", "e", "l", "l"]
  },
  "normalizedAlignment": {
    "charStartTimesMs": [0, 3, 7, 12],
    "charDurationsMs": [3, 4, 5, 3],
    "chars": ["H", "e", "l", "l"]
  }
}
```

See [WebSocket Alignment Object](#websocket-alignment-object) for type details.

*Final Output:*

```json
{
  "isFinal": true,
  "contextId": "context_1"
}
```

**Best Practices:**

1. Use a single connection per user session
1. Stream text in smaller chunks with `flush: true` at sentence ends
1. Close interrupted contexts before creating new ones
1. Close unused contexts promptly (20s inactivity timeout)
1. Send empty text to reset inactivity timer during processing delays

---

## Voices

### Voice Management

#### List Voices

```http
GET /v2/voices
```

**Query Parameters:**

| Parameter             | Type    | Default | Description                                                             |
|-----------------------|---------|---------|-------------------------------------------------------------------------|
| `page_size`           | integer | `10`    | Results per page (max 100)                                              |
| `next_page_token`     | string  | -       | Pagination token                                                        |
| `search`              | string  | -       | Search in name, description, labels, category                           |
| `voice_type`          | string  | -       | `personal`, `community`, `default`, `workspace`, `non-default`, `saved` |
| `category`            | string  | -       | `premade`, `cloned`, `generated`, `professional`                        |
| `collection_id`       | string  | -       | Filter by collection                                                    |
| `sort`                | string  | -       | `created_at_unix` or `name`                                             |
| `sort_direction`      | string  | -       | `asc` or `desc`                                                         |
| `fine_tuning_state`   | string  | -       | PVC states: `draft`, `queued`, `fine_tuning`, etc.                      |
| `voice_ids`           | array   | -       | Lookup specific IDs (max 100)                                           |
| `include_total_count` | boolean | `true`  | Include total in response                                               |

**Response:**

```json
{
  "voices": [VoiceResponseModel],
  "has_more": boolean,
  "total_count": integer,
  "next_page_token": "string | null"
}
```

---

#### Get Voice

```http
GET /v1/voices/{voice_id}
```

**Response:** `VoiceResponseModel`

```json
{
  "voice_id": "string",
  "name": "string",
  "category": "generated | cloned | premade | professional | famous | high_quality",
  "samples": [SampleModel],
  "settings": VoiceSettingsModel,
  "fine_tuning": FineTuningModel,
  "sharing": SharingModel,
  "verified_languages": [LanguageModel],
  "voice_verification": VoiceVerificationModel
}
```

---

#### Edit Voice

```http
POST /v1/voices/{voice_id}/edit
Content-Type: multipart/form-data
```

**Form Parameters:**

| Parameter                 | Type     | Required | Description                            |
|---------------------------|----------|----------|----------------------------------------|
| `name`                    | string   | Yes      | Voice display name                     |
| `files`                   | binary[] | No       | Audio files to add                     |
| `remove_background_noise` | boolean  | No       | Apply audio isolation                  |
| `description`             | string   | No       | Voice description                      |
| `labels`                  | object   | No       | Labels (language, accent, gender, age) |

**Response:**

```json
{
  "status": "ok"
}
```

---

#### Delete Voice

```http
DELETE /v1/voices/{voice_id}
```

**Response:**

```json
{
  "status": "ok"
}
```

---

#### Find Similar Voices

```http
POST /v1/similar-voices
Content-Type: multipart/form-data
```

**Form Parameters:**

| Parameter              | Type    | Required | Description                                  |
|------------------------|---------|----------|----------------------------------------------|
| `audio_file`           | binary  | Yes      | Audio sample for comparison                  |
| `similarity_threshold` | number  | No       | Match strictness (0-2, lower = more similar) |
| `top_k`                | integer | No       | Max voices to return (1-100)                 |

**Response:**

```json
{
  "voices": [LibraryVoiceResponseModel],
  "has_more": boolean,
  "last_sort_id": "string | null"
}
```

---

### Voice Settings

#### Get Default Voice Settings

```http
GET /v1/voices/settings/default
```

**Response:**

```json
{
  "stability": 0.5,
  "similarity_boost": 0.75,
  "use_speaker_boost": true,
  "style": 0,
  "speed": 1.0
}
```

---

#### Get Voice Settings

```http
GET /v1/voices/{voice_id}/settings
```

**Response:** Same as default settings.

---

#### Update Voice Settings

```http
POST /v1/voices/{voice_id}/settings/edit
```

**Request Body:**

```json
{
  "stability": 0.5,
  "similarity_boost": 0.75,
  "use_speaker_boost": true,
  "style": 0,
  "speed": 1.0
}
```

**Response:**

```json
{
  "status": "ok"
}
```

---

### Voice Samples

#### Get Voice Sample Audio

```http
GET /v1/voices/{voice_id}/samples/{sample_id}/audio
```

**Response:** Binary audio (`application/octet-stream`)

---

#### Delete Voice Sample

```http
DELETE /v1/voices/{voice_id}/samples/{sample_id}
```

**Response:**

```json
{
  "status": "ok"
}
```

---

### Voice Library

#### List Shared Voices

```http
GET /v1/shared-voices
```

**Query Parameters:**

| Parameter      | Type    | Default | Description                              |
|----------------|---------|---------|------------------------------------------|
| `page_size`    | integer | `30`    | Results per page (max 100)               |
| `page`         | integer | `0`     | Page number                              |
| `category`     | string  | -       | `professional`, `famous`, `high_quality` |
| `gender`       | string  | -       | Gender filter                            |
| `age`          | string  | -       | Age filter                               |
| `accent`       | string  | -       | Accent filter                            |
| `language`     | string  | -       | Language filter                          |
| `locale`       | string  | -       | Locale filter                            |
| `search`       | string  | -       | Search term                              |
| `use_cases`    | array   | -       | Use case filters                         |
| `descriptives` | array   | -       | Descriptive filters                      |
| `featured`     | boolean | -       | Featured voices only                     |
| `owner_id`     | string  | -       | Filter by owner                          |
| `sort`         | string  | -       | Sort criteria                            |

**Response:**

```json
{
  "voices": [LibraryVoiceResponseModel],
  "has_more": boolean,
  "last_sort_id": "string | null"
}
```

---

#### Add Shared Voice

```http
POST /v1/voices/add/{public_user_id}/{voice_id}
```

**Request Body:**

```json
{
  "new_name": "string"
}
```

**Response:**

```json
{
  "voice_id": "string"
}
```

---

### Professional Voice Cloning (PVC)

#### Create PVC Voice

```http
POST /v1/voices/pvc
```

**Request Body:**

| Field         | Type   | Required | Description                            |
|---------------|--------|----------|----------------------------------------|
| `name`        | string | Yes      | Voice name                             |
| `language`    | string | Yes      | Language used in samples               |
| `description` | string | No       | Voice description                      |
| `labels`      | object | No       | Labels (language, accent, gender, age) |

**Response:**

```json
{
  "voice_id": "string"
}
```

---

#### Update PVC Voice

```http
POST /v1/voices/pvc/{voice_id}
```

Same parameters as Create PVC Voice.

---

#### Train PVC Voice

```http
POST /v1/voices/pvc/{voice_id}/train
```

**Request Body:**

```json
{
  "model_id": "string | null"
}
```

**Response:**

```json
{
  "status": "ok"
}
```

---

#### Add Samples to PVC Voice

```http
POST /v1/voices/pvc/{voice_id}/samples
Content-Type: multipart/form-data
```

**Form Parameters:**

| Parameter                 | Type     | Required | Description           |
|---------------------------|----------|----------|-----------------------|
| `files`                   | binary[] | Yes      | Audio files           |
| `remove_background_noise` | boolean  | No       | Apply audio isolation |

**Response:**

```json
[
  {
    "sample_id": "string",
    "file_name": "string",
    "mime_type": "string",
    "size_bytes": integer,
    "hash": "string",
    "duration_secs": number,
    "remove_background_noise": boolean,
    "has_isolated_audio": boolean,
    "speaker_separation": {
      "status": "not_started | pending | completed | failed"
    },
    "trim_start": number,
    "trim_end": number
  }
]
```

---

#### Other PVC Endpoints

| Method | Endpoint                                                                  | Description                 |
|--------|---------------------------------------------------------------------------|-----------------------------|
| POST   | `/v1/voices/pvc/{voice_id}/samples/{sample_id}`                           | Update sample               |
| DELETE | `/v1/voices/pvc/{voice_id}/samples/{sample_id}`                           | Delete sample               |
| GET    | `/v1/voices/pvc/{voice_id}/samples/{sample_id}/audio`                     | Get sample audio            |
| GET    | `/v1/voices/pvc/{voice_id}/samples/{sample_id}/waveform`                  | Get waveform                |
| GET    | `/v1/voices/pvc/{voice_id}/samples/{sample_id}/speaker-separation-status` | Get separation status       |
| POST   | `/v1/voices/pvc/{voice_id}/samples/{sample_id}/separate-speakers`         | Start speaker separation    |
| GET    | `/v1/voices/pvc/{voice_id}/samples/{sample_id}/separated-speaker-audio`   | Get separated audio         |
| POST   | `/v1/voices/pvc/{voice_id}/verification/request`                          | Request manual verification |
| GET    | `/v1/voices/pvc/{voice_id}/verification/captcha`                          | Get verification captcha    |
| POST   | `/v1/voices/pvc/{voice_id}/verification/captcha/verify`                   | Verify captcha              |

---

## Sound Effects

#### Create Sound Effect

```http
POST /v1/sound-generation
```

**Query Parameters:**

| Parameter       | Type   | Default         | Description        |
|-----------------|--------|-----------------|--------------------|
| `output_format` | string | `mp3_44100_128` | Audio codec format |

**Request Body:**

| Field              | Type    | Required | Default                   | Description                      |
|--------------------|---------|----------|---------------------------|----------------------------------|
| `text`             | string  | Yes      | -                         | Text prompt for sound effect     |
| `duration_seconds` | number  | No       | auto                      | Duration (0.5-30 seconds)        |
| `loop`             | boolean | No       | -                         | Create looping effect (v2 model) |
| `prompt_influence` | number  | No       | `0.3`                     | Prompt adherence (0-1)           |
| `model_id`         | string  | No       | `eleven_text_to_sound_v2` | Sound generation model           |

**Response:** Binary audio (`application/octet-stream`)

---

## Core Resources

### Models

#### List Models

```http
GET /v1/models
```

**Response:**

```json
[
  {
    "model_id": "string",
    "name": "string",
    "description": "string",
    "token_cost_factor": number,
    "can_do_text_to_speech": boolean,
    "can_do_voice_conversion": boolean,
    "can_be_finetuned": boolean,
    "can_use_style": boolean,
    "can_use_speaker_boost": boolean,
    "serves_pro_voices": boolean,
    "languages": [
      { "language_id": "string", "name": "string" }
    ],
    "model_rates": {
      "character_cost_multiplier": number
    },
    "maximum_text_length_per_request": integer,
    "max_characters_request_free_user": integer,
    "max_characters_request_subscribed_user": integer,
    "requires_alpha_access": boolean,
    "concurrency_group": "string"
  }
]
```

---

### Single-Use Tokens

#### Create Single-Use Token

```http
POST /v1/single-use-token/{token_type}
```

**Path Parameters:**

| Parameter    | Type   | Values                             | Description   |
|--------------|--------|------------------------------------|---------------|
| `token_type` | string | `realtime_scribe`, `tts_websocket` | Token purpose |

**Response:**

```json
{
  "token": "string"
}
```

> **Note:** Token expires after 15 minutes and is consumed on first use.

---

### History

#### Get Generated Items

```http
GET /v1/history
```

**Query Parameters:**

| Parameter                     | Type    | Default | Description                 |
|-------------------------------|---------|---------|-----------------------------|
| `page_size`                   | integer | `100`   | Results per page (max 1000) |
| `start_after_history_item_id` | string  | -       | Pagination cursor           |
| `voice_id`                    | string  | -       | Filter by voice             |
| `model_id`                    | string  | -       | Filter by model             |
| `date_before_unix`            | integer | -       | Items before timestamp      |
| `date_after_unix`             | integer | -       | Items after timestamp       |
| `source`                      | string  | -       | `TTS` or `STS`              |
| `search`                      | string  | -       | Search term                 |
| `sort_direction`              | string  | `desc`  | `asc` or `desc`             |

**Response:**

```json
{
  "history": [SpeechHistoryItemResponseModel],
  "last_history_item_id": "string | null",
  "has_more": boolean,
  "scanned_until": integer | null
}
```

**History Item Model:**

```json
{
  "history_item_id": "string",
  "voice_id": "string",
  "voice_name": "string",
  "voice_category": "premade | cloned | generated | professional",
  "model_id": "string",
  "text": "string",
  "date_unix": integer,
  "content_type": "string",
  "state": "string",
  "feedback": FeedbackModel | null,
  "source": "TTS | STS | Projects | Dubbing | ...",
  "alignments": AlignmentModel,
  "dialogue": [DialogueModel] | null
}
```

---

#### Get History Item

```http
GET /v1/history/{history_item_id}
```

**Response:** `SpeechHistoryItemResponseModel`

---

#### Delete History Item

```http
DELETE /v1/history/{history_item_id}
```

**Response:**

```json
{
  "status": "ok"
}
```

---

#### Get History Item Audio

```http
GET /v1/history/{history_item_id}/audio
```

**Response:** Binary audio (`application/octet-stream`)

---

#### Download History Items

```http
POST /v1/history/download
```

**Request Body:**

```json
{
  "history_item_ids": ["string"],
  "output_format": "wav | null"
}
```

**Response:**

- Single item: Audio file (`application/octet-stream`)
- Multiple items: ZIP archive (`application/octet-stream`)

---

## Workspace

### Usage Statistics

#### Get Usage Stats

```http
GET /v1/usage/character-stats
```

**Required Query Parameters:**

| Parameter    | Type    | Description                    |
|--------------|---------|--------------------------------|
| `start_unix` | integer | Start timestamp (milliseconds) |
| `end_unix`   | integer | End timestamp (milliseconds)   |

**Optional Query Parameters:**

| Parameter                   | Type    | Default | Description                                                                             |
|-----------------------------|---------|---------|-----------------------------------------------------------------------------------------|
| `include_workspace_metrics` | boolean | `false` | Include workspace stats                                                                 |
| `breakdown_type`            | string  | `none`  | `voice`, `user`, `groups`, `api_keys`, `model`, `region`, etc.                          |
| `aggregation_interval`      | string  | -       | `hour`, `day`, `week`, `month`, `cumulative`                                            |
| `aggregation_bucket_size`   | integer | -       | Custom window in seconds                                                                |
| `metric`                    | string  | -       | `credits`, `tts_characters`, `minutes_used`, `request_count`, `ttfb_avg`, `concurrency` |

**Response:**

```json
{
  "time": [unix_timestamps],
  "usage": {
    "category": [values]
  }
}
```

---

### User Information

#### Get User

```http
GET /v1/user
```

**Response:**

```json
{
  "user_id": "string",
  "subscription": SubscriptionModel,
  "is_new_user": boolean,
  "xi_api_key": "string | null",
  "is_onboarding_completed": boolean,
  "first_name": "string | null",
  "created_at": integer
}
```

---

#### Get User Subscription

```http
GET /v1/user/subscription
```

**Response:**

```json
{
  "tier": "string",
  "character_count": integer,
  "character_limit": integer,
  "max_character_limit_extension": integer,
  "voice_slots_used": integer,
  "voice_limit": integer,
  "professional_voice_slots_used": integer,
  "professional_voice_limit": integer,
  "status": "trialing | active | incomplete | past_due | free | free_disabled",
  "billing_period": "monthly | 3-month | 6-month | annual",
  "currency": "USD | EUR | INR",
  "next_invoice": InvoiceModel,
  "open_invoices": [InvoiceModel],
  "pending_change": ChangeModel | null
}
```

---

### Resources

#### Get Resource

```http
GET /v1/workspace/resources/{resource_id}
```

**Query Parameters:**

| Parameter       | Type   | Required | Description                          |
|-----------------|--------|----------|--------------------------------------|
| `resource_type` | string | Yes      | Resource category (see Common Types) |

**Response:**

```json
{
  "resource_id": "string",
  "resource_type": "string",
  "creator_user_id": "string",
  "anonymous_access_level_override": "admin | editor | commenter | viewer | null",
  "role_to_group_ids": object,
  "share_options": [ShareOptionModel]
}
```

---

#### Share Resource

```http
POST /v1/workspace/resources/{resource_id}/share
```

**Request Body:**

| Field                  | Type   | Required | Description                              |
|------------------------|--------|----------|------------------------------------------|
| `role`                 | string | Yes      | `admin`, `editor`, `commenter`, `viewer` |
| `resource_type`        | string | Yes      | Resource category                        |
| `user_email`           | string | No       | Target user email                        |
| `group_id`             | string | No       | Target group (`default` for standard)    |
| `workspace_api_key_id` | string | No       | Target API key                           |

---

#### Unshare Resource

```http
POST /v1/workspace/resources/{resource_id}/unshare
```

**Request Body:**

| Field                  | Type   | Required | Description       |
|------------------------|--------|----------|-------------------|
| `resource_type`        | string | Yes      | Resource category |
| `user_email`           | string | No       | Target user email |
| `group_id`             | string | No       | Target group      |
| `workspace_api_key_id` | string | No       | Target API key    |

---

#### Copy Resource to Workspace

```http
POST /v1/workspace/resources/{resource_id}/copy-to-workspace
```

**Request Body:**

```json
{
  "resource_type": "string",
  "target_user_id": "string"
}
```

---

## Service Accounts

#### List Service Accounts

```http
GET /v1/service-accounts
```

**Response:**

```json
{
  "service-accounts": [
    {
      "service_account_user_id": "string",
      "name": "string",
      "created_at_unix": integer | null,
      "api-keys": [ApiKeyModel]
    }
  ]
}
```

---

#### List Service Account API Keys

```http
GET /v1/service-accounts/{service_account_user_id}/api-keys
```

**Response:**

```json
{
  "api-keys": [
    {
      "name": "string",
      "hint": "string",
      "key_id": "string",
      "service_account_user_id": "string",
      "created_at_unix": integer | null,
      "is_disabled": boolean,
      "permissions": [string] | null,
      "character_limit": integer | null,
      "character_count": integer | null
    }
  ]
}
```

---

#### Create API Key

```http
POST /v1/service-accounts/{service_account_user_id}/api-keys
```

**Request Body:**

```json
{
  "name": "string",
  "permissions": ["string"] | "all",
  "character_limit": integer | null
}
```

**Available Permissions:**

- `text_to_speech`, `speech_to_speech`, `speech_to_text`
- `models_read`, `models_write`
- `voices_read`, `voices_write`
- `speech_history_read`, `speech_history_write`
- `sound_generation`, `audio_isolation`, `voice_generation`
- `dubbing_read`, `dubbing_write`
- `pronunciation_dictionaries_read`, `pronunciation_dictionaries_write`
- `user_read`, `user_write`
- `projects_read`, `projects_write`
- `audio_native_read`, `audio_native_write`
- `workspace_read`, `workspace_write`
- `forced_alignment`
- `convai_read`, `convai_write`
- `music_generation`

**Response:**

```json
{
  "xi-api-key": "string",
  "key_id": "string"
}
```

---

#### Update API Key

```http
PATCH /v1/service-accounts/{service_account_user_id}/api-keys/{api_key_id}
```

**Request Body:**

```json
{
  "is_enabled": boolean,
  "name": "string",
  "permissions": ["string"] | "all",
  "character_limit": integer | null
}
```

---

#### Delete API Key

```http
DELETE /v1/service-accounts/{service_account_user_id}/api-keys/{api_key_id}
```

---

## Webhooks

#### List Webhooks

```http
GET /v1/workspace/webhooks
```

**Query Parameters:**

| Parameter        | Type    | Default | Description                        |
|------------------|---------|---------|------------------------------------|
| `include_usages` | boolean | `false` | Include active usages (admin only) |

**Response:**

```json
{
  "webhooks": [
    {
      "name": "string",
      "webhook_id": "string",
      "webhook_url": "string",
      "is_disabled": boolean,
      "is_auto_disabled": boolean,
      "created_at_unix": integer,
      "auth_type": "hmac | oauth2 | mtls",
      "usage": [ProductModel] | null,
      "most_recent_failure_error_code": integer | null,
      "most_recent_failure_timestamp": integer | null
    }
  ]
}
```

---

#### Create Webhook

```http
POST /v1/workspace/webhooks
```

**Request Body:**

```json
{
  "settings": {
    "auth_type": "hmac",
    "name": "string",
    "webhook_url": "string"
  }
}
```

**Response:**

```json
{
  "webhook_id": "string",
  "webhook_secret": "string | null"
}
```

---

#### Update Webhook

```http
PATCH /v1/workspace/webhooks/{webhook_id}
```

**Request Body:**

```json
{
  "is_disabled": boolean,
  "name": "string"
}
```

**Response:**

```json
{
  "status": "ok"
}
```

---

#### Delete Webhook

```http
DELETE /v1/workspace/webhooks/{webhook_id}
```

**Response:**

```json
{
  "status": "ok"
}
```

---

## Common Types

### Alignment Types

ElevenLabs uses **different alignment formats** for HTTP and WebSocket APIs:

| API                       | Time Unit    | Format            | Fields                                                                 |
|---------------------------|--------------|-------------------|------------------------------------------------------------------------|
| HTTP (`/with-timestamps`) | seconds      | start + end times | `characters`, `characterStartTimesSeconds`, `characterEndTimesSeconds` |
| WebSocket (TTS streaming) | milliseconds | start + duration  | `chars`, `charStartTimesMs`, `charDurationsMs`                         |

**`alignment` vs `normalizedAlignment`:**

Both APIs return two alignment objects:

- **`alignment`** — Timing data for the original input text
- **`normalizedAlignment`** — Timing data for normalized text (e.g., `"123"` → `"one hundred twenty-three"`)

---

### HTTP Alignment Object

Used in HTTP responses from `/with-timestamps` endpoints. Times are in **seconds** (float).

```json
{
  "characters": ["H", "e", "l", "l", "o"],
  "characterStartTimesSeconds": [0.0, 0.1, 0.15, 0.2, 0.25],
  "characterEndTimesSeconds": [0.1, 0.15, 0.2, 0.25, 0.35]
}
```

| Field                        | Type       | Description                              |
|------------------------------|------------|------------------------------------------|
| `characters`                 | `string[]` | Array of individual characters           |
| `characterStartTimesSeconds` | `number[]` | Start time for each character in seconds |
| `characterEndTimesSeconds`   | `number[]` | End time for each character in seconds   |

---

### WebSocket Alignment Object

Used in WebSocket audio output messages. Times are in **milliseconds** (integer).

```json
{
  "chars": ["H", "e", "l", "l", "o"],
  "charStartTimesMs": [0, 3, 7, 12, 18],
  "charDurationsMs": [3, 4, 5, 6, 4]
}
```

| Field              | Type        | Description                                   |
|--------------------|-------------|-----------------------------------------------|
| `chars`            | `string[]`  | Array of individual characters                |
| `charStartTimesMs` | `integer[]` | Start time for each character in milliseconds |
| `charDurationsMs`  | `integer[]` | Duration of each character in milliseconds    |

> **Note:** To calculate end time: `charStartTimesMs[i] + charDurationsMs[i]`

---

### Output Formats

| Format           | Sample Rate | Bitrate  | Notes        |
|------------------|-------------|----------|--------------|
| `mp3_22050_32`   | 22.05 kHz   | 32 kbps  | Low quality  |
| `mp3_44100_32`   | 44.1 kHz    | 32 kbps  |              |
| `mp3_44100_64`   | 44.1 kHz    | 64 kbps  |              |
| `mp3_44100_96`   | 44.1 kHz    | 96 kbps  |              |
| `mp3_44100_128`  | 44.1 kHz    | 128 kbps | Default      |
| `mp3_44100_192`  | 44.1 kHz    | 192 kbps | High quality |
| `pcm_8000`       | 8 kHz       | -        |              |
| `pcm_16000`      | 16 kHz      | -        |              |
| `pcm_22050`      | 22.05 kHz   | -        |              |
| `pcm_24000`      | 24 kHz      | -        |              |
| `pcm_44100`      | 44.1 kHz    | -        |              |
| `ulaw_8000`      | 8 kHz       | -        | Telephony    |
| `alaw_8000`      | 8 kHz       | -        | Telephony    |
| `opus_48000_32`  | 48 kHz      | 32 kbps  |              |
| `opus_48000_64`  | 48 kHz      | 64 kbps  |              |
| `opus_48000_96`  | 48 kHz      | 96 kbps  |              |
| `opus_48000_128` | 48 kHz      | 128 kbps |              |
| `opus_48000_192` | 48 kHz      | 192 kbps |              |

### Resource Types

- `voice`
- `voice_collection`
- `pronunciation_dictionary`
- `dubbing`
- `project`
- `convai_conversation`
- `convai_agent`
- `convai_secret`
- `convai_knowledge_base`
- `convai_knowledge_base_document`
- `convai_tool`
- `convai_phone_number`
- `convai_widget`
- ... (additional ConvAI types)

### Voice Categories

| Value          | Description                   |
|----------------|-------------------------------|
| `premade`      | ElevenLabs default voices     |
| `cloned`       | Instant voice clones          |
| `generated`    | AI-generated voices           |
| `professional` | Professional voice clones     |
| `famous`       | Celebrity voices              |
| `high_quality` | High-quality community voices |

### Subscription Statuses

| Value           | Description           |
|-----------------|-----------------------|
| `trialing`      | Active trial period   |
| `active`        | Paid subscription     |
| `incomplete`    | Payment pending       |
| `past_due`      | Payment overdue       |
| `free`          | Free tier             |
| `free_disabled` | Disabled free account |

---

## Error Handling

All endpoints return standard HTTP status codes:

| Code  | Description                          |
|-------|--------------------------------------|
| `200` | Success                              |
| `400` | Bad request                          |
| `401` | Unauthorized (invalid API key)       |
| `403` | Forbidden (insufficient permissions) |
| `404` | Resource not found                   |
| `422` | Validation error                     |
| `429` | Rate limit exceeded                  |
| `500` | Internal server error                |

**Validation Error Response:**

```json
{
  "detail": [
    {
      "loc": ["body", "field_name"],
      "msg": "error message",
      "type": "error_type"
    }
  ]
}
```

---

## SDK Support

Official SDKs are available for:

- TypeScript/JavaScript
- Python
- Go
- Ruby
- Java
- PHP
- C#
- Swift

See [ElevenLabs Documentation](https://elevenlabs.io/docs) for SDK installation and usage.
