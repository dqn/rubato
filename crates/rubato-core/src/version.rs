use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Version utility struct (mirrors Java's static Version class)
pub struct Version;

impl Version {
    pub fn get_version() -> &'static str {
        version()
    }

    pub fn long_version() -> &'static str {
        version_long()
    }

    pub fn compare_to_string(other: Option<&str>) -> i32 {
        compare_to_string(other)
    }

    pub fn git_commit_hash() -> Option<&'static str> {
        git_commit_hash()
    }
}

pub const VERSION_MAJOR: i32 = 0;
pub const VERSION_MINOR: i32 = 5;
pub const VERSION_PATCH: i32 = 0;

pub static BUILD_TYPE: BuildType = BuildType::Prerelease;

static VERSION: OnceLock<String> = OnceLock::new();
static UNQUALIFIED_VERSION: OnceLock<String> = OnceLock::new();
static VERSION_LONG: OnceLock<String> = OnceLock::new();
static BUILD_META_INFO: OnceLock<HashMap<String, String>> = OnceLock::new();

pub fn unqualified_version() -> &'static str {
    UNQUALIFIED_VERSION
        .get_or_init(|| format!("{}.{}.{}", VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH))
}

pub fn version() -> &'static str {
    VERSION.get_or_init(|| format!("{}{}", BUILD_TYPE.prefix(), unqualified_version()))
}

pub fn version_long() -> &'static str {
    VERSION_LONG.get_or_init(|| {
        let pre = if BUILD_TYPE.prefix().is_empty() {
            ""
        } else {
            "pre-release "
        };
        format!("rubato {}{}", pre, unqualified_version())
    })
}

fn version_string_to_int_array(version_string: &str) -> Vec<i32> {
    version_string
        .split('.')
        .filter_map(|s| s.parse::<i32>().ok())
        .collect()
}

pub fn compare_to_string(other: Option<&str>) -> i32 {
    let other = match other {
        None => return 1,
        Some(s) => s,
    };
    if other.is_empty() {
        return 1;
    }
    // Defend against version string that is malformed but long enough to pass the null and blank check
    if other.len() < 3 {
        return 1;
    }

    let other_prerelease;
    let other_major;
    let other_minor;
    let other_patch;

    // check for pre-release
    if let Some(rest) = other.strip_prefix("pre") {
        let version_parts = version_string_to_int_array(rest);
        // If the other version string is malformed (too few parts), this static version trumps it
        if version_parts.len() != 3 {
            return 1;
        }
        other_prerelease = true;
        other_major = version_parts[0];
        other_minor = version_parts[1];
        other_patch = version_parts[2];
    } else {
        let version_parts = version_string_to_int_array(other);
        if version_parts.len() != 3 {
            return 1;
        }
        other_prerelease = false;
        other_major = version_parts[0];
        other_minor = version_parts[1];
        other_patch = version_parts[2];
    }

    if VERSION_MAJOR != other_major {
        return match VERSION_MAJOR.cmp(&other_major) {
            Ordering::Less => -1,
            Ordering::Greater => 1,
            Ordering::Equal => 0,
        };
    }
    if VERSION_MINOR != other_minor {
        return match VERSION_MINOR.cmp(&other_minor) {
            Ordering::Less => -1,
            Ordering::Greater => 1,
            Ordering::Equal => 0,
        };
    }
    if VERSION_PATCH != other_patch {
        return match VERSION_PATCH.cmp(&other_patch) {
            Ordering::Less => -1,
            Ordering::Greater => 1,
            Ordering::Equal => 0,
        };
    }

    let this_prerelease = !BUILD_TYPE.prefix().is_empty();
    if this_prerelease && !other_prerelease {
        return -1;
    }
    if !this_prerelease && other_prerelease {
        return 1;
    }

    0
}

/// Get current build's git commit hash
pub fn git_commit_hash() -> Option<&'static str> {
    build_meta_info().get("git_commit").map(|s| s.as_str())
}

/// Get the build time of the current build
pub fn build_date() -> Option<&'static str> {
    build_meta_info().get("build_time").map(|s| s.as_str())
}

