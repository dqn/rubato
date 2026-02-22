use crate::file_mapping::FileMapping;
use crate::ir::*;
use crate::naming;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SignatureReport {
    pub file_mappings: Vec<FileMappingResult>,
    pub summary: SignatureSummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileMappingResult {
    pub java_file: String,
    pub rust_file: Option<String>,
    pub rust_crate: Option<String>,
    pub type_mappings: Vec<TypeMappingResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeMappingResult {
    pub java_type: String,
    pub java_kind: String,
    pub rust_type: Option<String>,
    pub method_mappings: Vec<MethodMappingResult>,
    pub field_count_java: usize,
    pub field_count_rust: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MethodMappingResult {
    pub java_method: String,
    pub java_params: usize,
    pub java_line: usize,
    pub rust_method: Option<String>,
    pub rust_line: Option<usize>,
    pub status: MappingStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum MappingStatus {
    Matched,
    NameConverted,
    FieldAccess,
    ConstructorOverload,
    MethodOverload,
    StandardTraitImpl,
    MissingInRust,
    ExtraInRust,
    RustSpecific,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignatureSummary {
    pub total_java_files: usize,
    pub mapped_files: usize,
    pub unmapped_files: usize,
    pub ignored_files: usize,
    pub total_java_types: usize,
    pub matched_types: usize,
    pub total_java_methods: usize,
    pub matched_methods: usize,
    pub field_access_methods: usize,
    pub constructor_overloads: usize,
    pub method_overloads: usize,
    pub standard_trait_impls: usize,
    pub missing_methods: usize,
    pub extra_rust_methods: usize,
    pub rust_specific_methods: usize,
}

/// Java file patterns that are intentionally not 1:1 mapped to Rust.
/// These are files that were replaced by different libraries, merged into other files,
/// or structurally translated differently.
fn ignored_java_patterns() -> Vec<&'static str> {
    vec![
        // bmson POJOs — merged into single Rust modules
        "bmson/BGA.java",
        "bmson/BGAHeader.java",
        "bmson/BarLine.java",
        "bmson/BmsonObject.java",
        "bmson/MineChannel.java",
        "bmson/Note.java",
        "bmson/SoundChannel.java",
        // Note subclasses — become enum variants in Rust
        "model/LongNote.java",
        "model/MineNote.java",
        "model/NormalNote.java",
        // Platform-specific Java implementations replaced by different Rust libs
        "PortAudioDriver.java",
        "PortAudioMixer.java",
        "external/DiscordRPC/DiscordRPC.java",
        "external/DiscordRPC/DiscordRichPresence.java",
        "external/DiscordRPC/DiscordUser.java",
        // JavaFX / ImGui specific (replaced by egui)
        "JavaFXUtils.java",
        "ImGuiRenderer.java",
        // Internal Java utilities with no Rust equivalent
        "util/ArraySerializer.java",
    ]
}

fn is_ignored_java_file(java_path: &str) -> bool {
    let patterns = ignored_java_patterns();
    patterns.iter().any(|pattern| java_path.ends_with(pattern))
}

/// Build a signature mapping report from parsed Java and Rust sources.
pub fn build_signature_map(
    file_mappings: &[FileMapping],
    java_files: &[SourceFile],
    rust_files: &[SourceFile],
) -> SignatureReport {
    let mut results = Vec::new();
    let mut summary = SignatureSummary {
        total_java_files: file_mappings.len(),
        mapped_files: 0,
        unmapped_files: 0,
        ignored_files: 0,
        total_java_types: 0,
        matched_types: 0,
        total_java_methods: 0,
        matched_methods: 0,
        field_access_methods: 0,
        constructor_overloads: 0,
        method_overloads: 0,
        standard_trait_impls: 0,
        missing_methods: 0,
        extra_rust_methods: 0,
        rust_specific_methods: 0,
    };

    for fm in file_mappings {
        let java_path_str = fm.java_path.display().to_string();

        // Skip ignored files
        if is_ignored_java_file(&java_path_str) {
            summary.ignored_files += 1;
            continue;
        }

        let java_source = java_files.iter().find(|f| f.path == fm.java_path);
        let rust_source = fm
            .rust_path
            .as_ref()
            .and_then(|rp| rust_files.iter().find(|f| f.path == *rp));

        if fm.rust_path.is_some() {
            summary.mapped_files += 1;
        } else {
            summary.unmapped_files += 1;
        }

        let type_mappings = match (java_source, rust_source) {
            (Some(java), Some(rust)) => build_type_mappings(
                &java.types,
                &rust.types,
                &rust.free_functions,
                rust_files,
                &mut summary,
            ),
            (Some(java), None) => {
                // All Java types are unmapped
                let mut mappings = Vec::new();
                for jt in &java.types {
                    summary.total_java_types += 1;
                    let method_count = jt.methods.len();
                    summary.total_java_methods += method_count;
                    summary.missing_methods += method_count;
                    mappings.push(TypeMappingResult {
                        java_type: jt.name.clone(),
                        java_kind: format!("{:?}", jt.kind),
                        rust_type: None,
                        method_mappings: jt
                            .methods
                            .iter()
                            .map(|m| MethodMappingResult {
                                java_method: m.name.clone(),
                                java_params: m.params.len(),
                                java_line: m.line,
                                rust_method: None,
                                rust_line: None,
                                status: MappingStatus::MissingInRust,
                            })
                            .collect(),
                        field_count_java: jt.fields.len(),
                        field_count_rust: 0,
                    });
                }
                mappings
            }
            _ => Vec::new(),
        };

        results.push(FileMappingResult {
            java_file: fm.java_path.display().to_string(),
            rust_file: fm.rust_path.as_ref().map(|p| p.display().to_string()),
            rust_crate: fm.rust_crate.clone(),
            type_mappings,
        });
    }

    SignatureReport {
        file_mappings: results,
        summary,
    }
}

fn build_type_mappings(
    java_types: &[TypeDecl],
    rust_types: &[TypeDecl],
    rust_free_fns: &[MethodDecl],
    all_rust_files: &[SourceFile],
    summary: &mut SignatureSummary,
) -> Vec<TypeMappingResult> {
    let mut results = Vec::new();

    for jt in java_types {
        summary.total_java_types += 1;

        // Find matching Rust type by name:
        // 1. Exact match in mapped file
        // 2. Exact match globally (handles pub use re-exports)
        // 3. Fuzzy match globally (handles Abstract/Base prefix removal)
        let rust_type = find_rust_type(&jt.name, rust_types)
            .or_else(|| find_rust_type_globally(&jt.name, all_rust_files))
            .or_else(|| find_rust_type_fuzzy(&jt.name, rust_types, all_rust_files));

        if rust_type.is_some() {
            summary.matched_types += 1;
        }

        let rust_methods: &[MethodDecl] = match &rust_type {
            Some(rt) => &rt.methods,
            None => &[],
        };

        let rust_fields: &[FieldDecl] = match &rust_type {
            Some(rt) => &rt.fields,
            None => &[],
        };

        // Also consider free functions for the first type in a file
        let all_rust_methods: Vec<&MethodDecl> =
            rust_methods.iter().chain(rust_free_fns.iter()).collect();

        let method_mappings =
            build_method_mappings(&jt.methods, &all_rust_methods, rust_fields, summary);

        results.push(TypeMappingResult {
            java_type: jt.name.clone(),
            java_kind: format!("{:?}", jt.kind),
            rust_type: rust_type.map(|rt| rt.name.clone()),
            method_mappings,
            field_count_java: jt.fields.len(),
            field_count_rust: rust_type.map(|rt| rt.fields.len()).unwrap_or(0),
        });

        // Recursively handle inner types
        if !jt.inner_types.is_empty() {
            let inner_results =
                build_type_mappings(&jt.inner_types, rust_types, &[], all_rust_files, summary);
            results.extend(inner_results);
        }
    }

    // Find extra Rust types (not in Java)
    for rt in rust_types {
        let is_matched = java_types
            .iter()
            .any(|jt| find_rust_type_match(&jt.name, &rt.name));
        if !is_matched {
            // Count extra methods that aren't Rust-specific
            for m in &rt.methods {
                if naming::is_rust_specific_method(&m.name) {
                    summary.rust_specific_methods += 1;
                } else {
                    summary.extra_rust_methods += 1;
                }
            }
        }
    }

    results
}

fn find_rust_type<'a>(java_name: &str, rust_types: &'a [TypeDecl]) -> Option<&'a TypeDecl> {
    rust_types
        .iter()
        .find(|rt| find_rust_type_match(java_name, &rt.name))
}

fn find_rust_type_match(java_name: &str, rust_name: &str) -> bool {
    // Direct name match (both use PascalCase for types)
    java_name == rust_name
}

/// Search for a Rust type globally across all parsed Rust source files.
/// Used as a fallback when the type isn't found in the directly mapped file
/// (e.g., when the file just contains `pub use beatoraja_types::*`).
fn find_rust_type_globally<'a>(
    java_name: &str,
    all_rust_files: &'a [SourceFile],
) -> Option<&'a TypeDecl> {
    for source in all_rust_files {
        if let Some(found) = find_rust_type(java_name, &source.types) {
            return Some(found);
        }
    }
    None
}

