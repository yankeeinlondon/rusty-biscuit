use super::super::*;
use schematic_define::{DefinitionContext, DefinitionError};
use schematic_define::{Endpoint, RestMethod};

/// Messaging endpoints (subscriptions, publishing, topics, retained messages).
pub fn messaging_endpoints(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        // Subscriptions
        Endpoint::new("ListSubscriptions", RestMethod::Get, "/subscriptions")
            .describes("List all subscriptions across the cluster")
            .returns_json::<ListSubscriptionsResponse>(ctx)
            .pagination(schematic_define::pagination::Collection::at("data").item_type("SubscriptionInfo").unpaginated())
            .build()?,
        // Publishing
        Endpoint::new("Publish", RestMethod::Post, "/publish")
            .describes("Publish an MQTT message to a topic")
            .request_json::<PublishBody>(ctx)
            .returns_empty()
            .build()?,
        Endpoint::new("PublishBulk", RestMethod::Post, "/publish/bulk")
            .describes("Publish multiple MQTT messages in a single request")
            .request_json::<PublishBatchBody>(ctx)
            .returns_empty()
            .build()?,
        // Topics
        Endpoint::new("ListTopics", RestMethod::Get, "/topics")
            .describes("List active topics in the broker")
            .returns_json::<ListTopicsResponse>(ctx)
            .pagination(schematic_define::pagination::Collection::at("data").item_type("TopicInfo").unpaginated())
            .build()?,
        // Retained Messages
        Endpoint::new("ListRetained", RestMethod::Get, "/retainer/messages")
            .describes("List retained messages with pagination")
            .returns_json::<ListRetainedResponse>(ctx)
            .pagination(schematic_define::pagination::Collection::at("data").item_type("RetainedMessage").unpaginated())
            .build()?,
        Endpoint::new("GetRetained", RestMethod::Get, "/retainer/messages/{topic}")
            .describes("Get a specific retained message by topic")
            .returns_json::<RetainedMessage>(ctx)
            .build()?,
        Endpoint::new("DeleteRetained", RestMethod::Delete, "/retainer/messages/{topic}")
            .describes("Delete a retained message")
            .returns_empty()
            .build()?,
    ])

}