fn build_meta_info() -> &'static HashMap<String, String> {
    BUILD_META_INFO.get_or_init(|| {
        // Try to load build.properties; return empty map on failure
        HashMap::new()
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildType {
    Prerelease,
    Stable,
}

impl BuildType {
    pub fn prefix(&self) -> &str {
        match self {
            BuildType::Prerelease => "pre",
            BuildType::Stable => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_constants() {
        assert_eq!(VERSION_MAJOR, 0);
        assert_eq!(VERSION_MINOR, 5);
        assert_eq!(VERSION_PATCH, 0);
    }

    #[test]
    fn test_build_type_prefix() {
        assert_eq!(BuildType::Prerelease.prefix(), "pre");
        assert_eq!(BuildType::Stable.prefix(), "");
    }

    #[test]
    fn test_unqualified_version_format() {
        let v = unqualified_version();
        assert_eq!(v, "0.5.0");
    }

    #[test]
    fn test_version_includes_build_type_prefix() {
        let v = version();
        // BUILD_TYPE is Prerelease, so version starts with "pre"
        assert!(
            v.starts_with("pre"),
            "version should start with 'pre', got: {}",
            v
        );
        assert_eq!(v, "pre0.5.0");
    }

    #[test]
    fn test_version_long_format() {
        let v = version_long();
        assert!(
            v.contains("rubato"),
            "long version should contain product name, got: {}",
            v
        );
        assert!(
            v.contains("pre-release"),
            "long version should contain 'pre-release' for prerelease build, got: {}",
            v
        );
        assert!(
            v.contains("0.5.0"),
            "long version should contain version number, got: {}",
            v
        );
    }

    #[test]
    fn test_version_struct_delegates() {
        assert_eq!(Version::get_version(), version());
        assert_eq!(Version::long_version(), version_long());
    }

    #[test]
    fn test_compare_to_string_none_returns_positive() {
        assert_eq!(compare_to_string(None), 1);
    }

    #[test]
    fn test_compare_to_string_empty_returns_positive() {
        assert_eq!(compare_to_string(Some("")), 1);
    }

    #[test]
    fn test_compare_to_string_short_returns_positive() {
        assert_eq!(compare_to_string(Some("ab")), 1);
    }

    #[test]
    fn test_compare_to_string_malformed_returns_positive() {
        assert_eq!(compare_to_string(Some("abc")), 1);
    }

    #[test]
    fn test_compare_to_string_same_version_same_prerelease() {
        // Current is pre0.5.0 (Prerelease)
        // Comparing with "pre0.5.0" should be equal
        assert_eq!(compare_to_string(Some("pre0.5.0")), 0);
    }

    #[test]
    fn test_compare_to_string_same_version_other_stable() {
        // Current is Prerelease, other is stable same version
        // Prerelease < Stable, so result is -1
        assert_eq!(compare_to_string(Some("0.5.0")), -1);
    }

    #[test]
    fn test_compare_to_string_older_version() {
        assert_eq!(compare_to_string(Some("0.4.0")), 1);
        assert_eq!(compare_to_string(Some("pre0.4.0")), 1);
    }

    #[test]
    fn test_compare_to_string_newer_version() {
        assert_eq!(compare_to_string(Some("0.6.0")), -1);
        assert_eq!(compare_to_string(Some("1.0.0")), -1);
    }

    #[test]
    fn test_compare_to_string_newer_patch() {
        assert_eq!(compare_to_string(Some("pre0.5.1")), -1);
    }

    #[test]
    fn test_compare_to_string_older_minor() {
        assert_eq!(compare_to_string(Some("pre0.4.9")), 1);
    }

    #[test]
    fn test_git_commit_hash_is_none_without_build_properties() {
        // No build.properties file, so hash should be None
        assert!(git_commit_hash().is_none());
    }

    #[test]
    fn test_build_date_is_none_without_build_properties() {
        assert!(build_date().is_none());
    }

    #[test]
    fn test_build_type_equality() {
        assert_eq!(BuildType::Prerelease, BuildType::Prerelease);
        assert_eq!(BuildType::Stable, BuildType::Stable);
        assert_ne!(BuildType::Prerelease, BuildType::Stable);
    }

    #[test]
    fn test_compare_to_string_non_ascii_does_not_panic() {
        // Non-ASCII input with byte length >= 3 must not panic on slicing
        let result = compare_to_string(Some("\u{00e4}\u{00f6}\u{00fc}"));
        assert_eq!(result, 1);
    }

    #[test]
    fn test_compare_to_string_multibyte_does_not_panic() {
        // Multi-byte UTF-8: 3 chars but 9 bytes
        let result = compare_to_string(Some("\u{3042}\u{3044}\u{3046}"));
        assert_eq!(result, 1);
    }

    #[test]
    fn test_compare_to_string_pre_prefix_with_non_ascii_suffix() {
        // "pre" followed by non-ASCII: must not panic when slicing the rest
        let result = compare_to_string(Some("pre\u{00e4}.\u{00f6}.\u{00fc}"));
        assert_eq!(result, 1);
    }
}
