use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ModelCapability {
    /// Use this when you want a fast and cheap model and you're sure that
    /// your cost savings will not impact the success of the query.
    ///
    /// **Note:** there are plenty of prompts which do not need the latest
    /// models to perform their task.
    FastCheap,
    /// A stack of fast models which are optimized for capability not cost.
    ///
    /// The **default** stack will use the latest "fast models" for all providers
    /// and tend to stack the US providers before Chinese and local options.
    ///
    /// Example model: `claude-haiku`
    Fast,
    /// A stack of models which provide strong capability while avoiding the
    /// top-stack models which might break the bank or be too slow.
    ///
    /// Typically these models will be _non-thinking_ or _low-thinking_ to
    /// keep performance/speed relatively quick.
    ///
    /// Example model: `claude-sonnet`
    Normal,
    /// A variant of the "Normal" capability which stacks all the less expensive
    /// models first and either _excludes_ the most expensive models in this class
    /// or puts at the end of the stack so that they are marked as _less preferred_.
    ///
    /// **Note:** The **default** stacks will not exclude but instead just put the
    /// the more expensive models at the _end_ of the stack. This allows more expensive
    /// models in this class to still be used as a fallback.
    NormalCheap,
    /// A stack of models which uses a similar class models as "Normal" but which have
    /// pushed to "think".
    NormalThinking,
    /// A stack of models which use a similar class of models as "NormalCheap" but which have
    /// pushed to "think".
    NormalThinkingCheap,

    NormalUltrathink,
    NormalCheapUltrathink,

    /// A stack of top-tier models only. These model's will not be asked to think in this
    /// setting so you'll get reasonably fast but smart responses.
    ///
    /// Example model: `claude-opus` (non-thinking)
    Smart,
    SmartCheap,

    /// A stack of top-tier models only who have been asked to operate in thinking mode.
    ///
    /// Example model: `claude-opus` (think)
    SmartThink,
    SmartCheapThink,

    /// A stack of top-tier models only who have been asked to take as much time as possible
    /// to think through the prompt (aka, `ultrathink`).
    ///
    /// Example model: `claude-opus` (ultrathink)
    SmartUltrathink,
    SmartCheapUltrathink,

    // Below you'll find some specialist settings for some edge cases

    // Creative models will have their temperatures lowered from default settings
    // to make them more creative
    CreativeFast,
    CreativeNormal,
    CreativeSmart,

    // Literal models will have their temperature increased, not to 1 but very close
    LiteralFast,
    LiteralNormal,
    LiteralSmart,
}
