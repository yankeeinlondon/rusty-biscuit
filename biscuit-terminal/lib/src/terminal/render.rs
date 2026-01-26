
/// **link** function create an OSC8 link for the terminal
///
/// > **Note:** use the `auto_link` function if you want the link to be
/// generated only when the terminal supports OSC8 and provides a good
/// fallback if it doesn't.
pub fn link<T: Into<String>, U: Into<String>>(content: T, link: U) -> String {
    todo!()
}


pub fn clear_screen() {
    todo!()
}

/// Strips all escape codes out of the passed in string.
pub fn strip_escape_codes<T: Into<String>>(content: T) -> String{
    todo!()
}



