//! Native steering discovery: providers' own records of sessions they run,
//! read without contacting the sessions. Each discoverer implements one
//! researched discovery method (see [`super::discovery::NativeDiscoverer`]).

pub mod claude_registry;
