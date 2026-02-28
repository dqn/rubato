use std::path::Path;

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
    FuzzyMethodMatch,
    ParameterLifted,
    MatchedStub,
    VisibilityFiltered,
    MissingInRust,
    ExtraInRust,
    RustSpecific,
}

/// Filter for Java method visibility levels.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VisibilityFilter {
    /// Include all methods regardless of visibility (default).
    #[default]
    All,
    /// Include only public and protected methods.
    PublicProtected,
    /// Include only public methods.
    Public,
}

impl VisibilityFilter {
    pub fn includes(&self, vis: Visibility) -> bool {
        match self {
            VisibilityFilter::All => true,
            VisibilityFilter::Public => vis == Visibility::Public,
            VisibilityFilter::PublicProtected => {
                matches!(vis, Visibility::Public | Visibility::Protected)
            }
        }
    }
}

/// Configuration for the signature mapping pass.
#[derive(Debug, Clone, Default)]
pub struct MapConfig {
    pub visibility_filter: VisibilityFilter,
    pub include_stubs: bool,
    pub ignore_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignatureSummary {
    pub total_java_files: usize,
    pub mapped_files: usize,
    pub unmapped_files: usize,
    pub ignored_files: usize,
    pub total_java_types: usize,
    pub matched_types: usize,
    pub ignored_types: usize,
    pub total_java_methods: usize,
    pub matched_methods: usize,
    pub field_access_methods: usize,
    pub constructor_overloads: usize,
    pub method_overloads: usize,
    pub standard_trait_impls: usize,
    pub fuzzy_method_matches: usize,
    pub parameter_lifted: usize,
    pub matched_stubs: usize,
    pub visibility_filtered: usize,
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
        "util/Pair.java",
        // Audio format (replaced by different Rust audio handling)
        "audio/ShortDirectPCM.java",
        // Discord IPC (platform-specific, replaced by discord-rich-presence crate)
        "external/DiscordRPC/IPCConnection.java",
        "external/DiscordRPC/UnixIPCConnection.java",
        "external/DiscordRPC/WindowsIPCConnection.java",
        // bmson container/info types (merged into single Rust module)
        "bmson/Bmson.java",
        "bmson/BMSInfo.java",
        // osu format container
        "osu/Osu.java",
        // Exception class (replaced by anyhow)
        "exceptions/PlayerConfigException.java",
        // bmson POJOs — merged into bms-model/src/bmson/mod.rs
        "bmson/BGASequence.java",
        "bmson/BMSONObject.java",
        "bmson/BpmEvent.java",
        "bmson/MineNote.java",
        "bmson/ScrollEvent.java",
        "bmson/Sequence.java",
        "bmson/StopEvent.java",
        // osu POJOs — merged into bms-model/src/osu/mod.rs
        "osu/Colours.java",
        "osu/Difficulty.java",
        "osu/Editor.java",
        "osu/Events.java",
        "osu/General.java",
        "osu/HitObjects.java",
        "osu/Metadata.java",
        "osu/TimingPoints.java",
    ]
}

fn is_ignored_java_file(java_path: &str, custom_patterns: &[String]) -> bool {
    if custom_patterns.is_empty() {
        let patterns = ignored_java_patterns();
        patterns.iter().any(|pattern| java_path.ends_with(pattern))
    } else {
        custom_patterns
            .iter()
            .any(|pattern| java_path.ends_with(pattern.as_str()))
    }
}

/// Load ignore patterns from a file. Each line is a suffix pattern.
/// Lines starting with `#` and empty lines are skipped.
pub fn load_ignore_patterns(path: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.to_string())
        .collect()
}

