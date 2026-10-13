mod outer {
    pub mod inner {
        pub use crate::a::B;
    }
    pub(crate) use super::C;
}
