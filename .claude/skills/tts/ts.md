# TypeScript/npm TTS Libraries

TTS libraries for JavaScript/TypeScript, from cloud APIs to browser-native solutions.

## Quick Comparison

| Library | Type | Best For |
|---------|------|----------|
| `@google-cloud/text-to-speech` | Cloud | Enterprise, WaveNet quality |
| `@mastra/voice-azure` | Cloud | Azure ecosystem |
| `@lobehub/tts` | Multi-provider | Full-stack, React projects |
| `text-to-speech-js` | Browser | Simple web apps |
| `@capacitor-community/text-to-speech` | Mobile | Hybrid apps |

## Cloud-Based Libraries

### @google-cloud/text-to-speech

Official Google Cloud client with WaveNet and Neural2 voices.

```typescript
import textToSpeech from '@google-cloud/text-to-speech';
import * as fs from 'fs';

async function synthesize(text: string) {
  const client = new textToSpeech.TextToSpeechClient();

  const [response] = await client.synthesizeSpeech({
    input: { text },
    voice: {
      languageCode: 'en-US',
      name: 'en-US-Wavenet-D',
      ssmlGender: 'NEUTRAL',
    },
    audioConfig: {
      audioEncoding: 'MP3',
      speakingRate: 1.0,
      pitch: 0,
    },
  });

  fs.writeFileSync('output.mp3', response.audioContent as Buffer);
}

// With SSML
async function synthesizeWithSSML() {
  const client = new textToSpeech.TextToSpeechClient();

  const ssml = `
    <speak>
      <prosody rate="slow" pitch="+2st">
        Welcome to our application.
      </prosody>
      <break time="500ms"/>
      Let's get started!
    </speak>
  `;

  const [response] = await client.synthesizeSpeech({
    input: { ssml },
    voice: { languageCode: 'en-US', name: 'en-US-Neural2-F' },
    audioConfig: { audioEncoding: 'MP3' },
  });

  return response.audioContent;
}
```

**Setup**:
```bash
npm install @google-cloud/text-to-speech
export GOOGLE_APPLICATION_CREDENTIALS="path/to/key.json"
```

### @mastra/voice-azure

Azure Speech Services integration.

```typescript
import { AzureVoice } from '@mastra/voice-azure';

const voice = new AzureVoice({
  subscriptionKey: process.env.AZURE_SPEECH_KEY,
  region: 'eastus',
});

const audio = await voice.synthesize({
  text: 'Hello from Azure!',
  voice: 'en-US-JennyNeural',
  style: 'cheerful',
  rate: 1.1,
});
```

## Server-Side & Cross-Platform

### @lobehub/tts

Multi-provider library with React components. Supports Edge, OpenAI, and Azure.

```typescript
import { EdgeSpeechTTS } from '@lobehub/tts';
import WebSocket from 'ws';
import * as fs from 'fs';

// Node.js requires WebSocket polyfill
global.WebSocket = WebSocket as any;

async function generateSpeech() {
  const tts = new EdgeSpeechTTS({ locale: 'en-US' });

  const response = await tts.create({
    input: 'Hello from LobeHub TTS!',
    options: {
      voice: 'en-US-GuyNeural',
    },
  });

  const buffer = Buffer.from(await response.arrayBuffer());
  fs.writeFileSync('output.mp3', buffer);
}

// OpenAI provider
import { OpenAITTS } from '@lobehub/tts';

const openaiTTS = new OpenAITTS({
  apiKey: process.env.OPENAI_API_KEY,
});

const audio = await openaiTTS.create({
  input: 'Hello from OpenAI!',
  options: {
    model: 'tts-1-hd',
    voice: 'nova',
  },
});
```

**React Integration**:
```tsx
import { AudioPlayer, AudioVisualizer } from '@lobehub/tts/react';

function TTSComponent() {
  const [audioUrl, setAudioUrl] = useState<string>();

  return (
    <div>
      <AudioPlayer src={audioUrl} />
      <AudioVisualizer audioSrc={audioUrl} />
    </div>
  );
}
```

**Installation**:
```bash
npm install @lobehub/tts ws
```