/// Build a signature mapping report from parsed Java and Rust sources.
pub fn build_signature_map(
    file_mappings: &[FileMapping],
    java_files: &[SourceFile],
    rust_files: &[SourceFile],
    config: &MapConfig,
) -> SignatureReport {
    let mut results = Vec::new();
    let mut summary = SignatureSummary {
        total_java_files: file_mappings.len(),
        mapped_files: 0,
        unmapped_files: 0,
        ignored_files: 0,
        total_java_types: 0,
        matched_types: 0,
        ignored_types: 0,
        total_java_methods: 0,
        matched_methods: 0,
        field_access_methods: 0,
        constructor_overloads: 0,
        method_overloads: 0,
        standard_trait_impls: 0,
        fuzzy_method_matches: 0,
        parameter_lifted: 0,
        matched_stubs: 0,
        visibility_filtered: 0,
        missing_methods: 0,
        extra_rust_methods: 0,
        rust_specific_methods: 0,
    };

    for fm in file_mappings {
        let java_path_str = fm.java_path.display().to_string();

        // Skip ignored files
        if is_ignored_java_file(&java_path_str, &config.ignore_patterns) {
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
                config,
            ),
            (Some(java), None) => {
                // No direct file mapping — try global type search
                build_type_mappings(&java.types, &[], &[], rust_files, &mut summary, config)
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
    config: &MapConfig,
) -> Vec<TypeMappingResult> {
    let mut results = Vec::new();

    for jt in java_types {
        // Skip ignored inner types
        if is_ignored_java_type(&jt.name) {
            summary.ignored_types += 1;
            continue;
        }

        summary.total_java_types += 1;

        // Find matching Rust type by name:
        // 1. Exact match in mapped file (also handles generics and case normalization)
        // 2. Exact match globally (handles pub use re-exports)
        // 3. Fuzzy match globally (handles Abstract/Base prefix removal)
        // 4. Suffix match (handles Data/State/Inner/Impl suffixes)
        let rust_type = find_rust_type(&jt.name, rust_types)
            .or_else(|| find_rust_type_globally(&jt.name, all_rust_files))
            .or_else(|| find_rust_type_fuzzy(&jt.name, rust_types, all_rust_files))
            .or_else(|| find_rust_type_with_suffix(&jt.name, rust_types, all_rust_files));

        if rust_type.is_some() {
            summary.matched_types += 1;
        }

        // Java abstract class → Rust trait + Data struct pattern:
        // Merge methods/fields from both matched type and *Data variant.
        let data_companion = rust_type.and_then(|_| {
            let data_name = format!("{}Data", jt.name);
            find_rust_type(&data_name, rust_types)
                .or_else(|| find_rust_type_globally(&data_name, all_rust_files))
        });

        let rust_methods: Vec<&MethodDecl> = match (&rust_type, &data_companion) {
            (Some(rt), Some(dt)) => rt.methods.iter().chain(dt.methods.iter()).collect(),
            (Some(rt), None) => rt.methods.iter().collect(),
            (None, _) => Vec::new(),
        };

        let rust_fields: Vec<&FieldDecl> = match (&rust_type, &data_companion) {
            (Some(rt), Some(dt)) => rt.fields.iter().chain(dt.fields.iter()).collect(),
            (Some(rt), None) => rt.fields.iter().collect(),
            (None, _) => Vec::new(),
        };

        let rust_field_count = rust_fields.len();

        // Also consider free functions for the first type in a file
        let all_rust_methods: Vec<&MethodDecl> = rust_methods
            .iter()
            .copied()
            .chain(rust_free_fns.iter())
            .collect();

        let method_mappings = build_method_mappings(
            &jt.methods,
            &all_rust_methods,
            &rust_fields,
            summary,
            config,
        );

        results.push(TypeMappingResult {
            java_type: jt.name.clone(),
            java_kind: format!("{:?}", jt.kind),
            rust_type: rust_type.map(|rt| rt.name.clone()),
            method_mappings,
            field_count_java: jt.fields.len(),
            field_count_rust: rust_field_count,
        });

        // Recursively handle inner types
        if !jt.inner_types.is_empty() {
            let inner_results = build_type_mappings(
                &jt.inner_types,
                rust_types,
                &[],
                all_rust_files,
                summary,
                config,
            );
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
    if java_name == rust_name {
        return true;
    }
    // Strip generic parameters: BmsTable<T> → BmsTable
    let rust_base = rust_name.split('<').next().unwrap_or(rust_name);
    if java_name == rust_base {
        return true;
    }
    // Case-normalized comparison via snake_case (BMSTable vs BmsTable → bms_table)
    naming::class_to_module(java_name) == naming::class_to_module(rust_base)
}

/// Search for a Rust type globally across all parsed Rust source files.
/// Used as a fallback when the type isn't found in the directly mapped file
/// (e.g., when the file just contains `pub use beatoraja_types::*`).
/// Returns the type with the most fields+methods to prefer real definitions over stubs.
fn find_rust_type_globally<'a>(
    java_name: &str,
    all_rust_files: &'a [SourceFile],
) -> Option<&'a TypeDecl> {
    let mut best: Option<&TypeDecl> = None;
    for source in all_rust_files {
        if let Some(found) = find_rust_type(java_name, &source.types) {
            match best {
                None => best = Some(found),
                Some(current) => {
                    let found_score = found.fields.len() + found.methods.len();
                    let current_score = current.fields.len() + current.methods.len();
                    if found_score > current_score {
                        best = Some(found);
                    }
                }
            }
        }
    }
    best
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

/// Search for a Rust type by trying common Rust suffixes.
/// Handles patterns like DirectoryBar → DirectoryBarData, LR2PlaySkinLoader → LR2PlaySkinLoaderState.
fn find_rust_type_with_suffix<'a>(
    java_name: &str,
    local_types: &'a [TypeDecl],
    all_rust_files: &'a [SourceFile],
) -> Option<&'a TypeDecl> {
    for suffix in ["Data", "State", "Inner", "Impl", "Info"] {
        let candidate = format!("{java_name}{suffix}");
        if let Some(found) = find_rust_type(&candidate, local_types) {
            return Some(found);
        }
        if let Some(found) = find_rust_type_globally(&candidate, all_rust_files) {
            return Some(found);
        }
    }
    None
}

/// Java inner types that are intentionally not 1:1 mapped to Rust.
fn is_ignored_java_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "DummyAudioDriver"
            | "ValueType"
            | "StringType"
            | "CustomItemBase"
            | "AudioCache"
            | "SoundMixer"
            | "WavInputStream"
            | "MovieSeekThread"
            | "ImageTransferable"
            | "ChangeSingleFieldEvent"
            | "ToggleVisibleEvent"
            | "FloatPropertyFactory"
            | "IntegerPropertyFactory"
            | "StringPropertyFactory"
    )
}

