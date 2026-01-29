# Research Technical Design





## Content Policy

To reinforce the idea of a content policy for research documents we will leverage:

- the `ContentExpiry` enum is a **reason** why the content will become stale/invalid
- the `ContentPolicy` struct which is just a container of `ContentExpiry` reasons

### Content Expiry

- critically, each "reason" must be matched with a way to **test** if this reason has been reached

The simplest kind of reason is just a time based expiry but reasons should also include:

- **Flagged Content** - if a document has the `stale` property in frontmatter set to **true** then this will expire the content
- **Hash Conflict** - if the body/content of a document -- _after being trimmed (to remove meaningless edge whitespace)_ -- no longer equals the `content_hash` hash value then this means the content has been changed and is in an invalid state
- **Software Update** - if a major (or minor) release of some software is released this can invalidate certain documents
- **Model Archived** - if the model used to generate some prompt-driven content (designated in the `model` frontmatter property) has been deemed "archived" in favor of newer model then a document can be invalidated.
    - a more sophisticated variant might also consider the `model_category` frontmatter property (e.g., "fast", "smart", etc.)

Both `ContentExpiry` and `ContentPolicy` are found in the research library's **metadata** module.

**IMPORTANT:** there is currently a draft implementation of these types; they are not finalized so if there are design considerations pushing you in a somewhat different direction then that is valid.


## Topic Struct

The `Topic` struct is defined in the **metadata** package of the research library and defines the metadata for a **topic** in the research library.

- this replaces the schema which used to define individual `metadata.json` files in each topic's directory
- now we simply store a dictionary of metadata to the `research_inventory.json` file at the root of the research tree.

There is a draft of the `Topic` struct and the containing `Inventory` HashMap in the `lib/src/metadata/topic.rs` file.
