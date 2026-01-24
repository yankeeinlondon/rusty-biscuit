# Cloud Provider TTS CLI Tools

CLI tools for major cloud TTS services.

## Summary

| Provider | CLI Tool | License | API Key | Free Tier |
|----------|----------|---------|---------|-----------|
| AWS | [AWS CLI](https://aws.amazon.com/cli/) | Apache-2.0 | Yes | 5M chars/month (12 mo) |
| Azure | [Azure CLI](https://learn.microsoft.com/en-us/cli/azure/) | MIT | Yes | 5M chars/month |
| Google | [gcloud CLI](https://cloud.google.com/sdk/docs/install) | Apache-2.0 | Yes | 4M chars/month |

## AWS Polly

Amazon's TTS service with Standard and Neural voices.

### Setup

```bash
# Install
pip install awscli
# or
brew install awscli

# Configure credentials
aws configure
# Enter: Access Key ID, Secret Access Key, Region

# Verify
aws polly describe-voices --language-code en-US
```

### Usage

```bash
# List voices
aws polly describe-voices --query 'Voices[*].{Name:Name,Gender:Gender,Engine:SupportedEngines}' --output table

# Synthesize to MP3
aws polly synthesize-speech \
  --output-format mp3 \
  --voice-id Joanna \
  --text "Hello from AWS Polly" \
  speech.mp3

# Neural voice
aws polly synthesize-speech \
  --output-format mp3 \
  --voice-id Joanna \
  --engine neural \
  --text "Neural voice synthesis" \
  neural.mp3

# With SSML
aws polly synthesize-speech \
  --output-format mp3 \
  --voice-id Joanna \
  --text-type ssml \
  --text '<speak>Hello <break time="500ms"/> world</speak>' \
  ssml.mp3

# Long-form (async)
aws polly start-speech-synthesis-task \
  --output-format mp3 \
  --output-s3-bucket-name my-bucket \
  --voice-id Joanna \
  --text "Very long text..." \
  --output-s3-key-prefix audio/
```

### Free Tier

- **Standard voices**: 5 million characters/month for 12 months
- **Neural voices**: 1 million characters/month for 12 months

## Azure Speech Service

Microsoft's TTS with 400+ neural voices.

### Setup

```bash
# Install
pip install azure-cli
# or
brew install azure-cli

# Login
az login

# Create Speech resource (if needed)
az cognitiveservices account create \
  --name my-speech-service \
  --resource-group my-rg \
  --kind SpeechServices \
  --sku F0 \
  --location eastus

# Get key
az cognitiveservices account keys list \
  --name my-speech-service \
  --resource-group my-rg
```

### Usage

```bash
# List voices (via REST API with curl)
curl -X GET "https://eastus.tts.speech.microsoft.com/cognitiveservices/voices/list" \
  -H "Ocp-Apim-Subscription-Key: YOUR_KEY"

# Synthesize (REST API)
curl -X POST "https://eastus.tts.speech.microsoft.com/cognitiveservices/v1" \
  -H "Ocp-Apim-Subscription-Key: YOUR_KEY" \
  -H "Content-Type: application/ssml+xml" \
  -H "X-Microsoft-OutputFormat: audio-16khz-128kbitrate-mono-mp3" \
  -d '<speak version="1.0" xml:lang="en-US">
        <voice name="en-US-JennyNeural">Hello from Azure!</voice>
      </speak>' \
  -o output.mp3

# With emotion/style
curl -X POST "https://eastus.tts.speech.microsoft.com/cognitiveservices/v1" \
  -H "Ocp-Apim-Subscription-Key: YOUR_KEY" \
  -H "Content-Type: application/ssml+xml" \
  -H "X-Microsoft-OutputFormat: audio-24khz-160kbitrate-mono-mp3" \
  -d '<speak version="1.0" xmlns:mstts="https://www.w3.org/2001/mstts" xml:lang="en-US">
        <voice name="en-US-JennyNeural">
          <mstts:express-as style="cheerful" styledegree="2">
            I am so happy to help you today!
          </mstts:express-as>
        </voice>
      </speak>' \
  -o cheerful.mp3
```

### Free Tier (F0)

- **Standard**: 5 million characters/month
- **Neural**: 500K characters/month
- Available every month (not just first 12)

## Google Cloud Text-to-Speech

WaveNet and Neural2 voices with 40+ languages.

### Setup

```bash
# Install
brew install google-cloud-sdk
# or see https://cloud.google.com/sdk/docs/install

# Initialize
gcloud init

# Authenticate
gcloud auth application-default login

# Enable API
gcloud services enable texttospeech.googleapis.com
```

### Usage

```bash
# List voices
gcloud ml speech voices list --filter="languageCodes:en-US"

# Synthesize (via REST API)
curl -X POST \
  -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  -H "Content-Type: application/json" \
  --data '{
    "input": {"text": "Hello from Google Cloud"},
    "voice": {
      "languageCode": "en-US",
      "name": "en-US-Wavenet-D"
    },
    "audioConfig": {"audioEncoding": "MP3"}
  }' \
  "https://texttospeech.googleapis.com/v1/text:synthesize" \
  | jq -r '.audioContent' | base64 -d > output.mp3

# With SSML
curl -X POST \
  -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  -H "Content-Type: application/json" \
  --data '{
    "input": {
      "ssml": "<speak>Hello <break time=\"500ms\"/> world</speak>"
    },
    "voice": {
      "languageCode": "en-US",
      "name": "en-US-Neural2-F"
    },
    "audioConfig": {
      "audioEncoding": "MP3",
      "speakingRate": 1.1
    }
  }' \
  "https://texttospeech.googleapis.com/v1/text:synthesize" \
  | jq -r '.audioContent' | base64 -d > ssml.mp3
```

### Free Tier

- **Standard voices**: 4 million characters/month (every month)
- **WaveNet voices**: 1 million characters/month (every month)
- **Neural2 voices**: Billed separately

## Comparison

| Feature | AWS Polly | Azure | Google Cloud |
|---------|-----------|-------|--------------|
| Neural voices | 20+ | 400+ | 30+ |
| Languages | 30+ | 140+ | 40+ |
| SSML | Yes | Yes (extended) | Yes |
| Emotion/Style | Limited | Yes (mstts tags) | Limited |
| Streaming | Yes | Yes | Yes |
| Custom voice | Yes (paid) | Yes (paid) | Yes (paid) |
| Free tier | 5M/month | 5M/month | 4M/month |

## Quick Scripts

### AWS Polly Wrapper

```bash
#!/bin/bash
# polly.sh - Simple Polly TTS
aws polly synthesize-speech \
  --output-format mp3 \
  --voice-id "${2:-Joanna}" \
  --engine neural \
  --text "$1" \
  /tmp/polly_output.mp3 && \
afplay /tmp/polly_output.mp3  # macOS
# or: mpv /tmp/polly_output.mp3  # Linux
```

### Google Cloud Wrapper

```bash
#!/bin/bash
# gcptts.sh - Simple Google Cloud TTS
curl -s -X POST \
  -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  -H "Content-Type: application/json" \
  --data "{
    \"input\": {\"text\": \"$1\"},
    \"voice\": {\"languageCode\": \"en-US\", \"name\": \"en-US-Neural2-F\"},
    \"audioConfig\": {\"audioEncoding\": \"MP3\"}
  }" \
  "https://texttospeech.googleapis.com/v1/text:synthesize" \
  | jq -r '.audioContent' | base64 -d > /tmp/gcp_output.mp3 && \
afplay /tmp/gcp_output.mp3
```

## When to Use Which

```
Need most voices/languages:
    -> Azure (400+ neural voices)

Need streaming for real-time:
    -> AWS Polly (good streaming API)

Need WaveNet quality:
    -> Google Cloud

Need emotion/style control:
    -> Azure (mstts:express-as tags)

Already in AWS ecosystem:
    -> AWS Polly

Generous free tier:
    -> Google Cloud (4M chars/month ongoing)
```
