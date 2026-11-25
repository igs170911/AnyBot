#[path = "catalog_error.rs"]
mod error;
#[path = "catalog_format.rs"]
mod format;
#[path = "catalog_store.rs"]
mod store;

pub use error::MessagesError;
pub use store::Catalog;

pub const EN_FTL: &str = include_str!("../../../locales/en.ftl");
pub const ZH_TW_FTL: &str = include_str!("../../../locales/zh-TW.ftl");

pub trait MessageCodeLike {
    fn as_str(&self) -> &str;
}
