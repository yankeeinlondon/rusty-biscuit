# Multi Model Providers

Some TTS CLI's use a single model to do perform their TTS functionality, however, other CLI's are more "meta CLI's" and can be configured to use a variety of models, voices, etc.

## Echogarden

The `echogarden` library (available on `npm`) can be installed globally on a system and it will expose the `echogarden` CLI but it can also be used purely as a JS/TS library. In this case we're only interested in it as a CLI.

- use the `echogarden` skill for more detail on this library

An example of how you might use the CLI:

```sh
echogarden speak "Hello World." --engine=kokoro --voice=Michael
```

- We must specify an "engine" and a "voice"
- In this example we used the high quality (but relatively low hardware requirements) **kokoro** model/engine
-

### CLI Help

```sh
echogarden speak text [output files...] [options...]
    Speak the given text

echogarden speak-file inputFile [output files...] [options...]
    Speak the given text file

echogarden speak-url url [output files...] [options...]
    Speak the HTML document on the given URL

echogarden speak-wikipedia articleName [output files...] [options...]
    Speak the given Wikipedia article. Language edition can be specified by --language=<langCode>

echogarden transcribe audioFile [output files...] [options...]
    Transcribe a spoken audio file

echogarden align audioFile transcriptFile [output files...] [options...]
    Align spoken audio file to its transcript

echogarden translate-text inputFile [output files...] [options...]
    Translate text to a different language

echogarden translate-speech audioFile [output files...] [options...]
    Transcribe spoken audio file directly to a different language

echogarden align-translation audioFile translatedTranscriptFile [output files...] [options...]
    Align spoken audio file to its translated transcript

echogarden align-transcript-and-translation audioFile transcriptFile translatedTranscriptFile [output files...] [options...]
    Align spoken audio file to both its transcript and its translated transcript using a two-stage approach.

echogarden align-timeline-translation timelineFile translatedFile [output files...] [options...]
    Align a given timeline file to its translated text

echogarden detect-text-language inputFile [output files...] [options...]
    Detect language of textual file

echogarden detect-speech-language audioFile [output files...] [options...]
    Detect language of spoken audio file

echogarden detect-voice-activity audioFile [output files...] [options...]
    Detect voice activity in audio file

echogarden denoise audioFile [output files...] [options...]
    Apply speech denoising to audio file

echogarden isolate audioFile [output files...] [options...]
    Extract isolated voice track from an audio file

echogarden list-engines operation
    List available engines for the specified operation

echogarden list-voices tts-engine [output files...] [options...]
    List available voices for the specified TTS engine

echogarden install [package names...] [options...]
    Install one or more Echogarden packages

echogarden uninstall [package names...] [options...]
    Uninstall one or more Echogarden packages

echogarden list-packages [options...]
    List installed Echogarden packages

echogarden serve [options...]
    Start a server

Options reference: https://bit.ly/echogarden-options
```

### Listing Engines

To determine what "engines" are available you can run `echogarden list-engines speak`.

This should provide a list something like:

```txt
Identifier: vits
Name: VITS
Description: A high-quality end-to-end neural speech synthesis architecture.
Type: local

Identifier: kokoro
Name: Kokoro
Description: A high-quality neural speech synthesis model based on the StyleTTS 2 architecture.
Type: local

Identifier: pico
Name: SVOX Pico
Description: A legacy diphone-based speech synthesizer.
Type: local

Identifier: flite
Name: Flite
Description: A legacy diphone-based speech synthesizer.
Type: local

Identifier: gnuspeech
Name: Gnuspeech
Description: A legacy articulatory speech synthesizer.
Type: local

Identifier: espeak
Name: eSpeak NG
Description: A lightweight, highly multilingual, 'robot'-like formant-based speech synthesizer.
Type: local

Identifier: sam
Name: SAM (Software Automatic Mouth)
Description: A classic 'robot'-like speech synthesizer from 1982.
Type: local

Identifier: sapi
Name: SAPI
Description: Microsoft Speech API (Windows only).
Type: local

Identifier: msspeech
Name: Microsoft Speech Platform
Description: Microsoft Server Speech API (Windows only).
Type: local

Identifier: coqui-server
Name: Coqui TTS
Description: A deep learning toolkit for Text-to-Speech.
Type: server

Identifier: google-cloud
Name: Google Cloud
Description: Google Cloud text-to-speech service.
Type: cloud

Identifier: microsoft-azure
Name: Azure Cognitive Services
Description: Microsoft Azure cloud text-to-speech service.
Type: cloud

Identifier: amazon-polly
Name: Amazon Polly
Description: Amazon Polly (also: AWS Polly) cloud text-to-speech.
Type: cloud

Identifier: openai-cloud
Name: OpenAI Cloud
Description: OpenAI cloud text-to-speech.
Type: cloud

Identifier: elevenlabs
Name: ElevenLabs
Description: A generative AI text-to-speech cloud service.
Type: cloud

Identifier: deepgram
Name: Deepgram
Description: A generative AI text-to-speech cloud service.
Type: cloud

Identifier: google-translate
Name: Google Translate
Description: Unofficial text-to-speech API used by the Google Translate web interface.
Type: cloud

Identifier: microsoft-edge
Name: Microsoft Edge
Description: Unofficial text-to-speech API used by the Microsoft Edge browser.
Type: cloud

Identifier: streamlabs-polly
Name: Streamlabs Polly
Description: Unofficial text-to-speech API provided by Streamlabs.
Type: cloud
```

