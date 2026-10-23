use super::super::*;
use schematic_define::{DefinitionContext, DefinitionError};
use schematic_define::{Endpoint, RestMethod, Success};

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
            // No media type is documented for the audio stream (the research page's
            // `text/event-stream` label is not SSE evidence), so none is declared.
            .returns(Success::bytes())
            .build()?,
        Endpoint::new("CreateSpeechWithTimestamps", RestMethod::Post, "/v1/text-to-speech/{voice_id}/with-timestamps")
            .describes("Returns audio with character-level timing information")
            .request_json::<CreateSpeechBody>(ctx)
            .returns_json::<SpeechWithTimestampsResponse>(ctx)
            .build()?,
        Endpoint::new("StreamSpeechWithTimestamps", RestMethod::Post, "/v1/text-to-speech/{voice_id}/stream/with-timestamps")
            .describes("Streams audio chunks with timing information")
            .request_json::<CreateSpeechBody>(ctx)
            // The reference says only "a stream of JSONs"; line framing comes from the
            // official Python SDK, which reads it with `iter_lines()` + `json.loads`.
            .returns(Success::ndjson(ctx.register::<SpeechWithTimestampsResponse>()?))
            .build()?,
    ])

}