/// Search for a Rust type using fuzzy name matching.
/// Tries stripping common Java prefixes like Abstract/Base/Default/I.
fn find_rust_type_fuzzy<'a>(
    java_name: &str,
    local_types: &'a [TypeDecl],
    all_rust_files: &'a [SourceFile],
) -> Option<&'a TypeDecl> {
    for candidate in naming::fuzzy_type_candidates(java_name) {
        // Try local first
        if let Some(found) = find_rust_type(&candidate, local_types) {
            return Some(found);
        }
        // Try global
        if let Some(found) = find_rust_type_globally(&candidate, all_rust_files) {
            return Some(found);
        }
    }
    None
}

fn build_method_mappings(
    java_methods: &[MethodDecl],
    rust_methods: &[&MethodDecl],
    rust_fields: &[FieldDecl],
    summary: &mut SignatureSummary,
) -> Vec<MethodMappingResult> {
    let mut results = Vec::new();
    let mut matched_rust_indices = Vec::new();
    // Track whether new/default has been used for constructor matching
    let mut constructor_primary_matched = false;
    // Track method names that have already been matched (for overload detection)
    let mut matched_java_names: Vec<String> = Vec::new();

    for jm in java_methods {
        summary.total_java_methods += 1;

        let (rust_match, status) = find_rust_method(&jm.name, rust_methods, &matched_rust_indices);

        if let Some((idx, rm)) = rust_match {
            matched_rust_indices.push(idx);
            if naming::is_constructor(&jm.name) && (rm.name == "new" || rm.name == "default") {
                constructor_primary_matched = true;
            }
            matched_java_names.push(jm.name.clone());
            summary.matched_methods += 1;
            results.push(MethodMappingResult {
                java_method: jm.name.clone(),
                java_params: jm.params.len(),
                java_line: jm.line,
                rust_method: Some(rm.name.clone()),
                rust_line: Some(rm.line),
                status,
            });
            continue;
        }

        // Try field matching for accessors (getter/setter/is/has → pub field)
        let field_candidates = naming::accessor_field_candidates(&jm.name);
        let field_match = field_candidates
            .iter()
            .find(|candidate| rust_fields.iter().any(|f| f.name == **candidate));

        if let Some(matched_field) = field_match {
            matched_java_names.push(jm.name.clone());
            summary.field_access_methods += 1;
            results.push(MethodMappingResult {
                java_method: jm.name.clone(),
                java_params: jm.params.len(),
                java_line: jm.line,
                rust_method: Some(format!("(field: {})", matched_field)),
                rust_line: None,
                status: MappingStatus::FieldAccess,
            });
            continue;
        }

        // Method overload detection — same name already matched or field-accessed
        if matched_java_names.contains(&jm.name) {
            summary.method_overloads += 1;
            results.push(MethodMappingResult {
                java_method: jm.name.clone(),
                java_params: jm.params.len(),
                java_line: jm.line,
                rust_method: Some("(overload)".to_string()),
                rust_line: None,
                status: MappingStatus::MethodOverload,
            });
            continue;
        }

        // Java standard method → Rust trait mapping
        if let Some(trait_name) = naming::java_standard_method_trait(&jm.name) {
            summary.standard_trait_impls += 1;
            results.push(MethodMappingResult {
                java_method: jm.name.clone(),
                java_params: jm.params.len(),
                java_line: jm.line,
                rust_method: Some(format!("(trait: {})", trait_name)),
                rust_line: None,
                status: MappingStatus::StandardTraitImpl,
            });
            matched_java_names.push(jm.name.clone());
            continue;
        }

        // Constructor handling
        if naming::is_constructor(&jm.name) {
            if constructor_primary_matched {
                summary.constructor_overloads += 1;
                results.push(MethodMappingResult {
                    java_method: jm.name.clone(),
                    java_params: jm.params.len(),
                    java_line: jm.line,
                    rust_method: Some("(constructor overload)".to_string()),
                    rust_line: None,
                    status: MappingStatus::ConstructorOverload,
                });
                continue;
            }

            // Try matching new/default even if already used by exclusion
            let ctor_match = rust_methods
                .iter()
                .enumerate()
                .find(|(_, rm)| rm.name == "new" || rm.name == "default");
            if let Some((_, rm)) = ctor_match {
                constructor_primary_matched = true;
                summary.constructor_overloads += 1;
                results.push(MethodMappingResult {
                    java_method: jm.name.clone(),
                    java_params: jm.params.len(),
                    java_line: jm.line,
                    rust_method: Some(rm.name.clone()),
                    rust_line: Some(rm.line),
                    status: MappingStatus::ConstructorOverload,
                });
                continue;
            }
        }

        // Fuzzy field matching for accessors — check if any Rust field contains the key part
        if let Some(field_name) = naming::accessor_field_name(&jm.name) {
            let fuzzy_match = rust_fields.iter().find(|f| {
                f.name.contains(&field_name)
                    || field_name.contains(&f.name)
                    || edit_distance_within(&f.name, &field_name, 2)
            });
            if let Some(rf) = fuzzy_match {
                matched_java_names.push(jm.name.clone());
                summary.field_access_methods += 1;
                results.push(MethodMappingResult {
                    java_method: jm.name.clone(),
                    java_params: jm.params.len(),
                    java_line: jm.line,
                    rust_method: Some(format!("(field~: {})", rf.name)),
                    rust_line: None,
                    status: MappingStatus::FieldAccess,
                });
                continue;
            }
        }

        summary.missing_methods += 1;
        results.push(MethodMappingResult {
            java_method: jm.name.clone(),
            java_params: jm.params.len(),
            java_line: jm.line,
            rust_method: None,
            rust_line: None,
            status: MappingStatus::MissingInRust,
        });
    }

    // Find extra Rust methods
    for (i, rm) in rust_methods.iter().enumerate() {
        if !matched_rust_indices.contains(&i) {
            if naming::is_rust_specific_method(&rm.name) {
                summary.rust_specific_methods += 1;
            } else {
                summary.extra_rust_methods += 1;
                results.push(MethodMappingResult {
                    java_method: String::new(),
                    java_params: 0,
                    java_line: 0,
                    rust_method: Some(rm.name.clone()),
                    rust_line: Some(rm.line),
                    status: MappingStatus::ExtraInRust,
                });
            }
        }
    }

    results
}

