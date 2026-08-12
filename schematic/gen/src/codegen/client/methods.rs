use proc_macro2::TokenStream;
use quote::quote;

/// Generates the shared build_request helper method.
pub(crate) fn generate_build_request_method(
    _struct_name: &proc_macro2::Ident,
    request_enum: &proc_macro2::Ident,
    auth_setup: &TokenStream,
) -> TokenStream {
    quote! {
        /// Builds and sends an HTTP request, returning the raw response plus context.
        ///
        /// This is an internal helper method used by the public request methods.
        /// Returns both the response and the context needed for hook processing.
        async fn build_and_send_request(
            &self,
            request: impl Into<#request_enum>,
        ) -> Result<(reqwest::Response, crate::shared::ResponseContext), SchematicError> {
            let request = request.into();
            let endpoint_id = request.endpoint_id();
            let (method, path, body, endpoint_headers) = request.into_parts()?;
            let url = format!("{}{}", self.base_url, path);

            let mut req_builder = match method {
                "GET" => self.client.get(&url),
                "POST" => self.client.post(&url),
                "PUT" => self.client.put(&url),
                "PATCH" => self.client.patch(&url),
                "DELETE" => self.client.delete(&url),
                "HEAD" => self.client.head(&url),
                "OPTIONS" => self.client.request(reqwest::Method::OPTIONS, &url),
                _ => return Err(SchematicError::UnsupportedMethod(method.to_string())),
            };

            // Resolve authentication and build API-level headers.
            let api_headers = #auth_setup;

            // Merge API-level and endpoint-level headers
            let merged_headers = Self::merge_headers(&api_headers, &endpoint_headers);
            for (key, value) in merged_headers {
                req_builder = req_builder.header(key.as_str(), value.as_str());
            }

            // Each body variant carries its own content type; reqwest sets the
            // header itself for form bodies (multipart needs its boundary).
            req_builder = match body {
                crate::shared::RequestBody::Empty => req_builder,
                crate::shared::RequestBody::Json(json) => req_builder
                    .header("Content-Type", "application/json")
                    .body(json),
                crate::shared::RequestBody::Multipart(parts) => {
                    let mut form = reqwest::multipart::Form::new();
                    for part in parts {
                        form = match part {
                            crate::shared::FormPart::Text { name, value } => {
                                form.text(name, value)
                            }
                            crate::shared::FormPart::Json { name, value } => {
                                let field = reqwest::multipart::Part::text(value)
                                    .mime_str("application/json")
                                    .map_err(|e| {
                                        SchematicError::SerializationError(e.to_string())
                                    })?;
                                form.part(name, field)
                            }
                            crate::shared::FormPart::File { name, file } => {
                                let mut field = reqwest::multipart::Part::bytes(file.bytes)
                                    .file_name(file.file_name);
                                if let Some(mime) = file.mime {
                                    field = field.mime_str(&mime).map_err(|e| {
                                        SchematicError::SerializationError(e.to_string())
                                    })?;
                                }
                                form.part(name, field)
                            }
                        };
                    }
                    req_builder.multipart(form)
                }
                crate::shared::RequestBody::UrlEncoded(pairs) => req_builder.form(&pairs),
                crate::shared::RequestBody::Raw { content_type, bytes } => req_builder
                    .header("Content-Type", content_type)
                    .body(bytes),
            };

            let response = req_builder.send().await?;

            if !response.status().is_success() {
                let status = response.status().as_u16();
                let body_text = response.text().await.unwrap_or_default();
                return Err(SchematicError::ApiError { status, body: body_text });
            }

            // Build response context for hooks
            let status = response.status().as_u16();
            let headers: Vec<(String, String)> = response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            let ctx = crate::shared::ResponseContext::new(
                endpoint_id,
                method,
                path.clone(),
                url,
                status,
                headers,
            );

            Ok((response, ctx))
        }
    }
}

/// Generates the merge_headers helper method.
pub(crate) fn generate_merge_headers_method() -> TokenStream {
    quote! {
        /// Merges API-level and endpoint-level headers.
        ///
        /// Endpoint headers override API headers for matching keys (case-insensitive).
        /// Returns a new Vec with the merged headers.
        fn merge_headers(
            api_headers: &[(String, String)],
            endpoint_headers: &[(String, String)],
        ) -> Vec<(String, String)> {
            let mut result: Vec<(String, String)> = Vec::new();

            // Add API headers that don't have endpoint overrides
            for (api_key, api_value) in api_headers {
                let has_override = endpoint_headers
                    .iter()
                    .any(|(k, _)| k.eq_ignore_ascii_case(api_key));
                if !has_override {
                    result.push((api_key.clone(), api_value.clone()));
                }
            }

            // Add all endpoint headers (they take precedence)
            for (key, value) in endpoint_headers {
                result.push((key.clone(), value.clone()));
            }

            result
        }
    }
}

