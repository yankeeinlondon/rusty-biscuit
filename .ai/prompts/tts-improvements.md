# TTS Improvements in Shared Library

ultrathink and review the TTS implementation in @shared/src/tts.rs.

We need to make a few adjustments to this implementation:

## Function Rationalization

1. The `announce_completion()`, `announce_research_complete()`, `format_completion_message()`, `format_research_message()` functions SHOULD NOT exist.
2. Speak struct

    ```rust
    struct Voice {
      language: Language,
      voice_stack: Vec<KnownVoice>,
      gender: Gender,
      default_volume: f32,
    };

    impl Default for Voice {
      fn default() -> Voice;
    }

    impl Voice {
      pub fn new();
      pub fn with_voice() -> this;
      pub fn of_gender() -> this;
      pub fn with_default_volume() -> this;
    }

    ```


3. Two Variants of Speak

     - the function `speak()` should return a `Result` type including an error if that occurs.
     - we should add a function called `speak_when_able()` which preserves the current speech function's approach to errors (e.g., ignoring them as the the TTS feature is a "nice to have")
4. All callers in the monorepo who used the functions being removed or the speak() function should use `speak_when_able()`!
5. Add an `available_voices()` function which returns the host's available voices
6.


> **Note:** current callers of the TTS functions which we are being removed will now need to add the prefix for their text themselves:
>
>- `announce_research_complete("abc")` -> `speak_when_able("Research for the abc library has completed")

## Speech Features

The library should offer a builder pattern to control aspects of the voice such as:

- volume: "soft" | "normal" | "loud"
- gender: "male" | "female"
- language: (with English as default)
- voice stack: _provide a list of preferred voices_ and the first "available" voice that is found on the system will be used

