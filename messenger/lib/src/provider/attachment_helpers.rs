use std::fs;

use crate::attachment::AttachmentSource;
use crate::error::MessengerError;

pub(crate) fn read_local_or_bytes_attachment(
    source: &AttachmentSource,
    provider_label: &str,
) -> Result<(String, Vec<u8>), MessengerError> {
    match source {
        AttachmentSource::Path(path) => {
            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| {
                    MessengerError::InvalidMessage(format!(
                        "{provider_label} attachment path has no valid filename: {}",
                        path.display()
                    ))
                })?
                .to_owned();
            let bytes = fs::read(path).map_err(|error| {
                MessengerError::InvalidMessage(format!(
                    "{provider_label} attachment path is not readable ({}): {error}",
                    path.display()
                ))
            })?;
            Ok((filename, bytes))
        }
        AttachmentSource::Bytes { filename, data, .. } => Ok((filename.clone(), data.to_vec())),
        AttachmentSource::Url(_) => Err(MessengerError::InvalidMessage(format!(
            "{provider_label} attachments must come from a local path or bytes payload"
        ))),
        AttachmentSource::ProviderFileId(_) => Err(MessengerError::InvalidMessage(format!(
            "{provider_label} does not support provider file ID attachments"
        ))),
    }
}
