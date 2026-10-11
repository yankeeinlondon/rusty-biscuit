//! Encoding B (compared, not chosen): knobs as type-level flags on one pager,
//! `TPager<Item, Kind, Size, Resume, Back>`, with a per-endpoint type alias
//! and a per-endpoint runtime config. Surface only; no network. Kept so the
//! compile-fail messages and per-endpoint code can be compared with
//! encoding A.

use std::marker::PhantomData;

use crate::error::PageSizeError;

pub struct Yes;
pub struct No;
pub struct PageNumberK;
pub struct CursorK;

/// Const generics cannot carry `&'static str` on stable, so names and limits
/// cannot live in the type: each endpoint also needs a runtime config value,
/// and nothing ties the flags to the config.
pub struct Config {
    pub size_param: Option<&'static str>,
    pub size_max: Option<u32>,
}

pub struct TPager<T, K, Size, Resume, Back> {
    config: &'static Config,
    size: Option<u32>,
    page: Option<u64>,
    cursor: Option<String>,
    _flags: PhantomData<fn() -> (T, K, Size, Resume, Back)>,
}

impl<T, K, S, R, B> TPager<T, K, S, R, B> {
    pub fn new(config: &'static Config) -> Self {
        Self {
            config,
            size: None,
            page: None,
            cursor: None,
            _flags: PhantomData,
        }
    }
    pub fn describe(&self) -> (Option<u32>, Option<u64>, Option<&str>) {
        (self.size, self.page, self.cursor.as_deref())
    }
}

impl<T, K, R, B> TPager<T, K, Yes, R, B> {
    pub fn page_size(mut self, n: u32) -> Result<Self, PageSizeError> {
        // The flag says a size exists; the config must agree, or this is a
        // runtime surprise the type system cannot catch.
        let max = self.config.size_max;
        if n == 0 || max.is_some_and(|m| n > m) {
            return Err(PageSizeError { requested: n, min: 1, max });
        }
        self.size = Some(n);
        Ok(self)
    }
}

impl<T, S, R, B> TPager<T, PageNumberK, S, R, B> {
    pub fn page(mut self, n: u64) -> Self {
        self.page = Some(n);
        self
    }
}

impl<T, S, B> TPager<T, CursorK, S, Yes, B> {
    pub fn after(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }
}

impl<T, S> TPager<T, CursorK, S, Yes, Yes> {
    pub fn before(mut self, cursor: impl Into<String>) -> Self {
        self.cursor = Some(cursor.into());
        self
    }
}

// --- per endpoint (generated) ------------------------------------------------

pub type ListIssuesB = TPager<crate::generated::Item, PageNumberK, Yes, No, No>;
pub static LIST_ISSUES_B: Config = Config { size_param: Some("per_page"), size_max: Some(100) };

pub type ListRolesB = TPager<crate::generated::Item, CursorK, No, No, No>;
pub static LIST_ROLES_B: Config = Config { size_param: None, size_max: None };

pub fn list_issues_b() -> ListIssuesB {
    TPager::new(&LIST_ISSUES_B)
}
pub fn list_roles_b() -> ListRolesB {
    TPager::new(&LIST_ROLES_B)
}
