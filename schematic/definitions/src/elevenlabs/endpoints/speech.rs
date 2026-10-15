use super::super::*;
use schematic_define::{DefinitionContext, DefinitionError};
use schematic_define::{Endpoint, RestMethod};

pub fn all(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        Endpoint::new("CreateSpeech", RestMethod::Post, "/v1/text-to-speech/{voice_id}")
            .describes("Converts text into speech and returns audio")
            .request_json::<CreateSpeechBody>(ctx)
            .returns_binary()
            .build()?,
        Endpoint::new("StreamSpeech", RestMethod::Post, "/v1/text-to-speech/{voice_id}/stream")
            .describes("Streams audio as it's generated")
            .request_json::<CreateSpeechBody>(ctx)
            .returns_binary()
            .build()?,
        Endpoint::new("CreateSpeechWithTimestamps", RestMethod::Post, "/v1/text-to-speech/{voice_id}/with-timestamps")
            .describes("Returns audio with character-level timing information")
            .request_json::<CreateSpeechBody>(ctx)
            .returns_json::<SpeechWithTimestampsResponse>(ctx)
            .build()?,
        Endpoint::new("StreamSpeechWithTimestamps", RestMethod::Post, "/v1/text-to-speech/{voice_id}/stream/with-timestamps")
            .describes("Streams audio chunks with timing information")
            .request_json::<CreateSpeechBody>(ctx)
            .returns_json::<SpeechWithTimestampsResponse>(ctx)
            .build()?,
    ])

}
