use super::super::*;
use schematic_define::{DefinitionContext, DefinitionError};
use schematic_define::{Endpoint, RestMethod};

pub fn all(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        Endpoint::new("CreateSoundEffect", RestMethod::Post, "/v1/sound-generation")
            .describes("Generates a sound effect from text")
            .request_json::<CreateSoundEffectBody>(ctx)
            .returns_binary()
            .build()?,
        Endpoint::new("ListModels", RestMethod::Get, "/v1/models")
            .describes("Lists all available models")
            .returns_json::<Vec<ModelInfo>>(ctx)
            .build()?,
        Endpoint::new("CreateSingleUseToken", RestMethod::Post, "/v1/single-use-token/{token_type}")
            .describes("Creates a single-use token for WebSocket auth")
            .returns_json::<SingleUseTokenResponse>(ctx)
            .build()?,
    ])

}