As you can see, this list contains a combination of **local** and **cloud** based TTS's. For our purposes in the `biscuit-speaks` library we're ONLY interested in the **vits** and **kokoro** engines as both produce high quality voices locally.

- **kokoro** voices should be considered `VoiceQuality::Excellent` quality in all cases
- **vits** voices have an identifier which ends with `-low`, `-medium`, or `-high`
    - We are only interested in medium and high quality voices, low quality voices should be filtered out
    - The quality here for a medium model should be marked as `VoiceQuality::Moderate` for a "medium" model
    - and `VoiceQuality::Good` for a "high" model


### Listing Voices

We can list the available voices with the `--list-voices` switch BUT we must provide an **engine**. For instance:

- `echogarden list-voices kokoro` will provide all the voices available to the `kokoro` model in a format like this:

    ```txt
    Identifier: Heart
    Languages: American English (en-US), English (en)
    Gender: female

    Identifier: Bella
    Languages: American English (en-US), English (en)
    Gender: female
    ```

- `echogarden list-voices vits` will provide all the voices available to the `vits` model in a format like this:

    ```txt
    Identifier: en_GB-danny-low
    Languages: British English (en-GB), English (en)
    Gender: male

    Identifier: en_GB-alba-medium
    Languages: British English (en-GB), English (en)
    Gender: female

    Identifier: en_GB-aru-medium
    Languages: British English (en-GB), English (en)
    Gender: unknown
    Speaker count: 12

    Identifier: en_GB-southern_english_female-low
    Languages: British English (en-GB), English (en)
    Gender: female
    ```


## Kokoro TTS CLI

To get the kokoro CLI to work you must explicitly state where to find the voices and model:

- `--model <path>` should point to a file `kokoro-v1.0.bin` (or something similar)
- `--voices <path>` should point to a file `voices-v1.0.bin` (or something similar)

If these files exist in the current working directory then they do NOT need to be specified but that would be highly unlikely for the purposes of this repo. To work around this we will rely on there being a:

- `KOKORO_MODEL` environment variable which points to the model
- `KOKORO_VOICES` environment variable which points to the voices

If both are _set_ then we are able to run:

- `kokoro-tts --model "${KOKORO_MODEL}" --voices "${KOKORO_VOICES}" --help-voices`

This will give us a list of voices in this format:

```sh
Consider using the pymupdf_layout package for a greatly improved page layout analysis.

Supported voices:
    1. af_alloy
    2. af_aoede
    3. af_bella
    4. af_heart
    5. af_jessica
    6. af_kore
    7. af_nicole
    8. af_nova
    9. af_river
    10. af_sarah
    11. af_sky
    12. am_adam
    13. am_echo
    14. am_eric
    15. am_fenrir
    16. am_liam
    17. am_michael
    18. am_onyx
    19. am_puck
    20. am_santa
    21. bf_alice
    22. bf_emma
    23. bf_isabella
    24. bf_lily
    25. bm_daniel
    26. bm_fable
    27. bm_george
    28. bm_lewis
    29. ef_dora
    30. em_alex
    31. em_santa
    32. ff_siwis
    33. hf_alpha
    34. hf_beta
    35. hm_omega
    36. hm_psi
    37. if_sara
    38. im_nicola
    39. jf_alpha
    40. jf_gongitsune
    41. jf_nezumi
    42. jf_tebukuro
    43. jm_kumo
    44. pf_dora
    45. pm_alex
    46. pm_santa
    47. zf_xiaobei
    48. zf_xiaoni
    49. zf_xiaoxiao
    50. zf_xiaoyi
```

