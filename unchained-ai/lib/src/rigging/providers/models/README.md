# Provider Models

This directory provides enumerations for every model provider we support in this repo with the goal of exposing each _known_ model that the provider provides. Of course this is a moving target as new models come out all the time. To address this dynamic nature of provider models we provide two things:

1. Enum Generation

   The enumerations you see in this directory were not hand written they were _generated_ by calling the `models` endpoint on each provider's [OpenAI Compatible API](../../../api/openai_api.rs)'s. This API endpoint lists out the current models each providers have and allows us to have an authoritative list whenever we rebuilt the enumerations.

   This means the enum variants are always up-to-date with the latest provider model offerings.

2. `Bespoke()` fallback

   If there is a need to use a model that isn't yet listed in the enumerations provided then each provider has a `Bespoke()` variant. You would use it like so:

      ```rust
      let model = ProviderModel::Anthropic(ProviderModelAnthropic::Bespoke("my-new-model".to_string()));
      ```

   In 99% of cases however, the model should already exist and furthermore you should consider using further abstractions like `ModelCapability` to describe the capabilities of the model you want.