fn build_method_mappings(
    java_methods: &[MethodDecl],
    rust_methods: &[&MethodDecl],
    rust_fields: &[&FieldDecl],
    summary: &mut SignatureSummary,
    config: &MapConfig,
) -> Vec<MethodMappingResult> {
    let mut results = Vec::new();
    let mut matched_rust_indices = Vec::new();
    // Track whether new/default has been used for constructor matching
    let mut constructor_primary_matched = false;
    // Track method names that have already been matched (for overload detection)
    let mut matched_java_names: Vec<String> = Vec::new();
    // Track ALL Java method names encountered (for unmatched-overload detection)
    let mut seen_java_names: Vec<String> = Vec::new();

    // Collect matched Rust method parameter names for parameter-lifted detection
    let matched_rust_param_names: Vec<String> = rust_methods
        .iter()
        .flat_map(|rm| rm.params.iter().map(|p| p.name.clone()))
        .collect();

    for jm in java_methods {
        summary.total_java_methods += 1;

        // Visibility filter — skip methods below the configured threshold
        if !config.visibility_filter.includes(jm.visibility) {
            summary.visibility_filtered += 1;
            results.push(MethodMappingResult {
                java_method: jm.name.clone(),
                java_params: jm.params.len(),
                java_line: jm.line,
                rust_method: Some("(visibility filtered)".to_string()),
                rust_line: None,
                status: MappingStatus::VisibilityFiltered,
            });
            seen_java_names.push(jm.name.clone());
            continue;
        }

        let (rust_match, status) = find_rust_method(&jm.name, rust_methods, &matched_rust_indices);

        if let Some((idx, rm)) = rust_match {
            matched_rust_indices.push(idx);
            if naming::is_constructor(&jm.name) && (rm.name == "new" || rm.name == "default") {
                constructor_primary_matched = true;
            }
            matched_java_names.push(jm.name.clone());
            seen_java_names.push(jm.name.clone());

            // Check if the matched Rust method is a stub
            let is_stub = rm.body.as_ref().is_some_and(|b| b.is_stub);
            if is_stub && !config.include_stubs {
                summary.matched_stubs += 1;
                results.push(MethodMappingResult {
                    java_method: jm.name.clone(),
                    java_params: jm.params.len(),
                    java_line: jm.line,
                    rust_method: Some(rm.name.clone()),
                    rust_line: Some(rm.line),
                    status: MappingStatus::MatchedStub,
                });
            } else {
                summary.matched_methods += 1;
                results.push(MethodMappingResult {
                    java_method: jm.name.clone(),
                    java_params: jm.params.len(),
                    java_line: jm.line,
                    rust_method: Some(rm.name.clone()),
                    rust_line: Some(rm.line),
                    status,
                });
            }
            continue;
        }

        // Try field matching for accessors (getter/setter/is/has → pub field)
        let field_candidates = naming::accessor_field_candidates(&jm.name);
        let field_match = field_candidates.iter().find(|candidate| {
            rust_fields.iter().any(|f| {
                f.name == **candidate
                    || f.serde_rename
                        .as_ref()
                        .is_some_and(|r| naming::method_to_snake(r) == **candidate)
            })
        });

        if let Some(matched_field) = field_match {
            matched_java_names.push(jm.name.clone());
            seen_java_names.push(jm.name.clone());
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

        // Method overload detection — same name already seen (resolved or missing)
        if seen_java_names.contains(&jm.name) {
            seen_java_names.push(jm.name.clone());
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
            seen_java_names.push(jm.name.clone());
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
                seen_java_names.push(jm.name.clone());
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
                seen_java_names.push(jm.name.clone());
                continue;
            }

            // Constructor in a type that has Rust methods/fields but no new/default
            if !rust_methods.is_empty() || !rust_fields.is_empty() {
                constructor_primary_matched = true;
                summary.constructor_overloads += 1;
                results.push(MethodMappingResult {
                    java_method: jm.name.clone(),
                    java_params: jm.params.len(),
                    java_line: jm.line,
                    rust_method: Some("(constructor, no new/default)".to_string()),
                    rust_line: None,
                    status: MappingStatus::ConstructorOverload,
                });
                seen_java_names.push(jm.name.clone());
                continue;
            }
        }

        // Fuzzy field matching for accessors — check if any Rust field contains the key part
        if let Some(field_name) = naming::accessor_field_name(&jm.name) {
            let fuzzy_match = rust_fields.iter().find(|f| {
                f.name.contains(&field_name)
                    || field_name.contains(&f.name)
                    || naming::edit_distance_within(&f.name, &field_name, 2)
            });
            if let Some(rf) = fuzzy_match {
                matched_java_names.push(jm.name.clone());
                seen_java_names.push(jm.name.clone());
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

        // Fuzzy method name matching (edit distance or containment)
        let snake_name = naming::method_to_snake(&jm.name);
        let fuzzy_method = rust_methods
            .iter()
            .enumerate()
            .filter(|(i, _)| !matched_rust_indices.contains(i))
            .find(|(_, rm)| {
                naming::edit_distance_within(&rm.name, &snake_name, 2)
                    || (rm.name.len() >= 4
                        && snake_name.len() >= 4
                        && (rm.name.contains(snake_name.as_str())
                            || snake_name.contains(rm.name.as_str())))
            });

        if let Some((idx, rm)) = fuzzy_method {
            matched_rust_indices.push(idx);
            seen_java_names.push(jm.name.clone());
            summary.fuzzy_method_matches += 1;
            results.push(MethodMappingResult {
                java_method: jm.name.clone(),
                java_params: jm.params.len(),
                java_line: jm.line,
                rust_method: Some(rm.name.clone()),
                rust_line: Some(rm.line),
                status: MappingStatus::FuzzyMethodMatch,
            });
            continue;
        }

        // Parameter lifted detection — accessor field name appears as a Rust method parameter
        if let Some(field_name) = naming::accessor_field_name(&jm.name) {
            let param_match = matched_rust_param_names
                .iter()
                .any(|p| *p == field_name || naming::method_to_snake(p) == field_name);
            if param_match {
                summary.parameter_lifted += 1;
                seen_java_names.push(jm.name.clone());
                results.push(MethodMappingResult {
                    java_method: jm.name.clone(),
                    java_params: jm.params.len(),
                    java_line: jm.line,
                    rust_method: Some(format!("(param: {})", field_name)),
                    rust_line: None,
                    status: MappingStatus::ParameterLifted,
                });
                continue;
            }
        }

        summary.missing_methods += 1;
        seen_java_names.push(jm.name.clone());
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
        let rust_methods = [make_rust_method("get_micro_time")];
        let rust_refs: Vec<&MethodDecl> = rust_methods.iter().collect();
        let (result, status) = find_rust_method("getMicroTime", &rust_refs, &[]);
        assert!(result.is_some());
        assert_eq!(status, MappingStatus::Matched);
    }

    #[test]
    fn test_method_matching_getter_short() {
        let rust_methods = [make_rust_method("title")];
        let rust_refs: Vec<&MethodDecl> = rust_methods.iter().collect();
        let (result, status) = find_rust_method("getTitle", &rust_refs, &[]);
        assert!(result.is_some());
        assert_eq!(status, MappingStatus::NameConverted);
    }

    #[test]
    fn test_method_matching_constructor() {
        let rust_methods = [make_rust_method("new")];
        let rust_refs: Vec<&MethodDecl> = rust_methods.iter().collect();
        let (result, status) = find_rust_method("<init>", &rust_refs, &[]);
        assert!(result.is_some());
        assert_eq!(status, MappingStatus::NameConverted);
    }

    #[test]
    fn test_method_not_found() {
        let rust_methods = [make_rust_method("something_else")];
        let rust_refs: Vec<&MethodDecl> = rust_methods.iter().collect();
        let (result, _status) = find_rust_method("getMicroTime", &rust_refs, &[]);
        assert!(result.is_none());
    }

    fn make_java_method_with_visibility(name: &str, vis: Visibility) -> MethodDecl {
        MethodDecl {
            name: name.to_string(),
            visibility: vis,
            is_static: false,
            is_abstract: false,
            params: Vec::new(),
            return_type: None,
            body: None,
            line: 1,
        }
    }

    fn make_rust_method_with_params(name: &str, param_names: &[&str]) -> MethodDecl {
        MethodDecl {
            name: name.to_string(),
            visibility: Visibility::Public,
            is_static: false,
            is_abstract: false,
            params: param_names
                .iter()
                .map(|p| ParamDecl {
                    name: p.to_string(),
                    type_name: "i32".to_string(),
                })
                .collect(),
            return_type: None,
            body: None,
            line: 1,
        }
    }

    #[test]
    fn test_visibility_filter_public() {
        let java_methods = vec![
            make_java_method_with_visibility("publicMethod", Visibility::Public),
            make_java_method_with_visibility("privateMethod", Visibility::Private),
            make_java_method_with_visibility("protectedMethod", Visibility::Protected),
        ];
        let rust_methods: Vec<&MethodDecl> = vec![];
        let rust_fields: Vec<&FieldDecl> = vec![];
        let mut summary = SignatureSummary {
            total_java_files: 0,
            mapped_files: 0,
            unmapped_files: 0,
            ignored_files: 0,
            total_java_types: 0,
            matched_types: 0,
            ignored_types: 0,
            total_java_methods: 0,
            matched_methods: 0,
            field_access_methods: 0,
            constructor_overloads: 0,
            method_overloads: 0,
            standard_trait_impls: 0,
            fuzzy_method_matches: 0,
            parameter_lifted: 0,
            matched_stubs: 0,
            visibility_filtered: 0,
            missing_methods: 0,
            extra_rust_methods: 0,
            rust_specific_methods: 0,
        };
        let config = MapConfig {
            visibility_filter: VisibilityFilter::Public,
            ..Default::default()
        };
        let results = build_method_mappings(
            &java_methods,
            &rust_methods,
            &rust_fields,
            &mut summary,
            &config,
        );
        // Only publicMethod should be MissingInRust; others are VisibilityFiltered
        assert_eq!(summary.visibility_filtered, 2);
        assert_eq!(summary.missing_methods, 1);
        assert_eq!(
            results
                .iter()
                .filter(|r| r.status == MappingStatus::VisibilityFiltered)
                .count(),
            2
        );
    }

    #[test]
    fn test_visibility_filter_public_protected() {
        let java_methods = vec![
            make_java_method_with_visibility("publicMethod", Visibility::Public),
            make_java_method_with_visibility("privateMethod", Visibility::Private),
            make_java_method_with_visibility("protectedMethod", Visibility::Protected),
        ];
        let rust_methods: Vec<&MethodDecl> = vec![];
        let rust_fields: Vec<&FieldDecl> = vec![];
        let mut summary = SignatureSummary {
            total_java_files: 0,
            mapped_files: 0,
            unmapped_files: 0,
            ignored_files: 0,
            total_java_types: 0,
            matched_types: 0,
            ignored_types: 0,
            total_java_methods: 0,
            matched_methods: 0,
            field_access_methods: 0,
            constructor_overloads: 0,
            method_overloads: 0,
            standard_trait_impls: 0,
            fuzzy_method_matches: 0,
            parameter_lifted: 0,
            matched_stubs: 0,
            visibility_filtered: 0,
            missing_methods: 0,
            extra_rust_methods: 0,
            rust_specific_methods: 0,
        };
        let config = MapConfig {
            visibility_filter: VisibilityFilter::PublicProtected,
            ..Default::default()
        };
        let results = build_method_mappings(
            &java_methods,
            &rust_methods,
            &rust_fields,
            &mut summary,
            &config,
        );
        // Only privateMethod is filtered
        assert_eq!(summary.visibility_filtered, 1);
        assert_eq!(summary.missing_methods, 2);
        assert_eq!(
            results
                .iter()
                .filter(|r| r.status == MappingStatus::VisibilityFiltered)
                .count(),
            1
        );
    }

    #[test]
    fn test_parameter_lifted_detection() {
        let java_methods = vec![make_java_method_with_visibility(
            "getKeyVolume",
            Visibility::Public,
        )];
        let rm = make_rust_method_with_params("update", &["key_volume", "bpm"]);
        let rust_methods: Vec<&MethodDecl> = vec![&rm];
        let rust_fields: Vec<&FieldDecl> = vec![];
        let mut summary = SignatureSummary {
            total_java_files: 0,
            mapped_files: 0,
            unmapped_files: 0,
            ignored_files: 0,
            total_java_types: 0,
            matched_types: 0,
            ignored_types: 0,
            total_java_methods: 0,
            matched_methods: 0,
            field_access_methods: 0,
            constructor_overloads: 0,
            method_overloads: 0,
            standard_trait_impls: 0,
            fuzzy_method_matches: 0,
            parameter_lifted: 0,
            matched_stubs: 0,
            visibility_filtered: 0,
            missing_methods: 0,
            extra_rust_methods: 0,
            rust_specific_methods: 0,
        };
        let config = MapConfig::default();
        let results = build_method_mappings(
            &java_methods,
            &rust_methods,
            &rust_fields,
            &mut summary,
            &config,
        );
        assert_eq!(summary.parameter_lifted, 1);
        assert_eq!(results[0].status, MappingStatus::ParameterLifted);
    }

    #[test]
    fn test_ignore_patterns_from_file() {
        // Test hardcoded fallback
        assert!(is_ignored_java_file("foo/bmson/BGA.java", &[]));
        assert!(!is_ignored_java_file("foo/Config.java", &[]));

        // Test custom patterns
        let custom = vec!["Config.java".to_string()];
        assert!(is_ignored_java_file("foo/Config.java", &custom));
        assert!(!is_ignored_java_file("foo/bmson/BGA.java", &custom));
    }
}