- we can infer the gender from the prefix (`f` is female, `m` is male)
- while the other letter in the prefix appears to _suggest_ the supported language there is a `--lang <str>` which sets the language
- if you use a voice and then specify a language it doesn't support you'll get back text like this:

    ```txt
    💻❯ kokoro-tts hi.txt --model "${KOKORO_MODEL}" --voices "${KOKORO_VOICES}" --voice af_bella --lang "en-uk"

    Consider using the pymupdf_layout package for a greatly improved page layout analysis.
    Error getting supported languages: Unsupported language: en-uk
    Supported languages are: cmn, en-gb, en-us, fr-fr, it, ja
    ```

- **Note:** that in this case I mistakenly used `en-uk` instead of `en-gb`; using either of the valid English dialects was spoken with an american accent which I believe the `a` in the prefix of `af_bella` stands for.

- the `--help-languages` switch on the CLi provides us with the languages supported:

    ```sh
    💻❯ kokoro-tts --model "${KOKORO_MODEL}" --voices "${KOKORO_VOICES}" --help-languages

    Consider using the pymupdf_layout package for a greatly improved page layout analysis.

    Supported languages:
        cmn
        en-gb
        en-us
        fr-fr
        it
        ja
    ```


Now when we call:

- `kokoro-tts hi.txt --model "${KOKORO_MODEL}" --voices "${KOKORO_VOICES}" --voice af_bella`

We convert the input text (`hi.txt`) into a WAV file `hi.wav`.

In order for this to work within the `biscuit-speaks` library we should use the `tempfile` crate to create a temp directory to put the text and allow the CLI to convert it to a WAV file:

```rust
use tempfile::tempdir;
use std::fs::File;
use std::io::{Write, Result};

fn speak(self) -> Result<()> {
    // Create a directory inside the temporary folder of the OS
    let dir = tempdir()?;

    let file_path = dir.path().join("my-temporary-log.txt");
    let mut file = File::create(file_path)?;

    writeln!(file, self.text)?;


}
```


### Help System

```sh
Consider using the pymupdf_layout package for a greatly improved page layout analysis.

Usage: kokoro-tts <input_text_file> [<output_audio_file>] [options]

Commands:
    -h, --help         Show this help message
    --help-languages   List all supported languages
    --help-voices      List all available voices
    --merge-chunks     Merge existing chunks in split-output directory into chapter files

Options:
    --stream            Stream audio instead of saving to file
    --speed <float>     Set speech speed (default: 1.0)
    --lang <str>        Set language (default: en-us)
    --voice <str>       Set voice or blend voices (default: interactive selection)
    --split-output <dir> Save each chunk as separate file in directory
    --format <str>      Audio format: wav or mp3 (default: wav)
    --debug             Show detailed debug information
    --model <path>      Path to kokoro-v1.0.onnx model file (default: ./kokoro-v1.0.onnx)
    --voices <path>     Path to voices-v1.0.bin file (default: ./voices-v1.0.bin)

Input formats:
    .txt               Text file input
    .epub              EPUB book input (will process chapters)
    .pdf               PDF document input (extracts chapters from TOC or content)

Examples:
    kokoro-tts input.txt output.wav --speed 1.2 --lang en-us --voice af_sarah
    kokoro-tts input.epub --split-output ./chunks/ --format mp3
    kokoro-tts input.pdf output.wav --speed 1.2 --lang en-us --voice af_sarah
    kokoro-tts input.pdf --split-output ./chunks/ --format mp3
    kokoro-tts input.txt --stream --speed 0.8
    kokoro-tts input.txt output.wav --voice "af_sarah:60,am_adam:40"
    kokoro-tts input.txt --stream --voice "am_adam,af_sarah" # 50-50 blend
    kokoro-tts --merge-chunks --split-output ./chunks/ --format wav
    kokoro-tts --help-voices
    kokoro-tts --help-languages
    kokoro-tts input.epub --split-output ./chunks/ --debug
    kokoro-tts input.txt output.wav --model /path/to/model.onnx --voices /path/to/voices.bin
    kokoro-tts input.txt --model ./models/kokoro-v1.0.onnx --voices ./models/voices-v1.0.bin
```




