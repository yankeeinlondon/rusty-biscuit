mod auth_endpoints;
mod banned_endpoints;
mod client_endpoints;
mod messaging_endpoints;
mod monitoring_endpoints;
mod rules_endpoints;

pub use auth_endpoints::auth_endpoints;
pub use banned_endpoints::banned_endpoints;
pub use client_endpoints::client_endpoints;
pub use messaging_endpoints::messaging_endpoints;
pub use monitoring_endpoints::monitoring_endpoints;
pub use rules_endpoints::rules_endpoints;
