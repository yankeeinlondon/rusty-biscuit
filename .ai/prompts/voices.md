# Voices and Gender

We recently refactored the `biscuit-speaks` library and while it was a step in the right direction is has caused some regressions:

- we can no longer list the voices which are "available" to a host
- there is some sort of variance when we specify gender but no voices sound male (just different)
- `echogarden` is detected as a "provider" but does not work (at least not via the `so-you-say` CLI)
- A LOT of the other providers we were "supposed" to support were NOT implemented at all!

## Solution Approach

Some of the symbols discussed here already have a good code starting point but because you find the symbol already in code does NOT mean it is complete. Always try to evaluate if an existing symbol can be made better.

### Voice Inventory By Provider

To solve this we will start with identifying the available voices the host has, not in general, but _per_ provider. When we identify the available voices we will also try to discern:

- the voice's gender
- the language(s) supported for the voice
- the quality level of the voice (`VoiceQuality` enum)
- a host's providers, and those provider's capabilities will be cached


## Technical Solution

### `TtsExecutor` Trait

we have a `TtsExecutor` trait which defines the _interface_ which we must implement for each "provider" which we hope to support.

- Currently the only requirement for a provider is that implement the `speak()` function but going forward this will not be enough
- We will add the following:
    - `is_ready()`
          - We can quickly detect whether a given host has the TTS providers _executable program_ (for local providers) or the appropriate API Key (for cloud providers) and for some providers this is all that is required to be able to use the service
          - However, other providers, require one or more of the following:
              - has an internet connection (any cloud solution)
              - has the appropriate API Keys
              - has an ENV variable to point to a valid TTS model (for local abstracted solutions like `sherpa-onnx` or `echogarden`)
              - has an ENV variable to point to valid Voice samples
              - has a complimentary audio player which can render it's audio output
                  - some TTS solutions will send audio directly to OS's audio channel, others require
    - `list_voices()`
        - each provider -- local or cloud -- must be able to detect what voices are available.
        - voices must be defined to a gender; this includes TTS solutions which don't always explicitly express this in their metadata. To resolve gender-non-specific voices we will use the `gender_guesser` crate which uses a simple lookup based approach to assigning gender.
        - voices must also be matched to 1:M languages
        - voices must also be assigned a "quality" setting (`VoiceQuality`)
    - `info()`
        - will provide a markdown string of information about the provider
        - CLI's like `so-you-say` will use the Darkmatter library to provide rich descriptions of the various providers to the console
    - `available_voices()`
        - provides a list of voices which the TTS solution _can_ download and use but which are NOT currently on the host machine
        - we should only ever list voices which are of Good or Excellent voice quality
            - for instance the `say` command for a user probably has a subset of voices which "are" available
    - `async download_voice<T: DownloadableVoice>(voice: T) -> void`
        - for TTS providers where we can offer a simple library call to download a set of voices we will provide an enumeration with the available voices for download
        - this enum must be a member of the `DownloadableVoice` trait
        - for TTS providers which do not provides an enumerated set of voices for download this function will use the shared `NoDownloadableVoice` enumeration (_which is a member of `DownloadableVoice`) which has no valid options to choose from


### Host Capabilities Cache

In order to avoid having to _re-discover_ a given host's capabilities each time we will save a cache file with a host's capabilities. These capabilities are broken down by the provider's which are found available on the host and will be saved to a cache file located at `${HOME}/.biscuit-speaks-cache`.

#### Creating and Cache Busting

- the `biscuit-speaks` library will expose two utility functions:
    - `read_from_cache() -> HostTtsCapability`
    - and `update_provider_in_cache(provider, capabilities)` (primary user will be the )
    - and `bust_host_capability_cache()`
- the `so-you-speak` CLI will offer a `--force` flag which will bust the cache and force host rediscovery
- the `biscuit-speaks` providers will by default leverage the cached capabilities, so a caller of `Speak` struct should first call `bust_host_capability_cache()` if they want to ensure that the cache is refreshed.



### Adding Providers

We need to add the following providers:

- `SAPI` (on Windows)
- `Gtts`
- `echogarden`
- `kokoro-tts`

The first two shouldn't be hard.

- SAPI
    - is always available on Windows
    - how to calculate the voices installed and/or available will need to be determined
    -
