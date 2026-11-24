#![forbid(unsafe_code)]

pub mod metadata;
pub mod scans;
pub mod tables;

pub use metadata::{Dependency, Metadata, Package, check_metadata, check_metadata_value};
pub use scans::{env_usage_violations, package_of_path, scan_exemption_attrs};
pub use tables::{
    allowed_internal_deps, disallowed_app_deps, forbidden_external_for_domain,
    packages_allowed_exemptions,
};