/// Generates the request<T> method for JSON responses.
pub(crate) fn generate_json_request_method(
    _struct_name: &proc_macro2::Ident,
    request_enum: &proc_macro2::Ident,
) -> TokenStream {
    quote! {
        /// Executes an API request expecting a JSON response.
        ///
        /// Takes any request type that can be converted into the request enum
        /// and returns the deserialized response. If response hooks are configured
        /// via the variant builder, they will be applied.
        ///
        /// ## Hooks
        ///
        /// If configured via `variant()`:
        /// - `pre_response_json` - Transforms JSON before deserialization
        /// - `mutate_response` - Mutates the response after deserialization
        ///
        /// ## Errors
        ///
        /// Returns an error if:
        /// - The HTTP request fails (network error, timeout, etc.)
        /// - The response indicates a non-success status code
        /// - The response body cannot be deserialized as JSON
        /// - A hook returns an error
        #[must_use = "this returns a Future that must be awaited"]
        pub async fn request<T: serde::de::DeserializeOwned + Send + Sync + 'static>(
            &self,
            request: impl Into<#request_enum>,
        ) -> Result<T, SchematicError> {
            let request = request.into();

            // Steer non-JSON endpoints to their dedicated decoder instead of
            // decoding text/binary/empty bodies as JSON.
            match request.response_kind() {
                crate::shared::ResponseKind::Json => {}
                crate::shared::ResponseKind::Text => {
                    return Err(SchematicError::WrongResponseDecoder {
                        endpoint_id: request.endpoint_id(),
                        kind: "text",
                        expected_method: "request_text",
                    });
                }
                crate::shared::ResponseKind::Binary => {
                    return Err(SchematicError::WrongResponseDecoder {
                        endpoint_id: request.endpoint_id(),
                        kind: "binary",
                        expected_method: "request_bytes",
                    });
                }
                crate::shared::ResponseKind::Empty => {
                    return Err(SchematicError::WrongResponseDecoder {
                        endpoint_id: request.endpoint_id(),
                        kind: "empty",
                        expected_method: "request_empty",
                    });
                }
            }

            let (response, ctx) = self.build_and_send_request(request).await?;

            // Check if hooks are configured
            let has_pre_hook = self.variant_hooks.pre_response_json.is_some();
            let has_mutator = self.variant_hooks.response_mutators.contains_key(ctx.endpoint_id);

            if has_pre_hook || has_mutator {
                // Hook path: read bytes, apply pre-hook, deserialize, apply mutator
                let bytes = response.bytes().await?;
                let mut json_value: serde_json::Value = serde_json::from_slice(&bytes)?;

                // Apply pre-response JSON hook if configured
                if let Some(ref hook) = self.variant_hooks.pre_response_json {
                    json_value = hook(&ctx, json_value)?;
                }

                // Deserialize to target type
                let mut result: T = serde_json::from_value(json_value)?;

                // Apply response mutator if registered for this endpoint
                if let Some(mutator) = self.variant_hooks.response_mutators.get(ctx.endpoint_id) {
                    mutator.mutate(&ctx, &mut result)?;
                }

                Ok(result)
            } else {
                // Fast path: no hooks, direct deserialization
                let result = response.json::<T>().await?;
                Ok(result)
            }
        }
    }
}

/// Generates the request_bytes method for binary responses.
pub(crate) fn generate_bytes_request_method(
    _struct_name: &proc_macro2::Ident,
    request_enum: &proc_macro2::Ident,
) -> TokenStream {
    quote! {
        /// Executes an API request expecting a binary response.
        ///
        /// Returns the raw bytes of the response body. Use this for endpoints
        /// that return binary data like audio files, images, or ZIP archives.
        ///
        /// ## Errors
        ///
        /// Returns an error if:
        /// - The HTTP request fails (network error, timeout, etc.)
        /// - The response indicates a non-success status code
        #[must_use = "this returns a Future that must be awaited"]
        pub async fn request_bytes(
            &self,
            request: impl Into<#request_enum>,
        ) -> Result<bytes::Bytes, SchematicError> {
            let (response, _ctx) = self.build_and_send_request(request).await?;
            let bytes = response.bytes().await?;
            Ok(bytes)
        }
    }
}

/// Generates the request_text method for text responses.
pub(crate) fn generate_text_request_method(
    _struct_name: &proc_macro2::Ident,
    request_enum: &proc_macro2::Ident,
) -> TokenStream {
    quote! {
        /// Executes an API request expecting a plain text response.
        ///
        /// Returns the response body as a String.
        ///
        /// ## Errors
        ///
        /// Returns an error if:
        /// - The HTTP request fails (network error, timeout, etc.)
        /// - The response indicates a non-success status code
        #[must_use = "this returns a Future that must be awaited"]
        pub async fn request_text(
            &self,
            request: impl Into<#request_enum>,
        ) -> Result<String, SchematicError> {
            let (response, _ctx) = self.build_and_send_request(request).await?;
            let text = response.text().await?;
            Ok(text)
        }
    }
}

/// Generates the request_empty method for empty responses.
pub(crate) fn generate_empty_request_method(
    _struct_name: &proc_macro2::Ident,
    request_enum: &proc_macro2::Ident,
) -> TokenStream {
    quote! {
        /// Executes an API request expecting no response body.
        ///
        /// Use this for endpoints that return 204 No Content or where
        /// the response body should be ignored.
        ///
        /// ## Errors
        ///
        /// Returns an error if:
        /// - The HTTP request fails (network error, timeout, etc.)
        /// - The response indicates a non-success status code
        #[must_use = "this returns a Future that must be awaited"]
        pub async fn request_empty(
            &self,
            request: impl Into<#request_enum>,
        ) -> Result<(), SchematicError> {
            let (_response, _ctx) = self.build_and_send_request(request).await?;
            Ok(())
        }
    }
}
