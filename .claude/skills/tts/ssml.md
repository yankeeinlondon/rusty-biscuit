# SSML Deep Dive

Speech Synthesis Markup Language (SSML) is the W3C standard for controlling TTS output. Version 1.1 is the current recommendation.

## Core Structure

```xml
<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US">
  <!-- Content here -->
</speak>
```

All SSML must be well-formed XML with `<speak>` as the root element.

## Prosody Control

The `<prosody>` element controls pitch, rate, and volume.

```xml
<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US">
  <!-- Pitch: Hz values, percentages, or keywords -->
  <prosody pitch="high">This is higher pitched.</prosody>
  <prosody pitch="+20%">Twenty percent higher.</prosody>
  <prosody pitch="200Hz">Fixed at 200 Hz.</prosody>

  <!-- Rate: percentage or keywords -->
  <prosody rate="slow">Speaking slowly now.</prosody>
  <prosody rate="150%">Fifty percent faster.</prosody>

  <!-- Volume: dB, percentage, or keywords -->
  <prosody volume="loud">Speaking loudly!</prosody>
  <prosody volume="+6dB">Six decibels louder.</prosody>

  <!-- Combined -->
  <prosody pitch="+10%" rate="80%" volume="soft">
    Quiet, slower, higher.
  </prosody>
</speak>
```

**Keywords**:
- Pitch: `x-low`, `low`, `medium`, `high`, `x-high`
- Rate: `x-slow`, `slow`, `medium`, `fast`, `x-fast`
- Volume: `silent`, `x-soft`, `soft`, `medium`, `loud`, `x-loud`

## Voice Selection

```xml
<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US">
  <!-- By name -->
  <voice name="en-US-AvaNeural">Hello, I'm Ava.</voice>

  <!-- By language -->
  <voice xml:lang="fr-FR">Bonjour, comment allez-vous?</voice>

  <!-- Multiple voices in dialogue -->
  <voice name="en-US-AvaNeural">How are you today?</voice>
  <voice name="en-US-AndrewNeural">I'm doing great, thanks!</voice>
</speak>
```

## Breaks and Pauses

```xml
<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US">
  <!-- Time-based -->
  <p>First sentence.</p>
  <break time="1s"/>
  <p>After one second pause.</p>

  <!-- Strength-based -->
  <p>Before break.</p>
  <break strength="strong"/>
  <p>After strong break.</p>

  <!-- Milliseconds -->
  Welcome to our service.
  <break time="500ms"/>
  Let me help you.
</speak>
```

**Strength values**: `none`, `x-weak`, `weak`, `medium`, `strong`, `x-strong`

## Pronunciation Control

### Phoneme Element

Specify exact pronunciation using IPA or X-SAMPA.

```xml
<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US">
  <!-- IPA notation -->
  <phoneme alphabet="ipa" ph="təˈmeɪtoʊ">tomato</phoneme>
  or
  <phoneme alphabet="ipa" ph="təˈmɑːtəʊ">tomato</phoneme>

  <!-- X-SAMPA notation -->
  <phoneme alphabet="x-sampa" ph='t@"meItoU'>tomato</phoneme>
</speak>
```

### Say-As Element

Context-dependent interpretation.

```xml
<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US">
  <!-- Date -->
  <say-as interpret-as="date" format="mdy">01/25/2026</say-as>
  <!-- "January twenty-fifth, twenty twenty-six" -->

  <!-- Time -->
  <say-as interpret-as="time">14:30</say-as>
  <!-- "two thirty PM" -->

  <!-- Telephone -->
  <say-as interpret-as="telephone">1-800-555-1234</say-as>

  <!-- Currency -->
  <say-as interpret-as="currency">$42.99</say-as>
  <!-- "forty-two dollars and ninety-nine cents" -->

  <!-- Cardinal/Ordinal -->
  <say-as interpret-as="cardinal">42</say-as>  <!-- "forty-two" -->
  <say-as interpret-as="ordinal">42</say-as>   <!-- "forty-second" -->

  <!-- Spell out -->
  <say-as interpret-as="characters">SSML</say-as>
  <!-- "S S M L" -->
</speak>
```

## Embedded Audio

```xml
<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US">
  <p>Listen to this sound:</p>
  <audio src="https://example.com/sound.wav">
    Audio could not be loaded.
  </audio>
  <p>Did you hear it?</p>
</speak>
```

Fallback text is spoken if audio fails to load.

## External Lexicons

Reference custom pronunciation dictionaries.

```xml
<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US">
  <lexicon uri="https://example.com/medical-terms.pls"/>
  <p>The patient has pneumonia.</p>
</speak>
```

