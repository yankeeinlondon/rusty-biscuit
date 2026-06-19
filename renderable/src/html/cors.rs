pub enum HttpCrossOrigin {
    /// Request uses CORS headers and credentials flag is set to 'same-origin'. There is no exchange of user
    /// credentials via cookies, client-side TLS certificates or HTTP authentication, unless destination is
    /// the same origin.
    Anonymous,
    /// Request uses CORS headers, credentials flag is set to 'include' and user credentials are always included.
    UseCredentials,
    /// Setting the attribute name to an empty value, like crossorigin or crossorigin="", is the same as anonymous.
    Empty,
}
