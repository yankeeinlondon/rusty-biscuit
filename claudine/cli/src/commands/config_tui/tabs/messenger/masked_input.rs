/// Bullet character used to mask a secret input buffer in the modal.
const MASK_BULLET: char = '●';

/// Render an input buffer for display, masking each character when the field
/// is marked secret. The mask preserves character count so the user has a
/// visual cue of how much they've typed without exposing the contents.
pub fn render_buffer(buffer: &str, is_secret: bool) -> String {
    if is_secret {
        MASK_BULLET.to_string().repeat(buffer.chars().count())
    } else {
        buffer.to_string()
    }
}
