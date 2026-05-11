use schematic_define::{ApiRequest, ApiResponse, Endpoint, RestMethod};

pub fn all() -> Vec<Endpoint> {
    vec![
        Endpoint {
            id: "CreateSpeech".to_string(),
            method: RestMethod::Post,
            path: "/v1/text-to-speech/{voice_id}".to_string(),
            description: "Converts text into speech and returns audio".to_string(),
            request: Some(ApiRequest::json_type("CreateSpeechBody")),
            response: ApiResponse::Binary,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "StreamSpeech".to_string(),
            method: RestMethod::Post,
            path: "/v1/text-to-speech/{voice_id}/stream".to_string(),
            description: "Streams audio as it's generated".to_string(),
            request: Some(ApiRequest::json_type("CreateSpeechBody")),
            response: ApiResponse::Binary,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "CreateSpeechWithTimestamps".to_string(),
            method: RestMethod::Post,
            path: "/v1/text-to-speech/{voice_id}/with-timestamps".to_string(),
            description: "Returns audio with character-level timing information".to_string(),
            request: Some(ApiRequest::json_type("CreateSpeechBody")),
            response: ApiResponse::json_type("SpeechWithTimestampsResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "StreamSpeechWithTimestamps".to_string(),
            method: RestMethod::Post,
            path: "/v1/text-to-speech/{voice_id}/stream/with-timestamps".to_string(),
            description: "Streams audio chunks with timing information".to_string(),
            request: Some(ApiRequest::json_type("CreateSpeechBody")),
            response: ApiResponse::json_type("SpeechWithTimestampsResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ]
}
