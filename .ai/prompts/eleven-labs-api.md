# ElevenLabs API

- research the ElevenLabs Creative Platform API:


    - TTS
        - [**WSS**: WebSocket](https://elevenlabs.io/docs/api-reference/text-to-speech/v-1-text-to-speech-voice-id-stream-input)

          The Text-to-Speech WebSockets API is designed to generate audio from partial text input while ensuring consistency throughout the generated audio. Although highly flexible, the WebSockets API isn’t a one-size-fits-all solution. It’s well-suited for scenarios where:

          The input text is being streamed or generated in chunks.
          Word-to-audio alignment information is required.
          However, it may not be the best choice when:

          The entire input text is available upfront. Given that the generations are partial, some buffering is involved, which could potentially result in slightly higher latency compared to a standard HTTP request.
          You want to quickly experiment or prototype. Working with WebSockets can be harder and more complex than using a standard HTTP API, which might slow down rapid development and testing.

        - [**WSS**: Multi-Context WebSocket](https://elevenlabs.io/docs/api-reference/text-to-speech/v-1-text-to-speech-voice-id-multi-stream-input)

          The Multi-Context Text-to-Speech WebSockets API allows for generating audio from text input while managing multiple independent audio generation streams (contexts) over a single WebSocket connection. This is useful for scenarios requiring concurrent or interleaved audio generations, such as dynamic conversational AI applications.

          Each context, identified by a context id, maintains its own state. You can send text to specific contexts, flush them, or close them independently. A close_socket message can be used to terminate the entire connection gracefully.

          For more information on best practices for how to use this API, please see the [multi context websocket guide](https://elevenlabs.io/docs/developers/guides/cookbooks/multi-context-web-socket).


        - [**POST**: Create Speech](https://elevenlabs.io/docs/api-reference/text-to-speech/convert)

          Converts text into speech using a voice of your choice and returns audio.

        - [**POST**: Create Speech with Timing](https://elevenlabs.io/docs/api-reference/text-to-speech/convert-with-timestamps)

          Generate speech from text with precise character-level timing information for audio-text synchronization.

        - [**STREAM**: Stream Speech](https://elevenlabs.io/docs/api-reference/text-to-speech/stream)

          Converts text into speech using a voice of your choice and returns audio as an audio stream.

        - [**STREAM**: Stream Speech with Timing](hhttps://elevenlabs.io/docs/api-reference/text-to-speech/stream-with-timestamps)

          Converts text into speech using a voice of your choice and returns a stream of JSONs containing audio as a base64 encoded string together with information on when which character was spoken.

    - Voices

        - [**GET**: List Voices](https://elevenlabs.io/docs/api-reference/voices/search)

          Gets a list of all available voices for a user with search, filtering and pagination.

        - [**GET**: Get Voice](https://elevenlabs.io/docs/api-reference/voices/get)

          Returns metadata about a specific voice.

        - [**DELETE**: Delete Voice](https://elevenlabs.io/docs/api-reference/voices/delete)
        - [**POST**: Edit Voice](https://elevenlabs.io/docs/api-reference/voices/update)
        - [**POST**: List Similar Voices](https://elevenlabs.io/docs/api-reference/voices/find-similar-voices)

          Returns a list of shared voices similar to the provided audio sample. If neither similarity_threshold nor top_k is provided, we will apply default values.

    - Voice Settings

        - [**GET**: Get Default Voice Settings](https://elevenlabs.io/docs/api-reference/voices/settings/get-default)

          Gets the default settings for voices. “similarity_boost” corresponds to”Clarity + Similarity Enhancement” in the web app and “stability” corresponds to “Stability” slider in the web app.

        - [**GET**: Get Voice Settings](https://elevenlabs.io/docs/api-reference/voices/settings/get)
        - [**POST**: Edit Voice Settings](https://elevenlabs.io/docs/api-reference/voices/settings/update)

    - Voice Samples

        - [**GET**: Get Voice Sample Audio](https://elevenlabs.io/docs/api-reference/voices/samples/get)
        - [**DELETE**: List Voices](https://elevenlabs.io/docs/api-reference/voices/samples/delete)

    - Voice Library

        - [**GET**: List Shared Voices](https://elevenlabs.io/docs/api-reference/voices/voice-library/get-shared)
        - [**POST**: Add Shared Voice](https://elevenlabs.io/docs/api-reference/voices/voice-library/share)

    - Voice IVC

        - TODO

    - Voice PVC

        - [**POST**: Create PVC Voice](https://elevenlabs.io/docs/api-reference/voices/pvc/create)
        - [**POST**: Update PVC Voice](https://elevenlabs.io/docs/api-reference/voices/pvc/update)
        - [**POST**: Train PVC Voice](https://elevenlabs.io/docs/api-reference/voices/pvc/train)
        - [**POST**: Add Samples to PVC Voice](https://elevenlabs.io/docs/api-reference/voices/pvc/samples/create)
        - [**POST**: Update PVC Voice Sample](https://elevenlabs.io/docs/api-reference/voices/pvc/samples/update)
        - [**DELETE**: Delete PVC Voice Sample](https://elevenlabs.io/docs/api-reference/voices/pvc/samples/delete)
        - [**GET**: Get PVC Voice Sample](https://elevenlabs.io/docs/api-reference/voices/pvc/samples/get-audio)
        - [**GET**: Get PVC Voice Sample Waveform](https://elevenlabs.io/docs/api-reference/voices/pvc/samples/get-waveform)
        - [**GET**: Get PVC Speaker Separation Status](https://elevenlabs.io/docs/api-reference/voices/pvc/samples/get-speaker-separation-status)
        - [**POST**: Start Speaker Separation](https://elevenlabs.io/docs/api-reference/voices/pvc/samples/separate-speakers)
        - [**GET**: Get Separated Audio](https://elevenlabs.io/docs/api-reference/voices/pvc/samples/get-separated-speaker-audio)
        - [**POST**: Request PVC Manual Verification](https://elevenlabs.io/docs/api-reference/voices/pvc/verification/request)
        - [**GET**: Get PVC Verification Captcha](https://elevenlabs.io/docs/api-reference/voices/pvc/verification/captcha)
        - [**POST**: Verify PVC Verification captcha](https://elevenlabs.io/docs/api-reference/voices/pvc/verification/captcha/verify)

    - Voice Changer
    - Voice Design

    - Sound Effects

        - [**POST**: Create Sound Effect](https://elevenlabs.io/docs/api-reference/text-to-sound-effects/convert)

          Turn text into sound effects for your videos, voice-overs or video games using the most advanced sound effects models in the world.

    - Audio Isolation
    - Dubbing
    - Forced Alignment
    - Pronunciation Dictionaries
    - Audio Native
    - Text to Dialog
    - Music
    - Studio





> **NOTE:** this is separate from the ElevenLabs Agents Platform and also the "Core Resources" but we are going to bundle the "Core Resources" into this API

- Core Resources
    - Models
        - [**GET**: List models](https://elevenlabs.io/docs/api-reference/models/list)
    - Tokens
        - [**POST**: Create Single Use Token](https://elevenlabs.io/docs/api-reference/tokens/create)
    - History
        - [**GET**: Get Generated Items](https://elevenlabs.io/docs/api-reference/history/list)
        - [**GET**: Get History Item](https://elevenlabs.io/docs/api-reference/history/get)
        - [**DELETE**: Delete History Item](https://elevenlabs.io/docs/api-reference/history/delete)
        - [**GET**: Get Audio from History Item](https://elevenlabs.io/docs/api-reference/history/get-audio)
        - [**POST**: Download History Items](https://elevenlabs.io/docs/api-reference/history/download)
- Workspace
    - [**GET**: Usage](https://elevenlabs.io/docs/api-reference/usage/get)
    - [**GET**: User](https://elevenlabs.io/docs/api-reference/user/get)
    - [**GET**: Get User Subscription](https://elevenlabs.io/docs/api-reference/user/subscription/get)
    - Resources
        - [**GET**: Get Resource](https://elevenlabs.io/docs/api-reference/workspace/resources/get)
        - [**POST**: Share Workspace Resource](https://elevenlabs.io/docs/api-reference/workspace/resources/share)
        - [**POST**: Unshare Workspace Resource](https://elevenlabs.io/docs/api-reference/workspace/resources/unshare)
        - [**POST**: Copy Resource to Other Workspace](https://elevenlabs.io/docs/api-reference/workspace/resources/copy-to-workspace)
- Service Accounts
    - [**GET**: Get Service Accounts](https://elevenlabs.io/docs/api-reference/service-accounts/list)
    - API Keys
        - [**GET**: API Keys](https://elevenlabs.io/docs/api-reference/service-accounts/api-keys/list)
        - [**POST**: Create API Keys](https://elevenlabs.io/docs/api-reference/service-accounts/api-keys/create)
        - [**DELETE**: Delete API Keys](https://elevenlabs.io/docs/api-reference/service-accounts/api-keys/delete)
        - [**PATCH**: Update API Keys](https://elevenlabs.io/docs/api-reference/service-accounts/api-keys/update)
- Webhooks
    - [**GET**: List Webhooks](https://elevenlabs.io/docs/api-reference/webhooks/list)
    - [**POST**: Create Webhook](https://elevenlabs.io/docs/api-reference/webhooks/create)
    - [**DELETE**: Delete Webhook](https://elevenlabs.io/docs/api-reference/webhooks/delete)
    - [**PATCH**: Update Webhooks](https://elevenlabs.io/docs/api-reference/webhooks/update)
