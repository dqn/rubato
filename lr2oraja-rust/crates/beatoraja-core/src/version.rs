use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::OnceLock;

/// Version utility struct (mirrors Java's static Version class)
pub struct Version;

#[allow(dead_code)]
impl Version {
    pub fn get_version() -> &'static str {
        version()
    }

    pub fn get_long_version() -> &'static str {
        version_long()
    }

    pub fn compare_to_string(other: Option<&str>) -> i32 {
        compare_to_string(other)
    }

    pub fn get_git_commit_hash() -> Option<&'static str> {
        get_git_commit_hash()
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
        format!("LR2oraja Endless Dream {}{}", pre, unqualified_version())
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
    if &other[0..3] == "pre" {
        let version_parts = version_string_to_int_array(&other[3..]);
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
pub fn get_git_commit_hash() -> Option<&'static str> {
    build_meta_info().get("git_commit").map(|s| s.as_str())
}

/// Get the build time of the current build
pub fn get_build_date() -> Option<&'static str> {
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
