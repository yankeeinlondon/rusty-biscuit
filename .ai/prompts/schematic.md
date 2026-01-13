# Schematic Package

We recently implemented a meta-programming approach to defining API's and then generating a user friendly enumeration that callers can used.

Unfortunately the approach which was take was VERY different from what was intended. You can still reference some of the outputs by looking at the files in the `./schematic` directory of this monorepo.

This planning and execution task though is largely a full REDO.

## Package Structure

The `schematic` package contains two sub-packages:

1. The **Definition Library** [`./define`]

     This is where all of the API's and SchemaObject's will be defined.

2. The **Schema** Library** [`./schema`]

    During the _generation_ phase the definitions we've created in the Definition Library will be transformed into usable/callable symbols in the Schema Library.


## The High Level Goal

### Definition Stage

In this repo we want to be able to define API's by specifying:

- Their approach to **Authentication**/**Authorization** (this should always be uniform across the whole API surface)
- The **Base URL** where all API endpoints stem off of
- Some basic **metadata**:
    - `name` of the API
    - Description of the API
    - URL for API documentation
    - etc.
- And then we must define every **endpoint** defined on the API surface. An endpoint has:
    - a **method** (GET, PUT, POST, ...)
    - a **path** (aka, the relative path off of the base URL)
    - an expected **type** for:
        - Request
        - and Response
    - an **id** which will be used during the _generation_ phase to create a new variant of an enumeration



### Generation Stage

The definition stage is where all the schema defining work is done. Once we're ready to "publish" we will need a rust program -- leveraging the `syn` crate amongst others -- to transform these definitions into a highly type save enumeration.

- this transformation will be convert a `RestApi` (the definition) into a `Api` struct and a uniquely named enumeration. For each `RestApi` definition in the Generation package we will _generate_ (in the Schema Library):

    1. a _public instance_ of the `Api` struct named after Api (PascalCase of API `name`)

        - this will mirror the API definitions high-level metadata (e.g., name, description, URLs, etc.)
        - it will also implement a `request(enum)` method where the specific enum for the API will be the only parameter

    2. a _bespoke_ public enumeration of the `Api` which has a 1:1 number of variants to the Api definition's endpoints. Each of these variants will require a strongly typed **request** object.
- in addition to transforming the `RestApi` and `Endpoint`'s, this Rust script will need to copy over the `SchemaObject`'s defined so that our API's Request and Response have type grounding.



## Key Symbols

These symbols aren't meant to final but represent a good starting point.

### Core Types

```rust
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use strum_macros::{Display, EnumIter, EnumString};

// Re-export strum for convenience
pub use strum;

/// ----------------------------------------------------------------------
/// 1. Schema Definition
/// ----------------------------------------------------------------------

/// The trait that all Request and Response bodies must implement.
/// This ensures they are Serde-compatible and provide necessary metadata
/// for the generation phase.
pub trait SchemaObject: Serialize + DeserializeOwned + Debug + Clone + Send + Sync + 'static {
    /// Returns the PascalCase name of the type (e.g., "Model", "CreateUserRequest").
    /// This is used by the generator to import/reference the type.
    fn name() -> &'static str;
}

/// A descriptor that represents a type in our Definition graph.
/// We use this instead of the raw type so we can store it in the heterogenous
/// `RestApi` endpoint list.
#[derive(Debug, Clone)]
pub struct Schema {
    pub name: String,
    // Future expansion: we could store JSON Schema definitions here
    // or TypeIds to allow the generator to inspect the structure.
}

impl Schema {
    /// Helper to create a Schema descriptor from a concrete type.
    pub fn of<T: SchemaObject>() -> Self {
        Self {
            name: T::name().to_string(),
        }
    }
}

/// ----------------------------------------------------------------------
/// 2. API Structure Definition
/// ----------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumIter, EnumString)]
#[strum(serialize_all = "UPPERCASE")]
pub enum RestMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Trace,
}

#[derive(Debug, Clone)]
pub enum AuthStrategy {
    None,
    BearerToken,
    ApiKey { header: String },
    OAuth2,
}

#[derive(Debug, Clone)]
pub enum ApiResponse {
    /// No content returned (HTTP 204)
    None,
    PlainText,
    Csv,
    Json(Schema),
}

/// Defines a single operation on the API.
/// Note: We removed the generic `<F>` here so that RestApi can hold a `Vec<Endpoint>`.
/// The format is now contained within `response: ApiResponse`.
#[derive(Debug, Clone)]
pub struct Endpoint {
    /// PascalCase unique ID (e.g., "ListModels", "CreateCompletion").
    /// Used as the Enum Variant name in the generated code.
    pub id: String,

    pub method: RestMethod,

    /// Path template (e.g., "/models/{model}").
    pub path: String,

    /// The Schema of the Request body (if applicable).
    pub request: Option<Schema>,

    /// The expected format and Schema of the Response.
    pub response: ApiResponse,

    pub description: Option<String>,
}

/// The Container for the entire API Definition.
#[derive(Debug, Clone)]
pub struct RestApi {
    pub name: String,
    pub description: Option<String>,
    pub base_url: String,
    pub doc_url: Option<String>,
    pub auth: AuthStrategy,
    pub endpoints: Vec<Endpoint>,
}
```

### Example Implementation

```rust
use serde::{Deserialize, Serialize};
// In a real scenario, these imports would come from the crate defined above
use schematic_define::{
    ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod, Schema, SchemaObject,
};

// ----------------------------------------------------------------------
// 1. Define Data Types (The "SchemaObjects")
// ----------------------------------------------------------------------

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub object: String,
    pub created: u32,
    pub owned_by: String,
}

// Implementing the trait required by the system
impl SchemaObject for Model {
    fn name() -> &'static str {
        "Model"
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ListModelsResponse {
    pub object: String,
    pub data: Vec<Model>,
}

impl SchemaObject for ListModelsResponse {
    fn name() -> &'static str {
        "ListModelsResponse"
    }
}

// ----------------------------------------------------------------------
// 2. Define the API Structure
// ----------------------------------------------------------------------

pub fn define_openai_api() -> RestApi {
    RestApi {
        name: "OpenAi".to_string(),
        description: Some("The OpenAI REST API".to_string()),
        base_url: "https://api.openai.com/v1".to_string(),
        doc_url: Some("https://platform.openai.com/docs/api-reference".to_string()),
        // Uniform Auth strategy
        auth: AuthStrategy::BearerToken,
        endpoints: vec![
            Endpoint {
                id: "ListModels".to_string(),
                method: RestMethod::Get,
                path: "/models".to_string(),
                description: Some("Lists the currently available models".to_string()),
                // GET request usually implies no body, so None
                request: None,
                // We expect JSON back, specifically the ListModelsResponse struct
                response: ApiResponse::Json(Schema::of::<ListModelsResponse>()),
            },
            Endpoint {
                id: "RetrieveModel".to_string(),
                method: RestMethod::Get,
                path: "/models/{model}".to_string(),
                description: Some("Retrieves a model instance".to_string()),
                request: None,
                response: ApiResponse::Json(Schema::of::<Model>()),
            },
        ],
    }
}

fn main() {
    let api = define_openai_api();
    println!("Defined API: {}", api.name);
    println!("Endpoint count: {}", api.endpoints.len());

    // In the real system, this is where we would pass `api`
    // to the generator logic in the `./schema` library.
}

```


### Generator Logic

```rust
use crate::define::{RestApi, Endpoint, ParamLocation};
use proc_macro2::TokenStream;
use quote::{quote, format_ident};

pub fn generate_api_code(api: &RestApi) -> TokenStream {
    let api_struct_name = format_ident!("{}", api.name);
    let enum_name = format_ident!("{}Request", api.name);

    // We need two lists of tokens:
    // 1. The structs (OpenAiListModelsRequest, etc.)
    // 2. The enum variants (OpenAiRequest::ListModels(OpenAiListModelsRequest))
    let mut generated_structs = Vec::new();
    let mut enum_variants = Vec::new();
    let mut match_arms = Vec::new();

    for endpoint in &api.endpoints {
        let req_struct_name = format_ident!("{}Request", endpoint.id);
        let variant_name = format_ident!("{}", endpoint.id);

        // -------------------------------------------------------
        // A. Build the Request Struct Fields
        // -------------------------------------------------------
        let mut struct_fields = Vec::new();
        let mut default_field_assignments = Vec::new(); // For impl Default
        let mut path_construction_logic = Vec::new();   // For into_parts logic

        // 1. Initialize path string
        let raw_path = &endpoint.path;
        path_construction_logic.push(quote! {
            let mut final_path = #raw_path.to_string();
        });

        // 2. Process Parameters (Path/Query)
        for param in &endpoint.params {
            let p_ident = format_ident!("{}", param.name);
            let p_name_str = &param.name;

            // Define the struct field
            struct_fields.push(quote! { pub #p_ident: String });

            // Define the Default value
            if let Some(def_val) = &param.default {
                default_field_assignments.push(quote! {
                    #p_ident: #def_val.to_string()
                });
            } else {
                // If no default is provided, Default::default() usually returns empty string for String.
                // Or we can panic/force user to not use default if it's mandatory.
                // For this example, we default to empty string.
                default_field_assignments.push(quote! {
                    #p_ident: String::new()
                });
            }

            // Define logic to inject this into the Path
            if param.location == ParamLocation::Path {
                path_construction_logic.push(quote! {
                    final_path = final_path.replace(
                        &format!("{{{}}}", #p_name_str),
                        &self.#p_ident
                    );
                });
            }
        }

        // 3. Process Body (if POST/PUT)
        let body_logic;
        if let Some(req_schema) = &endpoint.request {
            let body_type = format_ident!("{}", req_schema.name);

            // Add body field to struct
            struct_fields.push(quote! { pub body: #body_type });

            // Just use default for the body type itself
            default_field_assignments.push(quote! { body: #body_type::default() });

            body_logic = quote! { Some(serde_json::to_value(&self.body).unwrap()) };
        } else {
            body_logic = quote! { None };
        }

        // -------------------------------------------------------
        // B. Generate the Struct TokenStream
        // -------------------------------------------------------
        generated_structs.push(quote! {
            #[derive(Debug, Clone, PartialEq)]
            pub struct #req_struct_name {
                #(#struct_fields),*
            }

            impl Default for #req_struct_name {
                fn default() -> Self {
                    Self {
                        #(#default_field_assignments),*
                    }
                }
            }
        });

        // -------------------------------------------------------
        // C. Generate the Enum Variant & Match Arm
        // -------------------------------------------------------
        enum_variants.push(quote! {
            #variant_name(#req_struct_name)
        });

        let method_str = endpoint.method.to_string();

        match_arms.push(quote! {
            #enum_name::#variant_name(req) => {
                let (path, body) = req.resolve_components();
                (#method_str, path, body)
            }
        });

        // We also add a helper method to the Struct itself to resolve path/body
        generated_structs.push(quote! {
            impl #req_struct_name {
                pub fn resolve_components(&self) -> (String, Option<serde_json::Value>) {
                    #(#path_construction_logic)*
                    (final_path, #body_logic)
                }
            }
        });
    }

    // -------------------------------------------------------
    // D. Final Assembly
    // -------------------------------------------------------
    quote! {
        /// Generated API Client
        pub struct #api_struct_name;

        // ... impl #api_struct_name ...

        // The Request Structs
        #(#generated_structs)*

        // The Unified Enum
        #[derive(Debug, Clone)]
        pub enum #enum_name {
            #(#enum_variants),*
        }

        impl #enum_name {
            pub fn into_parts(self) -> (&'static str, String, Option<serde_json::Value>) {
                match self {
                    #(#match_arms),*
                }
            }
        }

        // Utility: Allow the Struct to turn into the Enum automatically
        #(
            impl From<#generated_structs> for #enum_name {
                fn from(r: #generated_structs) -> Self {
                    #enum_name::#enum_variants(r)
                }
            }
        )*
    }
}
```

#### Resulting Code (from Generation)

```rust
/// Generated API Client for: OpenAi
/// The OpenAI REST API
pub struct OpenAi;

impl OpenAi {
    pub const BASE_URL: &'static str = "https://api.openai.com/v1";

    pub fn request(req: OpenAiRequest) {
        let (method, path, body) = req.into_parts();
        println!("Sending {} to {}{} with body: {:?}", method, Self::BASE_URL, path, body);
    }
}

#[derive(Debug, Clone)]
pub enum OpenAiRequest {
    // Generated from: id: "ListModels", request: None
    ListModels,
    // Generated from: id: "RetrieveModel", request: None (Note: see improvements below regarding params)
    RetrieveModel
}

impl OpenAiRequest {
    pub fn into_parts(self) -> (&'static str, String, Option<serde_json::Value>) {
        match self {
            OpenAiRequest::ListModels => {
                ("GET", "/models".to_string(), None)
            },
            OpenAiRequest::RetrieveModel => {
                ("GET", "/models/{model}".to_string(), None)
            }
        }
    }
}
```

If we define an endpoint `/models/{model}` where model defaults to "":


```rust
// --- Generated Output ---

// 1. A dedicated struct for this specific endpoint's request
#[derive(Debug, Clone, PartialEq)]
pub struct RetrieveModelRequest {
    // We map path params here.
    // If it has a default, we can make it String and use impl Default
    // OR make it Option<String> and handle logic in conversion.
    pub model: String,
}

// 2. We implement Default so the user can just call RetrieveModelRequest::default()
impl Default for RetrieveModelRequest {
    fn default() -> Self {
        Self {
            model: "".to_string() // The default value from the API definition
        }
    }
}

// 3. The Main API Enum now just wraps these structs
#[derive(Debug, Clone)]
pub enum OpenAiRequest {
    RetrieveModel(RetrieveModelRequest),
    // other endpoints...
}
```

## Developer Experience

### Using the Default

```rust
// The user wants to list models.
// The default for 'model' is "" (all models).
let req = OpenAiRequest::RetrieveModel(RetrieveModelRequest::default());

// OR using the From impl
let req: OpenAiRequest = RetrieveModelRequest::default().into();

// Result path: "/models/"
```


