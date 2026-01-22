# Biscuit Speaks upgrade

Currently the `biscuit-speaks` package exports two utility functions:

- `speak_when_able`
- `speak`

Both have identical signatures but handle error conditions differently.

This functionality is wrapped around the `tts` crate and the original idea was to have a zero dependency way of leveraging what TTS capability the host had on it's system. However, the `tts` crate DOES require dependencies on Linux and possibly Windows too so we're going to move away from the `tts` package.

> The source file `biscuit-speaks/src/old.rs` contains many of the code used in the original/current implementation with ties to the `tts` crate which we're eliminating.

## Refactoring `biscuit-speaks`

We will refactor with the same goal we had originally in mind: leverage a host's TTS capabilities to provide TTS functionality. However, this time we will add in one cloud provider to allow some additional flexibility.

- We will use the `InstalledTtsClients` struct from the **Sniff** library (in this monorepo)
    - This will immediately give us a set of tools which the host has installed
- We will combine that with a type safe API client for [ElevenLabs](https://elevenlabs.com) TTS capabilities which is provided by the `schematic/schema` package (in this monorepo)

### Environment Variables

The following environment variables will play a role in which TTS solution and configuration we use on the host:

- `ELEVENLABS_API_KEY` or `ELEVEN_LABS_API_KEY`
    - when provided we know -- or at least strongly suspect -- that the **ElevenLabs** cloud TTS is an option
- `PREFER_TTS` -- if set to a valid TTS provider this is seen as an explicit desire to use this TTS provider. They will be placed at the top of the TTS stack (but we'll still fall back if for some reason their choice is not available)
- `PREFER_TTS_GENDER` will determine the preferred gender (male/female) of the voice used if the call didn't explicitly state this
- `PREFER_TTS_VOICE_MALE`, `PREFER_TTS_VOICE_FEMALE`
- `PREFER_TTS_LANG` or `PREFER_TTS_LANGUAGE`


### TTS Prioritization

The process we'll use to select the TTS provider is:

- create a force-ranked stack of desired options
    - these rankings are static by OS (see `LINUX_TTS_STACK`, `MACOS_TTS_STACK`, `WINDOWS_TTS_STACK`)
    - and generally equate to which TTS is better than another on that OS
    - exception:
        - the cloud option with ElevenLabs is always ranked last if there is an API Key (and not listed at all if there is no API Key)
        - we do this because use of the API _can_ cost money (although there is a generous free tier) and we want the user to explicitly opt-in to using the API for this reason
        - a user opts-in by setting the `PREFER_TTS` to `elevenlabs` (capitalization insensitive)
- then filter out those options which are _not available_ (because no local program or lack of API Key)
- We will then select the top ranked TTS provider which matches the desired language choice (which is English if not set otherwise)

### Voice, Gender, and Volume

The _gender_ of the speaker, the _volume_ at which the speaker speaks, and even the _specific voice_ used are things which are desirable to configure in a TTS:

- Volume and Gender are _relatively_ simple to abstract away from the actual TTS provider providing the service
    - like we did with the current implementation, the `VolumeLevel` enum is how we'll refer to volume
        - it provides `Soft`, `Normal`, and `Loud` as well as an explicit numeric value
        - if a TTS services doesn't provide volume control then we'll ignore the enum but where ever we can we'll try to honor the volume settings and map it to the TTS provider

## TTS Orchestration via the `Speak` struct

The main way that people will interact through the `biscuit-speaks` library is via the `Speak` struct

## Client's of this Library

There are no external client's of this library but there are several packages within this monorepo which depend on this library. Because this is a major breaking change these clients will all have to be adopted to the new API surface being provided.

- `so-you-say` is a simple CLI package which provides TTS services using **biscuit-speaks** as the underlying library
- `research/lib` is the shared library support the research package; it too interacts with this library
- `research/cli` apparently also interacts with this library

