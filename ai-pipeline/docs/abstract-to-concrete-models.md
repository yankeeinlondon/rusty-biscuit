# Abstract to Concrete Models

As has been mentioned the `Model` struct is quite abstract in that it should describe the _kind_ of model(s) you need but doesn't specify the actual LLM model or the provider it was sourced from yet.


In order to move from this abstract state to a concrete one where we can actually call models we will be leveraging several key symbols:

- [`ProviderModel`](./docs/ProviderModel.md) - the static set of known provider models
- [`PROVIDER_TO_ENV`](./docs/provider-to-env.md) - a static mapping table which allows us to know which ENV variables might contain a certain provider's API key.
