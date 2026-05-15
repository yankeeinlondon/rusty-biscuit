pub enum FetchPriority {
    /// Fetch the external script at a high priority relative to other external scripts.
    High,
    /// Fetch the external script at a low priority relative to other external scripts.
    Low,
    /// Don't set a preference for the fetch priority. This is the default. It is used if no
    /// value or an invalid value is set.
    Auto,
}

pub enum ReferrerPolicy {
    NoReferrer,
    Origin,
    OriginWhenCrossOrigin,
    SameOrigin,
    StrictOrigin,
    StrictOriginWhenCrossOrigin,
    UnsafeUrl,
}

pub enum ScriptTagType {
    /// This value causes the code to be treated as a JavaScript module. The processing of the script contents
    /// is deferred. The charset and defer attributes have no effect. For information on using module, see our
    /// JavaScript modules guide. Unlike classic scripts, module scripts require the use of the CORS protocol
    /// for cross-origin fetching.
    Module,
    ImportMap,
    SpeculationRules,
    /// The embedded content is treated as a data block, and won't be processed by the browser. Developers
    /// must use a valid MIME type that is not a JavaScript MIME type to denote data blocks. All of the other
    /// attributes will be ignored, including the src attribute.
    Other(String),
}

pub struct ScriptTag {
    /// **async**
    ///
    /// For classic scripts, if the async attribute is present, then the classic script will be fetched in parallel
    /// to parsing and evaluated as soon as it is available.
    /// For module scripts, if the async attribute is present then the scripts and all their dependencies will be
    /// fetched in parallel to parsing and evaluated as soon as they are available.
    ///
    /// > Note: the property name is `async` but that is a Rust keyword so we couldn't use it
    asynchronous: bool,
    /// Normal script elements pass minimal information to the window.onerror for scripts which do not pass the
    /// standard CORS checks. To allow error logging for sites which use a separate domain for static media, use
    /// this attribute. See CORS settings attributes for a more descriptive explanation of its valid arguments.
    crossorigin: Option<String>,
    /// Boolean attribute is set to indicate to a browser that the script is meant to be
    /// executed after the document has been parsed, but before firing DOMContentLoaded event.
    defer: bool,
    /// Provides a hint of the relative priority to use when fetching an external script.
    fetchpriority: Option<FetchPriority>,
    /// This attribute contains one or more hashes of the script. It is used to ensure that the content of the
    /// script is what the developer expects it to be, and has not been replaced with a malicious script in a
    /// supply chain attack. The attribute must not be specified when the src attribute is absent. See also
    /// [Subresource Integrity](https://developer.mozilla.org/en-US/docs/Web/Security/Defenses/Subresource_Integrity).
    integrity: Option<String>,
    /// This Boolean attribute is set to indicate that the script should not be executed in browsers that support ES
    /// modules — in effect, this can be used to serve fallback scripts to older browsers that do not support modular
    /// JavaScript code.
    nomodule: bool,
    /// A cryptographic nonce (number used once) to allow scripts in a script-src Content-Security-Policy. The server must
    /// generate a unique nonce value each time it transmits a policy. It is critical to provide a nonce that cannot be
    /// guessed as bypassing a resource's policy is otherwise trivial.
    nonce: Option<u32>,
    /// Indicates which referrer to send when fetching the script, or resources fetched by the script
    referrerpolicy: Option<ReferrerPolicy>,
    /// This attribute specifies the URI of an external script; this can be used as an alternative to embedding a
    /// script directly within a document.
    src: Option<String>,
    /// This attribute indicates the type of script represented;
    /// the name of the property is **type** not **kind** but **type**
    /// is a reserved word. `None` indicates that the script is a "classic script" type.
    kind: ScriptTagType,
}
