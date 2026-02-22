use colored::*;

use crate::constants::ConstantsReport;
use crate::signature_map::{MappingStatus, SignatureReport};
use crate::structural_compare::StructuralReport;

/// Format the signature mapping report as human-readable text.
pub fn format_signature_report(report: &SignatureReport) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "{}\n\n",
        "=== Signature Mapping Report ===".bold()
    ));

    // Unmapped files
    let unmapped_files: Vec<_> = report
        .file_mappings
        .iter()
        .filter(|f| f.rust_file.is_none())
        .collect();

    if !unmapped_files.is_empty() {
        out.push_str(&format!(
            "{}\n",
            "--- Unmapped Java Files ---".yellow().bold()
        ));
        for f in &unmapped_files {
            out.push_str(&format!("  {} {}\n", "[MISSING]".red(), f.java_file));
        }
        out.push('\n');
    }

    // Missing methods
    let mut missing_count = 0;
    for fm in &report.file_mappings {
        for tm in &fm.type_mappings {
            let missing: Vec<_> = tm
                .method_mappings
                .iter()
                .filter(|m| m.status == MappingStatus::MissingInRust)
                .collect();
            if !missing.is_empty() {
                for m in &missing {
                    missing_count += 1;
                    let file = fm.rust_file.as_deref().unwrap_or("(no Rust file)");
                    out.push_str(&format!(
                        "  {} {}.{}({} params) → {} [in {}]\n",
                        "[MISSING]".red(),
                        tm.java_type,
                        m.java_method,
                        m.java_params,
                        "no Rust counterpart".dimmed(),
                        file,
                    ));
                }
            }
        }
    }

    if missing_count > 0 {
        out.push('\n');
    }

    // Extra Rust methods
    let mut extra_count = 0;
    for fm in &report.file_mappings {
        for tm in &fm.type_mappings {
            let extra: Vec<_> = tm
                .method_mappings
                .iter()
                .filter(|m| m.status == MappingStatus::ExtraInRust)
                .collect();
            for m in &extra {
                extra_count += 1;
                out.push_str(&format!(
                    "  {} {} in {}\n",
                    "[EXTRA]".cyan(),
                    m.rust_method.as_deref().unwrap_or("?"),
                    fm.rust_file.as_deref().unwrap_or("?"),
                ));
            }
        }
    }

    if extra_count > 0 {
        out.push('\n');
    }

    // Summary
    let s = &report.summary;
    out.push_str(&format!("{}\n", "=== Summary ===".bold()));
    out.push_str(&format!(
        "  Files: {} Java → {} mapped, {} unmapped, {} ignored\n",
        s.total_java_files, s.mapped_files, s.unmapped_files, s.ignored_files
    ));
    out.push_str(&format!(
        "  Types: {} Java → {} matched\n",
        s.total_java_types, s.matched_types
    ));
    let resolved = s.matched_methods
        + s.field_access_methods
        + s.constructor_overloads
        + s.method_overloads
        + s.standard_trait_impls;
    out.push_str(&format!(
        "  Methods: {} Java → {} resolved, {} missing\n",
        s.total_java_methods, resolved, s.missing_methods
    ));
    out.push_str(&format!(
        "    resolved: {} matched, {} field-access, {} overloads, {} ctor-overloads, {} trait-impls\n",
        s.matched_methods,
        s.field_access_methods,
        s.method_overloads,
        s.constructor_overloads,
        s.standard_trait_impls
    ));
    out.push_str(&format!(
        "  Rust-only: {} extra, {} Rust-specific (ignored)\n",
        s.extra_rust_methods, s.rust_specific_methods
    ));

    out
}

/// Format the structural comparison report as human-readable text.
pub fn format_structural_report(report: &StructuralReport) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "{}\n\n",
        "=== Structural Comparison Report ===".bold()
    ));

    for comp in &report.comparisons {
        let sim_color = if comp.similarity >= 0.8 {
            format!("{:.0}%", comp.similarity * 100.0).green()
        } else if comp.similarity >= 0.5 {
            format!("{:.0}%", comp.similarity * 100.0).yellow()
        } else {
            format!("{:.0}%", comp.similarity * 100.0).red()
        };

        out.push_str(&format!(
            "  {}.{} — similarity: {}\n",
            comp.type_name, comp.method_name, sim_color
        ));

        for diff in &comp.differences {
            out.push_str(&format!("    {} {diff}\n", "▸".dimmed()));
        }
        out.push('\n');
    }

    let s = &report.summary;
    out.push_str(&format!("{}\n", "=== Summary ===".bold()));
    out.push_str(&format!("  Compared: {} methods\n", s.total_compared));
    out.push_str(&format!(
        "  High (≥80%): {}, Medium (50-80%): {}, Low (<50%): {}\n",
        s.high_similarity, s.medium_similarity, s.low_similarity
    ));
    out.push_str(&format!(
        "  Average similarity: {:.1}%\n",
        s.avg_similarity * 100.0
    ));

    out
}

/// Format the constants comparison report as human-readable text.
pub fn format_constants_report(report: &ConstantsReport) -> String {
    let mut out = String::new();

    out.push_str(&format!(
        "{}\n\n",
        "=== Constants Comparison Report ===".bold()
    ));

    for comp in &report.comparisons {
        out.push_str(&format!(
            "  {}.{}\n",
            comp.type_name.bold(),
            comp.method_name
        ));

        for lit in &comp.missing_in_rust {
            out.push_str(&format!(
                "    {} {} ({})\n",
                "[MISSING in Rust]".red(),
                lit.value,
                lit.kind
            ));
        }
        for lit in &comp.extra_in_rust {
            out.push_str(&format!(
                "    {} {} ({})\n",
                "[EXTRA in Rust]".cyan(),
                lit.value,
                lit.kind
            ));
        }
        out.push('\n');
    }

    let s = &report.summary;
    out.push_str(&format!("{}\n", "=== Summary ===".bold()));
    out.push_str(&format!("  Compared: {} methods\n", s.total_compared));
    out.push_str(&format!("  Methods with diffs: {}\n", s.methods_with_diffs));
    out.push_str(&format!(
        "  Missing in Rust: {}, Extra in Rust: {}\n",
        s.total_missing, s.total_extra
    ));

    out
}
