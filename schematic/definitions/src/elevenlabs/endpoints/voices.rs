use super::super::*;
use schematic_define::{DefinitionContext, DefinitionError};
use schematic_define::{
    Endpoint, FormField, RestMethod,
    params::{EndpointParams, QueryParamType},
};

pub fn all(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        Endpoint::new("ListVoices", RestMethod::Get, "/v2/voices")
            .describes("Lists all available voices")
            .returns_json::<ListVoicesResponse>(ctx)
            .params(EndpointParams::default().with_query_params(vec![
                ("page_size", QueryParamType::Integer, false, Some("Voices per page (at most 100)")),
                (
                    "page_token",
                    QueryParamType::String,
                    false,
                    Some("The previous page's `next_page_token`"),
                ),
            ]))
            .pagination(super::super::voices_collection())
            .build()?,
        Endpoint::new("GetVoice", RestMethod::Get, "/v1/voices/{voice_id}")
            .describes("Retrieves a voice by ID")
            .returns_json::<VoiceResponseModel>(ctx)
            .build()?,
        Endpoint::new("DeleteVoice", RestMethod::Delete, "/v1/voices/{voice_id}")
            .describes("Deletes a voice")
            .returns_json::<StatusResponse>(ctx)
            .build()?,
        Endpoint::new("GetDefaultVoiceSettings", RestMethod::Get, "/v1/voices/settings/default")
            .describes("Gets default voice settings")
            .returns_json::<VoiceSettings>(ctx)
            .build()?,
        Endpoint::new("GetVoiceSettings", RestMethod::Get, "/v1/voices/{voice_id}/settings")
            .describes("Gets voice settings for a specific voice")
            .returns_json::<VoiceSettings>(ctx)
            .build()?,
        Endpoint::new("UpdateVoiceSettings", RestMethod::Post, "/v1/voices/{voice_id}/settings/edit")
            .describes("Updates voice settings")
            .request_json::<VoiceSettings>(ctx)
            .returns_json::<StatusResponse>(ctx)
            .build()?,
        Endpoint::new("GetVoiceSampleAudio", RestMethod::Get, "/v1/voices/{voice_id}/samples/{sample_id}/audio")
            .describes("Gets audio for a voice sample")
            .returns_binary()
            .build()?,
        Endpoint::new("DeleteVoiceSample", RestMethod::Delete, "/v1/voices/{voice_id}/samples/{sample_id}")
            .describes("Deletes a voice sample")
            .returns_json::<StatusResponse>(ctx)
            .build()?,
        Endpoint::new("AddVoiceSample", RestMethod::Post, "/v1/voices/{voice_id}/samples")
            .describes("Upload audio sample for voice cloning")
            .request_form(vec![
                FormField::file_accept("audio", vec!["audio/*".into()])
                    .with_description("Audio file (mp3, wav, ogg, m4a)"),
                FormField::text("name")
                    .optional()
                    .with_description("Name for the sample"),
            ])
            .returns_json::<AddSampleResponse>(ctx)
            .build()?,
        Endpoint::new("ListSharedVoices", RestMethod::Get, "/v1/shared-voices")
            .describes("Lists voices from the public library")
            .returns_json::<ListSharedVoicesResponse>(ctx)
            .pagination(super::super::shared_voices_collection())
            .build()?,
        Endpoint::new("AddSharedVoice", RestMethod::Post, "/v1/voices/add/{public_user_id}/{voice_id}")
            .describes("Adds a shared voice to your library")
            .request_json::<AddSharedVoiceBody>(ctx)
            .returns_json::<AddSharedVoiceResponse>(ctx)
            .build()?,
        Endpoint::new("CreatePvcVoice", RestMethod::Post, "/v1/voices/pvc")
            .describes("Creates a professional voice clone")
            .request_json::<CreatePvcVoiceBody>(ctx)
            .returns_json::<AddSharedVoiceResponse>(ctx)
            .build()?,
        Endpoint::new("UpdatePvcVoice", RestMethod::Post, "/v1/voices/pvc/{voice_id}")
            .describes("Updates a PVC voice")
            .request_json::<CreatePvcVoiceBody>(ctx)
            .returns_json::<StatusResponse>(ctx)
            .build()?,
        Endpoint::new("TrainPvcVoice", RestMethod::Post, "/v1/voices/pvc/{voice_id}/train")
            .describes("Starts training a PVC voice")
            .request_json::<TrainPvcVoiceBody>(ctx)
            .returns_json::<StatusResponse>(ctx)
            .build()?,
    ])

}