## Browser-Native Libraries

### text-to-speech-js

Lightweight wrapper around Web Speech API.

```typescript
import { speak, getVoices, stop } from 'text-to-speech-js';

// Get available voices
const voices = await getVoices();
console.log(voices.map(v => v.name));

// Speak with options
speak('Hello world', {
  voice: voices.find(v => v.lang === 'en-US'),
  rate: 1.0,
  pitch: 1.0,
  volume: 1.0,
});

// Stop speaking
stop();
```

### jsvoice

Voice commands and speech synthesis for web apps.

```typescript
import { Voice } from 'jsvoice';

const voice = new Voice();

// Speak
voice.speak('Hello!');

// With callback
voice.speak('Processing complete.', {
  onEnd: () => console.log('Speech finished'),
  lang: 'en-US',
  rate: 1.2,
});

// Wake word detection (STT)
voice.listen('hey assistant', () => {
  console.log('Wake word detected!');
});
```

## Mobile & Hybrid Apps

### @capacitor-community/text-to-speech

Native TTS for Capacitor hybrid apps (iOS/Android).

```typescript
import { TextToSpeech } from '@capacitor-community/text-to-speech';

// Speak
await TextToSpeech.speak({
  text: 'Hello from Capacitor!',
  lang: 'en-US',
  rate: 1.0,
  pitch: 1.0,
  volume: 1.0,
  category: 'ambient',
});

// Get supported languages
const languages = await TextToSpeech.getSupportedLanguages();

// Get voices
const voices = await TextToSpeech.getSupportedVoices();

// Stop
await TextToSpeech.stop();
```

**Installation**:
```bash
npm install @capacitor-community/text-to-speech
npx cap sync
```

## Workflow-Specific

### @andresaya/n8n-nodes-edgetts

n8n workflow automation with 400+ Edge voices.

```typescript
// Used within n8n workflows
// Provides nodes for:
// - Voice filtering by language/gender
// - Base64 audio output
// - Batch TTS processing
```

### @truffle-ai/gemini-tts-server

MCP server for multi-speaker synthesis with Gemini.

```typescript
import { GeminiTTSServer } from '@truffle-ai/gemini-tts-server';

const server = new GeminiTTSServer({
  apiKey: process.env.GEMINI_API_KEY,
});

// Multi-speaker synthesis
const audio = await server.synthesize({
  speakers: [
    { name: 'Alice', text: 'Hello!' },
    { name: 'Bob', text: 'Hi there!' },
  ],
});
```

## Decision Guide

```
Building for browser only?
├── Need simple TTS -> text-to-speech-js
└── Need voice commands too -> jsvoice

Building Node.js server?
├── Need multiple providers -> @lobehub/tts
├── Using Google Cloud -> @google-cloud/text-to-speech
└── Using Azure -> @mastra/voice-azure

Building mobile hybrid app?
└── @capacitor-community/text-to-speech

Building automation workflows?
└── @andresaya/n8n-nodes-edgetts
```

## Important Considerations

### Authentication & Security

```typescript
// NEVER commit API keys
const apiKey = process.env.TTS_API_KEY;

// Use server-side for cloud APIs
// Don't expose keys in client-side code
```

### Browser Compatibility

Web Speech API support varies:
- Chrome, Edge: Full support
- Safari: Full support
- Firefox: Limited (synthesis yes, recognition limited)

### Rate Limiting

```typescript
// Implement backoff for cloud APIs
async function synthesizeWithRetry(text: string, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await tts.synthesize(text);
    } catch (error) {
      if (error.status === 429) {
        await sleep(1000 * Math.pow(2, i));
        continue;
      }
      throw error;
    }
  }
}
```

## Comparison Table

| Feature | Google Cloud | Azure | LobeHub | Browser API |
|---------|--------------|-------|---------|-------------|
| Server-side | Yes | Yes | Yes | No |
| Browser | No | No | Yes | Yes |
| Neural voices | Yes | Yes | Depends | No |
| SSML | Yes | Yes | Partial | No |
| Free tier | 4M chars/mo | 5M chars/mo | Varies | Unlimited |
| Offline | No | No | No | Yes |
