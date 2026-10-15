use super::super::*;
use schematic_define::{DefinitionContext, DefinitionError};
use schematic_define::{Endpoint, RestMethod};

pub fn all(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        Endpoint::new("GetHistory", RestMethod::Get, "/v1/history")
            .describes("Gets generated items history")
            .returns_json::<GetHistoryResponse>(ctx)
            .pagination(super::super::history_collection())
            .build()?,
        Endpoint::new("GetHistoryItem", RestMethod::Get, "/v1/history/{history_item_id}")
            .describes("Gets a specific history item")
            .returns_json::<SpeechHistoryItemResponseModel>(ctx)
            .build()?,
        Endpoint::new("DeleteHistoryItem", RestMethod::Delete, "/v1/history/{history_item_id}")
            .describes("Deletes a history item")
            .returns_json::<StatusResponse>(ctx)
            .build()?,
        Endpoint::new("GetHistoryItemAudio", RestMethod::Get, "/v1/history/{history_item_id}/audio")
            .describes("Gets audio for a history item")
            .returns_binary()
            .build()?,
        Endpoint::new("DownloadHistoryItems", RestMethod::Post, "/v1/history/download")
            .describes("Downloads multiple history items as ZIP")
            .request_json::<DownloadHistoryBody>(ctx)
            .returns_binary()
            .build()?,
    ])

}
