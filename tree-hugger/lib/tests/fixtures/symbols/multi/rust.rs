//! Multi-file fixture: Rust.

/// A widget with a size.
pub struct Widget {
    pub size: u32,
}

impl Widget {
    pub fn new(size: u32) -> Self {
        Self { size }
    }

    pub fn grow(&mut self, amount: u32) {
        self.size += amount;
    }
}

pub enum Shape {
    Circle,
    Square,
}

pub const MAX_SIZE: u32 = 10;

pub fn build_widget() -> Widget {
    Widget::new(MAX_SIZE)
}
