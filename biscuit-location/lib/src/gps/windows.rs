use std::time::Duration;

use crate::types::Location;

pub async fn current_fix(_timeout: Duration) -> crate::Result<Option<Location>> {
    Ok(None)
}
