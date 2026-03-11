pub mod attachment;
pub mod capabilities;
pub mod dispatch;
pub mod error;
pub mod markdown;
pub mod message;
pub mod prelude;
#[doc(hidden)]
pub mod prepared;
pub mod provider;
pub mod receipt;
pub mod target;
pub mod validate;

pub use attachment::{Attachment, AttachmentKind, AttachmentSource};
pub use capabilities::CapabilitySet;
pub use dispatch::{CompatibilityMode, DeliveryOptions, Dispatch, ProviderOverrides};
pub use error::MessengerError;
pub use message::{Location, Message, MessageBody};
#[doc(hidden)]
pub use prepared::PreparedMessage;
pub use provider::{Messenger, Provider, SendPlan};
pub use receipt::{MessageRef, ProviderKind, SendReceipt};
pub use target::Target;
pub use validate::{CompatibilityWarning, normalize_dispatch, validate_dispatch, validate_message};

#[cfg(test)]
mod tests;
