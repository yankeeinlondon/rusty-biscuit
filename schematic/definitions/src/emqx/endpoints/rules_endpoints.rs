use super::super::*;
use schematic_define::{DefinitionContext, DefinitionError};
use schematic_define::{Endpoint, RestMethod};

/// Rules engine endpoints (list, create, get, update, delete, test).
pub fn rules_endpoints(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        Endpoint::new("ListRules", RestMethod::Get, "/rules")
            .describes("List all rules in the rules engine")
            .returns_json::<ListRulesResponse>(ctx)
            .pagination(schematic_define::pagination::Collection::at("data").item_type("RuleInfo").unpaginated())
            .build()?,
        Endpoint::new("CreateRule", RestMethod::Post, "/rules")
            .describes("Create a new rule in the rules engine")
            .request_json::<CreateRuleBody>(ctx)
            .returns_json::<RuleInfo>(ctx)
            .build()?,
        Endpoint::new("GetRule", RestMethod::Get, "/rules/{id}")
            .describes("Get details of a specific rule")
            .returns_json::<RuleInfo>(ctx)
            .build()?,
        Endpoint::new("UpdateRule", RestMethod::Put, "/rules/{id}")
            .describes("Update an existing rule")
            .request_json::<CreateRuleBody>(ctx)
            .returns_json::<RuleInfo>(ctx)
            .build()?,
        Endpoint::new("DeleteRule", RestMethod::Delete, "/rules/{id}")
            .describes("Delete a rule from the rules engine")
            .returns_empty()
            .build()?,
        Endpoint::new("TestRule", RestMethod::Post, "/rules/{id}/test")
            .describes("Test a rule with sample data")
            .request_json::<TestRuleBody>(ctx)
            .returns_json::<TestRuleResponse>(ctx)
            .build()?,
    ])

}