## Vendor Extensions

### Microsoft Azure (mstts namespace)

```xml
<speak version="1.0"
       xmlns="http://www.w3.org/2001/10/synthesis"
       xmlns:mstts="https://www.w3.org/2001/mstts"
       xml:lang="en-US">

  <!-- Speaking style -->
  <voice name="en-US-JennyNeural">
    <mstts:express-as style="cheerful" styledegree="2">
      I'm so happy to help you today!
    </mstts:express-as>
  </voice>

  <!-- Sad style -->
  <voice name="en-US-JennyNeural">
    <mstts:express-as style="sad">
      I'm sorry to hear that.
    </mstts:express-as>
  </voice>

  <!-- Multi-speaker dialogue -->
  <voice name="en-US-MultiTalker-Ava-Andrew:DragonHDLatestNeural">
    <mstts:dialog>
      <mstts:turn speaker="ava">Hello Andrew!</mstts:turn>
      <mstts:turn speaker="andrew">Hi Ava, how are you?</mstts:turn>
    </mstts:dialog>
  </voice>
</speak>
```

**Available Styles**: `cheerful`, `sad`, `angry`, `fearful`, `excited`, `friendly`, `hopeful`, `empathetic`, `newscast`, `customerservice`

### Amazon Polly Extensions

```xml
<speak>
  <!-- Whisper effect -->
  <amazon:effect name="whispered">This is whispered.</amazon:effect>

  <!-- Soft phonation -->
  <amazon:effect phonation="soft">Gentle voice.</amazon:effect>

  <!-- Auto breaths -->
  <amazon:auto-breaths>
    This long sentence will have natural breath sounds inserted automatically.
  </amazon:auto-breaths>
</speak>
```

## Data-SSML (HTML Integration)

1EdTech standard for embedding SSML in HTML.

```html
<!-- JSON-based SSML in data attributes -->
<div data-ssml='{"say-as": {"interpret-as": "date", "format": "mdy"}}'>
  The test is scheduled for 01/25/2026.
</div>

<div data-ssml='{"phoneme": {"ph": "oʊˈskɑːr", "alphabet": "ipa"}}'>
  Oscar is a common name.
</div>
```

Useful for accessibility in educational assessments.

## Best Practices

### 1. Always Include Required Attributes

```xml
<!-- Good -->
<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US">

<!-- Missing xmlns - may fail -->
<speak version="1.0" xml:lang="en-US">
```

### 2. Use Fallbacks

```xml
<audio src="sound.wav">Audio not available.</audio>

<voice name="en-US-SpecificVoice">
  <voice name="en-US-FallbackVoice">
    Content with fallback voice.
  </voice>
</voice>
```

### 3. Test Across Platforms

SSML support varies by provider:
- Azure: Most complete with mstts extensions
- Google: Good core support, limited extensions
- AWS Polly: Core support plus amazon extensions

### 4. Handle Special Characters

```xml
<!-- Escape XML entities -->
<speak version="1.0" xmlns="http://www.w3.org/2001/10/synthesis" xml:lang="en-US">
  5 &lt; 10 and 10 &gt; 5
  Use &amp; for ampersand
</speak>
```

## Common Issues

### Pronunciation Problems

```xml
<!-- Problem: "read" is ambiguous -->
I read the book. / I will read the book.

<!-- Solution: Use phoneme -->
I <phoneme alphabet="ipa" ph="rɛd">read</phoneme> the book.
I will <phoneme alphabet="ipa" ph="rid">read</phoneme> the book.
```

### Numbers and Dates

```xml
<!-- Problem: "2025" said as "two thousand twenty-five" -->
<say-as interpret-as="date" format="y">2025</say-as>

<!-- Or force digit-by-digit -->
<say-as interpret-as="characters">2025</say-as>
```

### Acronyms

```xml
<!-- Problem: "ASAP" might be pronounced wrong -->
<say-as interpret-as="characters">ASAP</say-as>
<!-- or -->
<phoneme alphabet="ipa" ph="ˌeɪ.ɛsˌeɪˈpiː">ASAP</phoneme>
```

## Resources

- [W3C SSML 1.1 Specification](https://www.w3.org/TR/speech-synthesis11/)
- [Azure SSML Reference](https://learn.microsoft.com/en-us/azure/ai-services/speech-service/speech-synthesis-markup)
- [Google Cloud SSML](https://cloud.google.com/text-to-speech/docs/ssml)
- [Amazon Polly SSML](https://docs.aws.amazon.com/polly/latest/dg/ssml.html)