/// Check if two strings have edit distance within the given threshold.
/// Uses a simple Levenshtein distance with early termination.
fn edit_distance_within(a: &str, b: &str, max_dist: usize) -> bool {
    let a_len = a.len();
    let b_len = b.len();

    if a_len.abs_diff(b_len) > max_dist {
        return false;
    }

    let mut prev: Vec<usize> = (0..=b_len).collect();
    let mut curr = vec![0usize; b_len + 1];

    for (i, ca) in a.chars().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(curr[j] + 1);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[b_len] <= max_dist
}

fn find_rust_method<'a>(
    java_name: &str,
    rust_methods: &[&'a MethodDecl],
    excluded_indices: &[usize],
) -> (Option<(usize, &'a MethodDecl)>, MappingStatus) {
    // Try direct snake_case conversion
    let snake_name = naming::method_to_snake(java_name);

    for (i, rm) in rust_methods.iter().enumerate() {
        if excluded_indices.contains(&i) {
            continue;
        }
        if rm.name == snake_name {
            return (Some((i, rm)), MappingStatus::Matched);
        }
    }

    // Try getter/setter candidates
    if naming::is_getter(java_name) {
        for candidate in naming::getter_candidates(java_name) {
            for (i, rm) in rust_methods.iter().enumerate() {
                if excluded_indices.contains(&i) {
                    continue;
                }
                if rm.name == candidate {
                    return (Some((i, rm)), MappingStatus::NameConverted);
                }
            }
        }
    }

    // Try Java constructor → Rust new()
    if java_name
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase())
        || java_name == "<init>"
    {
        for (i, rm) in rust_methods.iter().enumerate() {
            if excluded_indices.contains(&i) {
                continue;
            }
            if rm.name == "new" || rm.name == "default" {
                return (Some((i, rm)), MappingStatus::NameConverted);
            }
        }
    }

    (None, MappingStatus::MissingInRust)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn make_java_method(name: &str, params: usize) -> MethodDecl {
        MethodDecl {
            name: name.to_string(),
            visibility: Visibility::Public,
            is_static: false,
            is_abstract: false,
            params: (0..params)
                .map(|i| ParamDecl {
                    name: format!("arg{i}"),
                    type_name: "int".to_string(),
                })
                .collect(),
            return_type: None,
            body: None,
            line: 1,
        }
    }

    fn make_rust_method(name: &str) -> MethodDecl {
        MethodDecl {
            name: name.to_string(),
            visibility: Visibility::Public,
            is_static: false,
            is_abstract: false,
            params: Vec::new(),
            return_type: None,
            body: None,
            line: 1,
        }
    }

    #[test]
    fn test_method_matching_snake_case() {
        let rust_methods = vec![make_rust_method("get_micro_time")];
        let rust_refs: Vec<&MethodDecl> = rust_methods.iter().collect();
        let (result, status) = find_rust_method("getMicroTime", &rust_refs, &[]);
        assert!(result.is_some());
        assert_eq!(status, MappingStatus::Matched);
    }

    #[test]
    fn test_method_matching_getter_short() {
        let rust_methods = vec![make_rust_method("title")];
        let rust_refs: Vec<&MethodDecl> = rust_methods.iter().collect();
        let (result, status) = find_rust_method("getTitle", &rust_refs, &[]);
        assert!(result.is_some());
        assert_eq!(status, MappingStatus::NameConverted);
    }

    #[test]
    fn test_method_matching_constructor() {
        let rust_methods = vec![make_rust_method("new")];
        let rust_refs: Vec<&MethodDecl> = rust_methods.iter().collect();
        let (result, status) = find_rust_method("<init>", &rust_refs, &[]);
        assert!(result.is_some());
        assert_eq!(status, MappingStatus::NameConverted);
    }

    #[test]
    fn test_method_not_found() {
        let rust_methods = vec![make_rust_method("something_else")];
        let rust_refs: Vec<&MethodDecl> = rust_methods.iter().collect();
        let (result, _status) = find_rust_method("getMicroTime", &rust_refs, &[]);
        assert!(result.is_none());
    }
}
