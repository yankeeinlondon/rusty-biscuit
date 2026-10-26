use super::super::*;
use schematic_define::{DefinitionContext, DefinitionError};
use schematic_define::{Endpoint, RestMethod};

/// Monitoring endpoints (nodes, cluster, listeners, metrics, stats, alarms, prometheus).
pub fn monitoring_endpoints(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        // Node & Cluster
        Endpoint::new("ListNodes", RestMethod::Get, "/nodes")
            .describes("List all nodes in the EMQX cluster")
            .returns_json::<ListNodesResponse>(ctx)
            .build()?,
        Endpoint::new("GetNode", RestMethod::Get, "/nodes/{node}")
            .describes("Get detailed information about a specific node")
            .returns_json::<NodeInfo>(ctx)
            .build()?,
        Endpoint::new("GetCluster", RestMethod::Get, "/cluster")
            .describes("Get cluster status and node membership")
            .returns_json::<ClusterStatus>(ctx)
            .build()?,
        // Listeners
        Endpoint::new("ListListeners", RestMethod::Get, "/listeners")
            .describes("List all configured listeners")
            .returns_json::<ListListenersResponse>(ctx)
            .build()?,
        Endpoint::new("GetListener", RestMethod::Get, "/listeners/{id}")
            .describes("Get details of a specific listener")
            .returns_json::<ListenerInfo>(ctx)
            .build()?,
        // Metrics & Stats
        Endpoint::new("ListMetrics", RestMethod::Get, "/metrics")
            .describes("Get broker metrics for all nodes")
            .returns_json::<ListMetricsResponse>(ctx)
            .build()?,
        Endpoint::new("ListStats", RestMethod::Get, "/stats")
            .describes("Get broker statistics for all nodes")
            .returns_json::<ListStatsResponse>(ctx)
            .build()?,
        Endpoint::new("GetPrometheus", RestMethod::Get, "/prometheus/stats")
            .describes("Get metrics in Prometheus format")
            .returns_text()
            .build()?,
        // Alarms
        Endpoint::new("ListAlarms", RestMethod::Get, "/alarms")
            .describes("List active alarms")
            .returns_json::<ListAlarmsResponse>(ctx)
            .pagination(schematic_define::pagination::Collection::at("data").item_type("AlarmInfo").unpaginated())
            .build()?,
    ])

}
