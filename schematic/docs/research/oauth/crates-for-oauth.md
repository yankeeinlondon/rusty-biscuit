---
prompt: |-
    Research the various Rust crates which help in working with OAuth authentication. For each crate:

    1. List the name, docs URL, repo URL
    2. Description of the goals for this crate
    3. A Rust code example of how you might use this crate to implement OAuth authentication
    4. The feature flags the crate offers and when to use each flag (and when not to)
    5. Any common gotchas that developer report hitting when working with this crate along with ways in which these obstacles can be overcome.

    After review all of the crates, suggest which of these crates you feel would be most likely to fit well with the goals of the schematic ecosystem. In your recommendation, be sure to explain WHY you are recommending this crate over the others.

    Your final deliverable is a Markdown document (written to the body of this page) which well formed and idiomatic Markdown. Tables are Markdown tables. Links are Markdown links.  

    If you want visualize an idea then using a Mermaid block is the best way to do that.
last_updated: 2026-03-10
---
# Rust OAuth Crates: A Comprehensive Analysis

This document provides an in-depth analysis of the various Rust crates available for implementing OAuth authentication. Each crate is evaluated based on its goals, usage patterns, feature flags, and common challenges developers face.

---

## Table of Contents

1. [Overview](#overview)
2. [oauth2 (oauth2-rs)](#oauth2-oauth2-rs)
3. [openidconnect](#openidconnect)
4. [oxide-auth](#oxide-auth)
5. [yup-oauth2](#yup-oauth2)
6. [yew-oauth2](#yew-oauth2)
7. [async-oauth2](#async-oauth2)
8. [oauth (Universal Adapter)](#oauth-universal-adapter)
9. [arctic-oauth](#arctic-oauth)
10. [Comparison Matrix](#comparison-matrix)
11. [Recommendation for the Schematic Ecosystem](#recommendation-for-the-schematic-ecosystem)

---

## Overview

The Rust ecosystem offers several OAuth crates, each targeting different use cases:

```mermaid
graph TD
    A[OAuth Crates] --> B[Client Libraries]
    A --> C[Server Libraries]
    A --> D[Framework-Specific]
    
    B --> B1[oauth2-rs]
    B --> B2[openidconnect]
    B --> B3[yup-oauth2]
    B --> B4[async-oauth2]
    B --> B5[arctic-oauth]
    
    C --> C1[oxide-auth]
    
    D --> D1[yew-oauth2]
    D --> D2[oauth - Universal Adapter]
```

---

## oauth2 (oauth2-rs)

### Basic Information

| Property           | Value                                          |
| ------------------ | ---------------------------------------------- |
| **Name**           | `oauth2`                                       |
| **Docs URL**       | https://docs.rs/oauth2/latest/oauth2           |
| **Repo URL**       | https://github.com/ramosbugs/oauth2-rs         |
| **Latest Version** | 5.0.0+                                         |
| **Downloads**      | Most popular OAuth crate in the Rust ecosystem |

### Goals and Description

The `oauth2` crate provides an **extensible, strongly-typed implementation of OAuth2 (RFC 6749)**, including support for token introspection (RFC 7662) and token revocation (RFC 7009). It is designed to be a client-side library for consuming OAuth2 services, with a focus on type safety and correctness. The crate aims to express OAuth2 protocol requirements through Rust's type system, preventing many common implementation mistakes at compile time rather than runtime.

The library supports multiple OAuth2 flows including Authorization Code Grant (with and without PKCE), Implicit Grant, Resource Owner Password Credentials Grant, and Client Credentials Grant. It is designed to be HTTP-client agnostic, allowing developers to choose between reqwest, curl, or custom implementations.

### Code Example

```rust
use oauth2::{
    AuthorizationCode,
    AuthUrl,
    ClientId,
    ClientSecret,
    CsrfToken,
    PkceCodeChallenge,
    RedirectUrl,
    Scope,
    TokenUrl,
};
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create an OAuth2 client
    let client = BasicClient::new(
        ClientId::new("client_id".to_string()),
        Some(ClientSecret::new("client_secret".to_string())),
        AuthUrl::new("https://authorization-server.com/auth".to_string())?,
        Some(TokenUrl::new("https://authorization-server.com/token".to_string())?)
    )
    .set_redirect_uri(RedirectUrl::new("https://localhost:8080/callback".to_string())?);

    // Generate a PKCE challenge
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the authorization URL
    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("read".to_string()))
        .add_scope(Scope::new("write".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    println!("Browse to: {}", auth_url);

    // After the user authorizes, exchange the code for a token
    // (This would typically be done in a callback handler)
    let code = AuthorizationCode::new("authorization_code".to_string());
    
    let token_result = client
        .exchange_code(code)
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await?;

    println!("Access Token: {:?}", token_result.access_token());
    
    Ok(())
}
```

### Feature Flags

| Feature Flag                     | Default | When to Use                                                  | When Not to Use                                              |
| -------------------------------- | ------- | ------------------------------------------------------------ | ------------------------------------------------------------ |
| `reqwest`                        | Yes     | For async HTTP operations using the reqwest client; most common choice for async applications | When you need a different HTTP client or want to minimize dependencies |
| `rustls-tls`                     | Yes     | For TLS support using rustls (pure Rust TLS implementation)  | When you need native TLS or have specific TLS requirements   |
| `curl`                           | No      | For synchronous operations or when reqwest is not suitable; useful in environments where reqwest has issues | In async contexts where blocking is problematic              |
| `native-tls`                     | No      | For TLS support using the system's native TLS implementation | When you prefer rustls for security or cross-platform consistency |
| `timing-resistant-secret-traits` | No      | For applications requiring constant-time secret comparisons (security-critical) | When performance is more critical than timing attack resistance |

### Common Gotchas and Solutions

1. **Blocking HTTP Client in Async Context**
   - **Problem**: Using `reqwest::blocking::Client` within async Rust code can cause panics or deadlocks.
   - **Solution**: Always use the async HTTP client (`async_http_client`) in async contexts. If you must use blocking code, wrap it with `tokio::task::spawn_blocking`.

2. **Redirect URI Mismatch**
   - **Problem**: The redirect URI used during authorization must exactly match the one used during token exchange, causing `redirect_uri_mismatch` errors.
   - **Solution**: Store the redirect URI configuration centrally and reuse it consistently. Ensure trailing slashes and query parameters match exactly.

3. **Feature Flag Confusion with reqwest**
   - **Problem**: The default `reqwest` feature only enables the async client, not the blocking client.
   - **Solution**: Add `reqwest` with the `blocking` feature to your Cargo.toml if you need synchronous operations: `reqwest = { version = "...", features = ["blocking"] }`.

4. **Custom Token Fields**
   - **Problem**: OAuth2 providers often return additional fields in token responses that the crate doesn't parse by default.
   - **Solution**: Implement a custom `TokenResponse` or use the `extra_fields` mechanism to capture additional data.

5. **SSRF Vulnerabilities**
   - **Problem**: The HTTP client may follow redirects, potentially leading to Server-Side Request Forgery.
   - **Solution**: Configure the HTTP client not to follow redirects when making token requests.

---

## openidconnect

### Basic Information

| Property           | Value                                         |
| ------------------ | --------------------------------------------- |
| **Name**           | `openidconnect`                               |
| **Docs URL**       | https://docs.rs/openidconnect                 |
| **Repo URL**       | https://github.com/ramosbugs/openidconnect-rs |
| **Latest Version** | 4.0.0+                                        |
| **Dependencies**   | Built on top of `oauth2` crate                |

### Goals and Description

The `openidconnect` crate provides **extensible, strongly-typed interfaces for the OpenID Connect protocol**, which builds upon OAuth2 for authentication. It can be used to authenticate users via providers like Google, GitLab, Okta, Keycloak, and any other OpenID Connect compliant identity provider.

This crate is specifically designed for **authentication** (single sign-on, social login) rather than just authorization. It handles OpenID Connect specific features like ID token verification, discovery documents, and userinfo endpoints. The library expresses the protocol's security requirements within Rust's type system to prevent common security mistakes.

### Code Example

```rust
use openidconnect::{
    AccessTokenHash,
    AuthenticationFlow,
    AuthorizationCode,
    ClientId,
    ClientSecret,
    CsrfToken,
    IssuerUrl,
    Nonce,
    OAuth2TokenResponse,
    PkceCodeChallenge,
    RedirectUrl,
    Scope,
    TokenResponse,
};
use openidconnect::core::{CoreClient, CoreProviderMetadata, CoreResponseType};
use openidconnect::reqwest::async_http_client;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Discover OpenID Connect provider metadata
    let provider_metadata = CoreProviderMetadata::discover(
        IssuerUrl::new("https://accounts.google.com".to_string())?,
        async_http_client,
    ).await?;

    // Create an OpenID Connect client
    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        ClientId::new("client_id".to_string()),
        Some(ClientSecret::new("client_secret".to_string())),
    )
    .set_redirect_uri(RedirectUrl::new("https://localhost:8080/callback".to_string())?);

    // Generate a PKCE challenge
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    // Generate the authorization URL
    let (auth_url, csrf_token, nonce) = client
        .authorize_url(
            AuthenticationFlow::<CoreResponseType>::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("openid".to_string()))
        .add_scope(Scope::new("email".to_string()))
        .add_scope(Scope::new("profile".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    println!("Browse to: {}", auth_url);

    // After callback, exchange the code for tokens
    let code = AuthorizationCode::new("authorization_code".to_string());
    
    let token_response = client
        .exchange_code(code)
        .set_pkce_verifier(pkce_verifier)
        .request_async(async_http_client)
        .await?;

    // Verify the ID token
    let id_token = token_response.id_token().unwrap();
    let claims = id_token.claims(&client.id_token_verifier(), &nonce)?;
    
    println!("User ID: {:?}", claims.subject());
    println!("Email: {:?}", claims.email());
    
    // Verify access token hash if present
    if let Some(expected_access_token_hash) = claims.access_token_hash() {
        let actual_hash = AccessTokenHash::from_token(
            token_response.access_token(),
            id_token.signing_alg()?,
        )?;
        if actual_hash != *expected_access_token_hash {
            return Err("Access token hash mismatch".into());
        }
    }

    Ok(())
}
```

### Feature Flags

| Feature Flag                     | Default | When to Use                                              | When Not to Use                                              |
| -------------------------------- | ------- | -------------------------------------------------------- | ------------------------------------------------------------ |
| `reqwest`                        | Yes     | For async HTTP operations                                | When using a different HTTP client                           |
| `rustls-tls`                     | Yes     | For TLS with rustls                                      | When native TLS is preferred                                 |
| `curl`                           | No      | For synchronous operations                               | In async contexts                                            |
| `native-tls`                     | No      | For native TLS                                           | When rustls is preferred                                     |
| `timing-resistant-secret-traits` | No      | For constant-time comparisons                            | When performance is critical                                 |
| `jwk-alg`                        | No      | To use the `alg` field from JWKs for algorithm selection | When strict algorithm verification is needed (non-default due to breaking changes) |

### Common Gotchas and Solutions

1. **Token Response Type Confusion**
   - **Problem**: Both `oauth2::TokenResponse` and `openidconnect::TokenResponse` exist, causing confusion about which to import.
   - **Solution**: Use fully qualified paths or import only the one you need. The `openidconnect` version extends the `oauth2` version with ID token support.

2. **Nonce Verification Failure**
   - **Problem**: The nonce generated during authorization must be stored and used for ID token verification.
   - **Solution**: Store the nonce in a session or state management system and retrieve it during the callback. The `claims()` method requires the same nonce used during authorization.

3. **Discovery Document Caching**
   - **Problem**: Provider metadata discovery adds network latency on every startup.
   - **Solution**: Cache the provider metadata or use `CoreClient::from_provider_metadata` with pre-fetched metadata.

4. **Custom Claims**
   - **Problem**: Some providers return custom claims not defined in the standard claims struct.
   - **Solution**: Implement custom claims by extending the standard claims struct with `AdditionalClaims`.

5. **RSA Timing Attack Vulnerability**
   - **Problem**: The `rsa` crate had a timing attack vulnerability (RUSTSEC-2023-0071).
   - **Solution**: Enable the `timing-resistant-secret-traits` feature flag or update to the latest version where this is addressed.

---

## oxide-auth

### Basic Information

| Property           | Value                                        |
| ------------------ | -------------------------------------------- |
| **Name**           | `oxide-auth`                                 |
| **Docs URL**       | https://docs.rs/oxide-auth/latest/oxide_auth |
| **Repo URL**       | https://github.com/197g/oxide-auth           |
| **Latest Version** | 0.5.x                                        |
| **Type**           | OAuth2 Server Library                        |

### Goals and Description

`oxide-auth` is a **server-side OAuth2 library** designed for implementing an OAuth2 provider/authorization server. It aims to provide a comprehensive and extensible interface to managing OAuth2 tokens on a server, featuring a set of configurable and pluggable backends.

The core package is **agnostic of the HTTP library used**, making it compatible with actix, rocket, axum, iron, and rouille through extension crates. This separation allows the OAuth logic to remain independent of the web framework, making it highly portable and testable.

The crate supports the Authorization Code Grant flow with extensions like PKCE, and provides primitives for token storage, client registration, and scope management.

### Code Example

```rust
use oxide_auth::endpoint::AuthorizationFlow;
use oxide_auth::frontends::simple::endpoint::{FnSolicitor, Generic, Vacant};
use oxide_auth::primitives::prelude::*;
use std::sync::Mutex;
use std::collections::HashMap;

// Create a simple in-memory registrar (client storage)
struct SimpleRegistrar {
    clients: HashMap<String, Client>,
}

impl Registrar for SimpleRegistrar {
    fn bound_redirect<'a>(&'a self, bound: &'a Client) -> Result<BoundRedirect<'a>, RegistrarError> {
        // Validate and bind the redirect URI
        Ok(BoundRedirect::from(bound.redirect_uri()))
    }

    fn negotiate(&self, client_id: &str, _scope: Option<&Scope>) -> Result<Client, RegistrarError> {
        self.clients
            .get(client_id)
            .cloned()
            .ok_or(RegistrarError::UnregisteredClient)
    }
}

// Create a simple authorizer (code storage)
struct SimpleAuthorizer {
    codes: Mutex<HashMap<String, Grant>>,
}

impl Authorizer for SimpleAuthorizer {
    fn negotiate(&self, grant: Grant) -> Result<String, ()> {
        let code = generate_random_string();
        self.codes.lock().unwrap().insert(code.clone(), grant);
        Ok(code)
    }

    fn extract(&self, code: &str) -> Result<Option<Grant>, ()> {
        Ok(self.codes.lock().unwrap().remove(code))
    }
}

// Create a simple issuer (token storage)
struct SimpleIssuer {
    tokens: Mutex<HashMap<String, Grant>>,
}

impl Issuer for SimpleIssuer {
    fn issue(&self, grant: Grant) -> Result<String, ()> {
        let token = generate_random_string();
        self.tokens.lock().unwrap().insert(token.clone(), grant);
        Ok(token)
    }

    fn recover_token(&self, token: &str) -> Result<Option<Grant>, ()> {
        Ok(self.tokens.lock().unwrap().get(token).cloned())
    }
}

fn generate_random_string() -> String {
    // Use a proper random string generator in production
    uuid::Uuid::new_v4().to_string()
}

// Example with actix-web integration
#[cfg(feature = "actix")]
mod actix_example {
    use oxide_auth_actix::{OAuth, OAuthRequest, OAuthResponse};
    use oxide_auth::endpoint::AuthorizationFlow;
    
    pub async fn authorize(
        (request, oauth): (OAuthRequest, OAuth),
    ) -> Result<OAuthResponse, ()> {
        oauth.authorization_flow()
            .execute(request)
            .map_err(|_| ())
    }
}
```

### Feature Flags

| Feature Flag                       | Default     | When to Use                          | When Not to Use             |
| ---------------------------------- | ----------- | ------------------------------------ | --------------------------- |
| Default features                   | No defaults | When building a minimal OAuth server | N/A                         |
| `actix` (via `oxide-auth-actix`)   | No          | When using the actix-web framework   | When using other frameworks |
| `axum` (via `oxide-auth-axum`)     | No          | When using the axum framework        | When using other frameworks |
| `rocket` (via `oxide-auth-rocket`) | No          | When using the Rocket framework      | When using other frameworks |

**Note**: The feature flags are primarily handled through separate extension crates rather than feature flags within the main crate.

### Common Gotchas and Solutions

1. **Framework Integration Complexity**
   - **Problem**: Integrating with a specific web framework requires understanding both the framework's request/response model and oxide-auth's primitives.
   - **Solution**: Use the extension crates (`oxide-auth-actix`, `oxide-auth-axum`, `oxide-auth-rocket`) which provide pre-built integrations and examples.

2. **In-Memory Storage Limitations**
   - **Problem**: The simple in-memory backends are not suitable for production (data lost on restart, not distributed).
   - **Solution**: Implement custom `Registrar`, `Authorizer`, and `Issuer` traits backed by a database like PostgreSQL or Redis.

3. **Missing Documentation and Examples**
   - **Problem**: The crate has limited tutorials and real-world examples.
   - **Solution**: Refer to the examples in the GitHub repository and community resources like blog posts. The test suite also provides useful patterns.

4. **Grant Expiration Handling**
   - **Problem**: Authorization codes and tokens need proper expiration handling.
   - **Solution**: Implement expiration checks in your storage backend and use the `Grant` struct's timestamp fields.

5. **Scope Management Complexity**
   - **Problem**: Implementing proper scope restrictions can be complex.
   - **Solution**: Use the `Scope` primitive and implement a policy module that maps users/clients to allowed scopes.

---

## yup-oauth2

### Basic Information

| Property             | Value                                   |
| -------------------- | --------------------------------------- |
| **Name**             | `yup-oauth2`                            |
| **Docs URL**         | https://docs.rs/yup-oauth2              |
| **Repo URL**         | https://github.com/dermesser/yup-oauth2 |
| **Latest Version**   | 12.x                                    |
| **Primary Use Case** | Google API authentication               |

### Goals and Description

`yup-oauth2` is a utility library implementing several OAuth 2.0 flows, **primarily designed for Google API authentication**. It's mainly used by the `google-apis-rs` project to authenticate against Google services, but may work with other providers that support similar flows.

The crate specializes in flows needed for service accounts, installed applications, and device flows—scenarios particularly relevant for CLI tools, background services, and applications running on devices with limited input capabilities.

### Code Example

```rust
use yup_oauth2::{
    AccessToken,
    ApplicationSecret,
    Authenticator,
    InstalledFlowReturnMethod,
    ServiceAccountAuthenticator,
    ServiceAccountKey,
};
use yup_oauth2::authenticator::DefaultHyperClient;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Service Account Flow (for server-side applications)
    let service_account_key = ServiceAccountKey {
        key_type: Some("service_account".to_string()),
        project_id: Some("my-project".to_string()),
        private_key_id: Some("key-id".to_string()),
        private_key: "-----BEGIN PRIVATE KEY-----\n...".to_string(),
        client_email: "my-service@my-project.iam.gserviceaccount.com".to_string(),
        client_id: Some("123456789".to_string()),
        auth_uri: Some("https://accounts.google.com/o/oauth2/auth".to_string()),
        token_uri: Some("https://oauth2.googleapis.com/token".to_string()),
        auth_provider_x509_cert_url: None,
        client_x509_cert_url: None,
    };

    let auth = ServiceAccountAuthenticator::builder(service_account_key)
        .build()
        .await?;

    let scopes = &["https://www.googleapis.com/auth/drive.readonly"];
    let token = auth.token(scopes).await?;
    println!("Access Token: {:?}", token.token());

    // Example 2: Installed Application Flow (for desktop/CLI apps)
    let secret = ApplicationSecret {
        client_id: "client_id.apps.googleusercontent.com".to_string(),
        client_secret: "client_secret".to_string(),
        token_uri: "https://oauth2.googleapis.com/token".to_string(),
        auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
        redirect_uris: Some(vec!["urn:ietf:wg:oauth:2.0:oob".to_string()]),
        project_id: Some("my-project".to_string()),
        client_email: None,
        auth_provider_x509_cert_url: None,
        client_x509_cert_url: None,
        auth_x509_cert_url: None,
    };

    let installed_auth = Authenticator::new(
        secret,
        InstalledFlowReturnMethod::HTTPRedirect,
        DefaultHyperClient.with_client(hyper::Client::new()),
        None,
        None,
    );

    let scopes = &["https://www.googleapis.com/auth/gmail.readonly"];
    let token = installed_auth.token(scopes).await?;
    println!("Gmail Access Token: {:?}", token.token());

    Ok(())
}
```

### Feature Flags

| Feature Flag      | Default | When to Use                                  | When Not to Use                          |
| ----------------- | ------- | -------------------------------------------- | ---------------------------------------- |
| `hyper-rustls`    | Yes     | For TLS using rustls (pure Rust)             | When native TLS is required              |
| `ring`            | Yes     | For cryptographic operations                 | When using alternative crypto libraries  |
| `service-account` | Yes     | For service account authentication flows     | When only using user authentication      |
| `rustls-pemfile`  | Yes     | For reading PEM-encoded service account keys | When using other key formats             |
| `aws-lc-rs`       | No      | For AWS LC crypto backend                    | When using ring or other crypto backends |
| `hyper-tls`       | No      | For TLS using native TLS implementation      | When rustls is preferred                 |

### Common Gotchas and Solutions

1. **Google-Specific Implementation**
   - **Problem**: The crate is optimized for Google's OAuth implementation and may not work correctly with other providers.
   - **Solution**: For non-Google providers, use the more generic `oauth2` or `openidconnect` crates instead.

2. **Scope Configuration**
   - **Problem**: Setting incorrect or missing scopes results in authentication failures or limited API access.
   - **Solution**: Carefully review Google API documentation for required scopes and use the exact scope URLs.

3. **Token Storage**
   - **Problem**: By default, tokens are not persisted across application restarts.
   - **Solution**: Implement a custom `TokenStorage` to persist tokens to disk or a database.

4. **Service Account Impersonation**
   - **Problem**: Service accounts can impersonate users, but this requires additional configuration.
   - **Solution**: Use the `with_subject()` method to specify the user email to impersonate.

---

## yew-oauth2

### Basic Information

| Property           | Value                               |
| ------------------ | ----------------------------------- |
| **Name**           | `yew-oauth2`                        |
| **Docs URL**       | https://docs.rs/yew-oauth2          |
| **Repo URL**       | https://github.com/ctron/yew-oauth2 |
| **Latest Version** | 0.7.x                               |
| **Framework**      | Yew (WebAssembly)                   |

### Goals and Description

`yew-oauth2` provides **Yew components for implementing OAuth2 and OpenID Connect login flows** in WebAssembly applications. It is designed to work seamlessly with the Yew framework, offering pre-built components that handle the complexity of OAuth flows in browser environments.

The crate supports both plain OAuth2 and OpenID Connect, with OIDC providing additional features like logout URLs, discovery, and user information. It is framework-agnostic on the backend side, meaning it can work with any OAuth2/OIDC provider.

### Code Example

```rust
use yew::prelude::*;
use yew_oauth2::prelude::*;
use yew_oauth2::oauth2::{OAuth2, OAuth2Config};
use yew_oauth2::openid::{OpenId, OpenIdConfig};
use yew_oauth2::components::{AuthProvider, PrivateRoute, LoginRedirectReason};
use yew_router::prelude::*;

// Define your routes
#[derive Routable Clone)]
enum AppRoute {
    #[at("/")]
    Home,
    #[at("/login")]
    Login,
    #[at("/callback")]
    Callback,
    #[at("/protected")]
    Protected,
}

// Configure OAuth2
fn create_oauth2_config() -> OAuth2Config {
    OAuth2Config {
        client_id: "my-client-id".to_string(),
        client_secret: Some("my-client-secret".to_string()),
        auth_url: "https://auth.example.com/auth".to_string(),
        token_url: "https://auth.example.com/token".to_string(),
        redirect_url: "https://myapp.example.com/callback".to_string(),
        scopes: vec!["openid".to_string(), "profile".to_string()],
        ..Default::default()
    }
}

// Main App component with OAuth2
#[function_component(App)]
fn app() -> Html {
    let config = create_oauth2_config();
    
    html! {
        <OAuth2 {config}>
            <BrowserRouter>
                <Switch<AppRoute> render={switch} />
            </BrowserRouter>
        </OAuth2>
    }
}

// Protected route component
#[function_component(ProtectedPage)]
fn protected_page() -> Html {
    let auth = use_context::<OAuth2>().unwrap();
    
    html! {
        <div>
            <h1>{ "Protected Content" }</h1>
            <p>{ "You are logged in!" }</p>
            <button onclick={move |_| auth.logout()}>
                { "Logout" }
            </button>
        </div>
    }
}

// Login page component
#[function_component(LoginPage)]
fn login_page() -> Html {
    let auth = use_context::<OAuth2>().unwrap();
    
    html! {
        <div>
            <h1>{ "Please Login" }</h1>
            <button onclick={move |_| auth.start_login()}>
                { "Login with OAuth2" }
            </button>
        </div>
    }
}

fn switch(route: AppRoute) -> Html {
    match route {
        AppRoute::Home => html! { <h1>{ "Welcome" }</h1> },
        AppRoute::Login => html! { <LoginPage /> },
        AppRoute::Callback => html! { <Callback /> },
        AppRoute::Protected => html! {
            <PrivateRoute>
                <ProtectedPage />
            </PrivateRoute>
        },
    }
}

// Callback component for handling OAuth redirect
#[function_component(Callback)]
fn callback() -> Html {
    let auth = use_context::<OAuth2>().unwrap();
    let navigator = use_navigator().unwrap();
    
    use_effect_with((), move |_| {
        if let Ok(()) = auth.handle_callback() {
            navigator.push(&AppRoute::Protected);
        }
        || ()
    });
    
    html! { <p>{ "Processing login..." }</p> }
}
```

### Feature Flags

| Feature Flag | Default | When to Use                                                  | When Not to Use                  |
| ------------ | ------- | ------------------------------------------------------------ | -------------------------------- |
| `openid`     | No      | When using OpenID Connect (provides discovery, logout, userinfo) | When only plain OAuth2 is needed |
| `router`     | No      | When integrating with yew-router for route protection        | When not using yew-router        |

### Common Gotchas and Solutions

1. **CORS Issues**
   - **Problem**: Browser-based OAuth flows often encounter CORS restrictions.
   - **Solution**: Ensure your OAuth provider is configured to accept requests from your application's origin. Use the Authorization Code flow with PKCE instead of implicit flow.

2. **Token Storage in Browser**
   - **Problem**: Tokens stored in memory are lost on page refresh.
   - **Solution**: The crate automatically handles localStorage for token persistence. Ensure your application handles the `on_token_change` callback to sync with your state.

3. **Redirect URL Configuration**
   - **Problem**: The redirect URL must exactly match what's configured in your OAuth provider.
   - **Solution**: Include the full path including `/callback` in your OAuth provider's allowed redirect URIs.

---

## async-oauth2

### Basic Information

| Property           | Value                                            |
| ------------------ | ------------------------------------------------ |
| **Name**           | `async-oauth2`                                   |
| **Docs URL**       | https://docs.rs/async-oauth2                     |
| **Repo URL**       | https://github.com/ramosbugs/oauth2-rs (related) |
| **Latest Version** | 0.5.x                                            |

### Goals and Description

`async-oauth2` provides an **asynchronous OAuth2 flow implementation**, trying to adhere as much as possible to RFC 6749. It is designed for async/await-based applications and provides native async support for all OAuth2 operations.

This crate serves as an alternative to the main `oauth2` crate when async-first design is preferred. It aims to provide cleaner async ergonomics without the complexity of choosing between sync and async features.

### Code Example

```rust
use async_oauth2::{
    AuthorizationCode,
    Client,
    ClientId,
    ClientSecret,
    CodeAuthorizationRequest,
    Scope,
    TokenResponse,
};
use http::Uri;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(
        ClientId::new("my-client-id".to_string()),
        Some(ClientSecret::new("my-client-secret".to_string())),
        "https://auth.example.com/auth".parse()?,
        Some("https://auth.example.com/token".parse()?),
    );

    // Generate authorization URL
    let auth_url = client
        .authorize_url()?
        .redirect_uri("https://myapp.example.com/callback".parse()?)
        .scope(Scope::new("read write".to_string()))
        .url();

    println!("Browse to: {}", auth_url);

    // Exchange authorization code for token
    let token = client
        .exchange_code(AuthorizationCode::new("auth-code".to_string()))
        .redirect_uri("https://myapp.example.com/callback".parse()?)
        .request()
        .await?;

    println!("Access Token: {:?}", token.access_token());
    println!("Refresh Token: {:?}", token.refresh_token());

    Ok(())
}
```

### Feature Flags

| Feature Flag     | Default | When to Use                      | When Not to Use                     |
| ---------------- | ------- | -------------------------------- | ----------------------------------- |
| Default features | Yes     | Standard async OAuth2 operations | N/A                                 |
| `surf`           | No      | When using the surf HTTP client  | When using reqwest or other clients |

### Common Gotchas and Solutions

1. **Less Actively Maintained**
   - **Problem**: The crate may lag behind the main `oauth2` crate in terms of updates and features.
   - **Solution**: Consider using the main `oauth2` crate with async features instead, which receives more regular updates.

2. **Provider Compatibility**
   - **Problem**: Some OAuth providers have non-standard implementations that may not work out of the box.
   - **Solution**: Test thoroughly with your specific provider and be prepared to handle custom fields in responses.

---

## oauth (Universal Adapter)

### Basic Information

| Property           | Value                          |
| ------------------ | ------------------------------ |
| **Name**           | `oauth`                        |
| **Docs URL**       | https://docs.rs/oauth-lib      |
| **Repo URL**       | https://crates.io/crates/oauth |
| **Latest Version** | 0.0.x                          |
| **Type**           | Framework Adapter              |

### Goals and Description

The `oauth` crate (sometimes referred to as `oauth-lib`) is a **universal OAuth 2.0 adapter for Rust web frameworks**, providing a single configuration model and framework-specific glue code. It aims to abstract away the differences between web frameworks, allowing developers to write OAuth logic once and use it across different frameworks.

The crate supports multiple async runtimes and provides a consistent API regardless of the underlying web framework being used.

### Code Example

```rust
use oauth::{OAuth, OAuthConfig, Provider};

// Configure OAuth with a provider
let config = OAuthConfig {
    client_id: "my-client-id".to_string(),
    client_secret: "my-client-secret".to_string(),
    redirect_uri: "https://myapp.example.com/callback".to_string(),
    auth_url: "https://auth.example.com/auth".to_string(),
    token_url: "https://auth.example.com/token".to_string(),
    scopes: vec!["openid".to_string(), "profile".to_string()],
};

let oauth = OAuth::new(config);

// Framework-agnostic usage
async fn handle_login(oauth: &OAuth) -> Result<String, oauth::Error> {
    let auth_url = oauth.authorization_url();
    Ok(auth_url)
}

async fn handle_callback(oauth: &OAuth, code: &str) -> Result<Token, oauth::Error> {
    let token = oauth.exchange_code(code).await?;
    Ok(token)
}
```

### Feature Flags

| Feature Flag | Default | When to Use                        | When Not to Use      |
| ------------ | ------- | ---------------------------------- | -------------------- |
| `tokio`      | Yes     | When using the tokio async runtime | When using async-std |
| `async-std`  | No      | When using the async-std runtime   | When using tokio     |

### Common Gotchas and Solutions

1. **New and Less Battle-Tested**
   - **Problem**: Being relatively new, the crate may have undiscovered issues.
   - **Solution**: Thoroughly test with your specific use case and framework combination before production use.

2. **Limited Framework Support**
   - **Problem**: Not all Rust web frameworks have adapter implementations.
   - **Solution**: Check for framework-specific adapters or implement custom integration using the core abstractions.

---

## arctic-oauth

### Basic Information

| Property           | Value                                    |
| ------------------ | ---------------------------------------- |
| **Name**           | `arctic-oauth`                           |
| **Docs URL**       | https://crates.io/crates/arctic-oauth    |
| **Repo URL**       | https://github.com/pilcrowonpaper/arctic |
| **Latest Version** | 0.x                                      |
| **Providers**      | 64 pre-configured                        |

### Goals and Description

`arctic-oauth` is a collection of **OAuth 2.0 clients for popular providers with 64 pre-configured providers**, each behind its own feature flag. Every provider encodes its production endpoints, HTTP client, and specific configuration, making it easy to integrate with well-known OAuth providers without manually configuring endpoint URLs.

This crate is particularly useful for applications that need to support multiple OAuth providers (social login, enterprise SSO, etc.) without the complexity of managing different configurations for each.

### Code Example

```rust
// Note: Actual API may vary - this is conceptual
use arctic_oauth::{OAuthClient, providers};

// Enable specific provider feature flags in Cargo.toml:
// arctic-oauth = { version = "0.x", features = ["google", "github", "microsoft"] }

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Google OAuth
    let google_client = providers::Google::new(
        "google-client-id",
        "google-client-secret",
        "https://myapp.example.com/callback/google",
    );

    let google_auth_url = google_client.authorization_url(vec!["openid", "email", "profile"]);
    println!("Google Auth URL: {}", google_auth_url);

    // GitHub OAuth
    let github_client = providers::GitHub::new(
        "github-client-id",
        "github-client-secret",
        "https://myapp.example.com/callback/github",
    );

    let github_auth_url = github_client.authorization_url(vec!["user", "repo"]);
    println!("GitHub Auth URL: {}", github_auth_url);

    Ok(())
}
```

### Feature Flags

| Feature Flag                                           | Default | When to Use                        | When Not to Use                                        |
| ------------------------------------------------------ | ------- | ---------------------------------- | ------------------------------------------------------ |
| Provider flags (`google`, `github`, `microsoft`, etc.) | No      | Only enable the providers you need | Don't enable unused providers to minimize compile time |

### Common Gotchas and Solutions

1. **Limited to Authorization Code Flow**
   - **Problem**: Only the authorization code flow is supported.
   - **Solution**: For other flows (client credentials, device flow), use the base `oauth2` crate directly.

2. **Provider Endpoint Updates**
   - **Problem**: If a provider changes their endpoints, the crate may become outdated.
   - **Solution**: Check for crate updates or manually configure using the base `oauth2` crate.

---

## Comparison Matrix

```mermaid
graph LR
    subgraph "Client Libraries"
        OAUTH2[oauth2] --- |Base for| OIDC[openidconnect]
        OAUTH2 --- |Alternative| ASYNC[async-oauth2]
        YUP[yup-oauth2] --- |Google Focused| OAUTH2
        ARCTIC[arctic-oauth] --- |Multi-Provider| OAUTH2
    end
    
    subgraph "Server Libraries"
        OXIDE[oxide-auth] --- OXIDE_ACTIX[oxide-auth-actix]
        OXIDE --- OXIDE_AXUM[oxide-auth-axum]
        OXIDE --- OXIDE_ROCKET[oxide-auth-rocket]
    end
    
    subgraph "Framework Specific"
        YEW[yew-oauth2] --- |WASM/Yew| OAUTH2
        OAUTH_ADAPTER[oauth] --- |Universal Adapter| OAUTH2
    end
```

### Feature Comparison Table

| Crate           | Type    | Async | OIDC | Server | Primary Use Case            |
| --------------- | ------- | ----- | ---- | ------ | --------------------------- |
| `oauth2`        | Client  | ✅     | ❌    | ❌      | General OAuth2 client       |
| `openidconnect` | Client  | ✅     | ✅    | ❌      | Authentication/SSO          |
| `oxide-auth`    | Server  | ✅     | ❌    | ✅      | OAuth2 authorization server |
| `yup-oauth2`    | Client  | ✅     | ❌    | ❌      | Google API authentication   |
| `yew-oauth2`    | Client  | ✅     | ✅    | ❌      | Yew/WASM applications       |
| `async-oauth2`  | Client  | ✅     | ❌    | ❌      | Async-first OAuth2          |
| `oauth`         | Adapter | ✅     | ❌    | ❌      | Framework-agnostic OAuth    |
| `arctic-oauth`  | Client  | ✅     | ❌    | ❌      | Multi-provider OAuth        |

---

## Recommendation for the Schematic Ecosystem

### Understanding the Schematic Ecosystem

The **schematic** crate (from moonrepo) is a **light-weight, macro-based, layered serde configuration and schema library** with built-in support for:

- Merge strategies for configuration values
- Validation rules
- Environment variable integration
- Schema modeling (TypeScript types, JSON schemas)
- Layered configuration loading from multiple sources

The schematic ecosystem emphasizes:

- **Type safety** through Rust's type system
- **Extensibility** through traits and macros
- **Developer experience** with clear error messages
- **Configuration flexibility** with layered loading
- **Schema generation** for cross-language compatibility

### Recommended Crate: `oauth2`

**The `oauth2` crate is the best fit for the schematic ecosystem.**

### Justification

#### 1. **Philosophical Alignment**

Both `oauth2` and schematic share the same core philosophy:

| Aspect                     | schematic                                                   | oauth2                                                       |
| -------------------------- | ----------------------------------------------------------- | ------------------------------------------------------------ |
| **Type Safety**            | Uses Rust's type system to ensure configuration correctness | Uses Rust's type system to ensure OAuth2 protocol correctness |
| **Extensibility**          | Traits and macros for custom configuration sources          | Traits for custom HTTP clients and token handling            |
| **No Runtime Overhead**    | Compile-time validation where possible                      | Compile-time protocol enforcement                            |
| **Explicit over Implicit** | Clear configuration layers                                  | Explicit flow and scope requirements                         |

#### 2. **Complementary Architecture**

```mermaid
graph TB
    subgraph "Schematic Configuration"
        CONFIG[Config Loader] --> OAUTH_CONFIG[OAuth2Config]
        OAUTH_CONFIG --> MERGE[Merge Strategies]
        MERGE --> VALIDATE[Validation Rules]
    end
    
    subgraph "OAuth2 Integration"
        VALIDATE --> CLIENT[OAuth2 Client]
        CLIENT --> FLOWS[OAuth Flows]
        FLOWS --> TOKENS[Token Management]
    end
    
    subgraph "Schema Generation"
        OAUTH_CONFIG --> JSON_SCHEMA[JSON Schema]
        JSON_SCHEMA --> TS_TYPES[TypeScript Types]
    end
```

The `oauth2` crate's design allows seamless integration with schematic's configuration system:

```rust
use schematic::{Config, ConfigLoader, derive::Config};
use oauth2::{AuthUrl, ClientId, ClientSecret, TokenUrl, RedirectUrl};

#[derive(Config)]
struct OAuthConfig {
    #[setting(merge_strategy = "replace")]
    client_id: ClientId,
    
    #[setting(merge_strategy = "replace", secret)]
    client_secret: Option<ClientSecret>,
    
    #[setting(merge_strategy = "replace")]
    auth_url: AuthUrl,
    
    #[setting(merge_strategy = "replace")]
    token_url: TokenUrl,
    
    #[setting(merge_strategy = "replace")]
    redirect_url: RedirectUrl,
    
    #[setting(default = vec!["openid".to_string()])]
    scopes: Vec<String>,
}

// Load OAuth config with schematic
async fn load_oauth_config() -> Result<OAuthConfig, schematic::Error> {
    ConfigLoader::<OAuthConfig>::new()
        .file("oauth.yaml")?
        .env_prefix("OAUTH_")
        .load()
        .await
}
```

#### 3. **Why Not Other Crates?**

| Crate             | Reason for Not Recommending                                  |
| ----------------- | ------------------------------------------------------------ |
| `openidconnect`   | While excellent, it's more specialized (authentication vs authorization). Can be layered on top of `oauth2` when OIDC is needed. |
| `oxide-auth`      | Server-side implementation; schematic is primarily focused on client-side configuration. |
| `yup-oauth2`      | Too Google-specific; doesn't align with schematic's provider-agnostic approach. |
| `yew-oauth2`      | Framework-specific (WASM/Yew); schematic needs to work across contexts. |
| `async-oauth2`    | Less actively maintained; `oauth2` now has excellent async support. |
| `oauth` (adapter) | Too new and less battle-tested than `oauth2`.                |
| `arctic-oauth`    | Good for quick multi-provider setup, but `oauth2` provides more control needed for a configuration-focused ecosystem. |

#### 4. **Practical Integration Benefits**

1. **Configuration Validation**: The `oauth2` crate's use of newtypes (`ClientId`, `AuthUrl`, etc.) naturally integrates with schematic's validation system.

2. **Feature Flag Alignment**: Both crates use feature flags thoughtfully, allowing minimal dependency builds.

3. **Error Handling**: Both crates provide detailed, typed errors that can be composed and presented to users clearly.

4. **Ecosystem Compatibility**: `oauth2` is the foundation that other OAuth crates build upon, ensuring long-term maintenance and ecosystem support.

#### 5. **Potential Integration Pattern**

```rust
use schematic::{Config, ConfigLoader};
use oauth2::basic::BasicClient;

pub struct OAuth2Provider {
    config: OAuthConfig,
    client: BasicClient,
}

impl OAuth2Provider {
    pub async fn from_config() -> Result<Self, Box<dyn std::error::Error>> {
        let config = ConfigLoader::<OAuthConfig>::new()
            .file("oauth.yaml")?
            .env_prefix("OAUTH_")
            .load()
            .await?;
        
        let client = BasicClient::new(
            config.client_id.clone(),
            config.client_secret.clone(),
            config.auth_url.clone(),
            Some(config.token_url.clone()),
        )
        .set_redirect_uri(config.redirect_url.clone());
        
        Ok(Self { config, client })
    }
    
    pub fn authorize_url(&self) -> url::Url {
        self.client
            .authorize_url(oauth2::CsrfToken::new_random)
            .add_scopes(self.config.scopes.iter().map(|s| oauth2::Scope::new(s.clone())))
            .url()
            .0
    }
}
```

### Summary

The `oauth2` crate is the recommended choice for the schematic ecosystem because:

1. **Type-safe design** aligns with schematic's philosophy
2. **Extensible architecture** allows custom integrations
3. **Well-maintained** with active development and wide adoption
4. **Feature-flagged** for minimal builds
5. **Foundation crate** that other OAuth libraries build upon

For applications needing OpenID Connect support, the `openidconnect` crate (built on `oauth2`) can be added as an optional dependency, providing a natural extension path while maintaining the same type-safe, configuration-driven approach.

