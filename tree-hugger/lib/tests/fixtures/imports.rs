// Test fixture for Rust import extraction

// Simple use
use std::io;

// Use with path
use std::collections::HashMap;

// Use with multiple items
use std::process::{Child, Command, Stdio};

// Aliased use
use std::io::Read as IoRead;

// Glob import (not captured by current query)
// use std::prelude::*;

// External crate
extern crate serde;

fn main() {
    let _: HashMap<String, String> = HashMap::new();
}
