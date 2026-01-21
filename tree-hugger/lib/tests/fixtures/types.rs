/// A tuple struct representing a 2D point.
pub struct Point(i32, i32);

/// A generic container for any type.
pub struct Container<T> {
    /// The value stored in the container.
    value: T,
}

/// Message types for the application.
pub enum Message {
    /// Quit the application.
    Quit,
    /// Write a string message.
    Write(String),
    /// Move to a position.
    Move { x: i32, y: i32 },
    /// Change the color with RGB values.
    ChangeColor(u8, u8, u8),
}

/// A result type with custom error.
pub enum Result<T, E> {
    /// Success value.
    Ok(T),
    /// Error value.
    Err(E),
}
