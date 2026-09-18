use std::collections::HashSet;
use std::fmt;
use std::path::Path;

use crate::analysis::complexity::SyntaxAnalyzer;
use crate::analysis::dependencies::DependencyEdge;
use tree_sitter::{Language, Node, Parser, Tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Tsx,
}

impl SupportedLanguage {
    fn language(self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
        }
    }
}

impl fmt::Display for SupportedLanguage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::JavaScript => "JavaScript",
            Self::TypeScript => "TypeScript",
            Self::Tsx => "TSX",
        };

        formatter.write_str(name)
    }
}

#[derive(Debug)]
pub struct ParsedSource {
    pub language: SupportedLanguage,
    pub tree: Tree,
}

#[derive(Debug, thiserror::Error)]
pub enum ParserError {
    #[error("failed to configure Tree-sitter parser")]
    Configuration,

    #[error("failed to parse {language} source")]
    ParseFailed { language: SupportedLanguage },
}

pub fn parse(source: &str, language: SupportedLanguage) -> Result<ParsedSource, ParserError> {
    let mut parser = Parser::new();

    parser
        .set_language(&language.language())
        .map_err(|_| ParserError::Configuration)?;

    let tree = parser
        .parse(source, None)
        .ok_or(ParserError::ParseFailed { language })?;

    Ok(ParsedSource { language, tree })
}

pub fn language_from_path(path: &str) -> Option<SupportedLanguage> {
    let extension = path.rsplit('.').next()?.to_ascii_lowercase();

    match extension.as_str() {
        "rs" => Some(SupportedLanguage::Rust),
        "py" => Some(SupportedLanguage::Python),
        "js" | "mjs" | "cjs" => Some(SupportedLanguage::JavaScript),
        "ts" => Some(SupportedLanguage::TypeScript),
        "tsx" => Some(SupportedLanguage::Tsx),
        _ => None,
    }
}

pub fn resolve_dependency_path(
    reference: &str,
    source_file: &str,
    files: &HashSet<String>,
) -> Option<String> {
    let clean = clean_reference(reference);
    let source_dir = Path::new(source_file).parent().unwrap_or(Path::new(""));
    let source_dir_str = source_dir.to_string_lossy().replace('\\', "/");

    let mut candidates: Vec<String> = Vec::new();

    for variant in path_variants(&clean) {
        if files.contains(&variant) {
            candidates.push(variant);
        }
    }

    for variant in path_variants(&clean) {
        let joined = format!("{}/{}", source_dir_str, variant);
        if files.contains(&joined) {
            candidates.push(joined);
        }
    }

    if !candidates.is_empty() {
        return Some(candidates[0].clone());
    }

    for variant in path_variants(&clean) {
        for extension in ["py", "rs", "js", "ts", "tsx", "jsx", "mjs", "cjs"] {
            let with_ext = format!("{}.{}", variant, extension);
            let joined = format!("{}/{}", source_dir_str, with_ext);
            let relative = format!(
                "{}/{}",
                source_file.rsplit('/').next().unwrap_or(""),
                with_ext
            );
            if files.contains(&joined) {
                return Some(joined);
            }
            if files.contains(&relative) {
                return Some(relative);
            }
        }
    }

    None
}

fn clean_reference(reference: &str) -> String {
    reference
        .trim()
        .trim_start_matches("use ")
        .trim_start_matches("import")
        .trim()
        .replace(';', "")
        .trim()
        .to_owned()
}

fn path_variants(clean: &str) -> Vec<String> {
    let mut variants = Vec::new();

    let normalized = clean
        .replace("::", "/")
        .replace(".", "/")
        .trim_matches('/')
        .to_owned();

    if normalized.contains('/') {
        let full = normalized.clone();
        variants.push(full.clone());
        variants.push(
            normalized
                .strip_prefix("crate/")
                .unwrap_or(&full)
                .to_owned(),
        );
        variants.push(normalized.strip_prefix("src/").unwrap_or(&full).to_owned());
    } else {
        variants.push(normalized.clone());
    }

    variants
}

pub struct TreeSitterAnalyzer {
    language: SupportedLanguage,
}

impl TreeSitterAnalyzer {
    pub fn new(language: SupportedLanguage) -> Self {
        Self { language }
    }

    fn is_kind(&self, node: Node<'_>, kinds: &[&str]) -> bool {
        kinds.contains(&node.kind())
    }
}

impl SyntaxAnalyzer for TreeSitterAnalyzer {
    fn is_function(&self, node: Node<'_>) -> bool {
        match self.language {
            SupportedLanguage::Rust => self.is_kind(node, &["function_item", "closure_expression"]),

            SupportedLanguage::Python => self.is_kind(node, &["function_definition", "lambda"]),

            SupportedLanguage::JavaScript
            | SupportedLanguage::TypeScript
            | SupportedLanguage::Tsx => self.is_kind(
                node,
                &[
                    "function_declaration",
                    "function_expression",
                    "arrow_function",
                    "method_definition",
                ],
            ),
        }
    }

    fn is_decision(&self, node: Node<'_>) -> bool {
        match self.language {
            SupportedLanguage::Rust => self.is_kind(
                node,
                &[
                    "if_expression",
                    "match_expression",
                    "while_expression",
                    "for_expression",
                    "loop_expression",
                ],
            ),

            SupportedLanguage::Python => self.is_kind(
                node,
                &[
                    "if_statement",
                    "elif_clause",
                    "for_statement",
                    "while_statement",
                    "except_clause",
                ],
            ),

            SupportedLanguage::JavaScript
            | SupportedLanguage::TypeScript
            | SupportedLanguage::Tsx => self.is_kind(
                node,
                &[
                    "if_statement",
                    "for_statement",
                    "for_in_statement",
                    "while_statement",
                    "do_statement",
                    "switch_case",
                    "catch_clause",
                    "ternary_expression",
                ],
            ),
        }
    }

    fn is_nesting(&self, node: Node<'_>) -> bool {
        match self.language {
            SupportedLanguage::Rust => self.is_kind(
                node,
                &[
                    "block",
                    "if_expression",
                    "match_expression",
                    "for_expression",
                    "while_expression",
                    "loop_expression",
                ],
            ),

            SupportedLanguage::Python => self.is_kind(
                node,
                &[
                    "block",
                    "if_statement",
                    "for_statement",
                    "while_statement",
                    "try_statement",
                    "with_statement",
                ],
            ),

            SupportedLanguage::JavaScript
            | SupportedLanguage::TypeScript
            | SupportedLanguage::Tsx => self.is_kind(
                node,
                &[
                    "statement_block",
                    "if_statement",
                    "for_statement",
                    "for_in_statement",
                    "while_statement",
                    "do_statement",
                    "switch_statement",
                    "try_statement",
                ],
            ),
        }
    }
}

pub fn extract_dependency_references(source: &str, parsed: &ParsedSource) -> Vec<String> {
    let mut references = Vec::new();

    collect_dependency_references(
        parsed.tree.root_node(),
        source.as_bytes(),
        parsed.language,
        &mut references,
    );

    references.sort();
    references.dedup();
    references
}

fn collect_dependency_references(
    node: Node<'_>,
    source: &[u8],
    language: SupportedLanguage,
    references: &mut Vec<String>,
) {
    match language {
        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript | SupportedLanguage::Tsx => {
            collect_es_dependency_references(node, source, references);
        }
        _ => {
            let is_dependency_node = match language {
                SupportedLanguage::Rust => matches!(
                    node.kind(),
                    "use_declaration" | "extern_crate_declaration" | "mod_item"
                ),
                SupportedLanguage::Python => {
                    matches!(node.kind(), "import_statement" | "import_from_statement")
                }
                _ => false,
            };

            if is_dependency_node && let Ok(text) = node.utf8_text(source) {
                let reference = normalize_dependency_reference(text, language);

                if !reference.is_empty() {
                    references.push(reference);
                }
            }

            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                collect_dependency_references(child, source, language, references);
            }
        }
    }
}

fn collect_es_dependency_references(node: Node<'_>, source: &[u8], references: &mut Vec<String>) {
    let is_import = matches!(
        node.kind(),
        "import_statement" | "export_statement" | "import_clause"
    );

    if is_import
        && let Some(specifier) = node.child_by_field_name("source")
        && let Ok(text) = specifier.utf8_text(source)
    {
        let reference = text.trim_matches(['\'', '"']).trim().to_owned();
        if !reference.is_empty() {
            references.push(reference);
        }
    }

    let is_require = node.kind() == "call_expression" && {
        let args = node.child_by_field_name("arguments");
        let func = node.child_by_field_name("function");
        let func_text = func.and_then(|f| f.utf8_text(source).ok()).unwrap_or("");
        func_text.trim() == "require"
            && args
                .and_then(|a| a.utf8_text(source).ok())
                .map(|arg| arg.contains('\''))
                .unwrap_or(false)
    };

    if is_require
        && let Some(args) = node.child_by_field_name("arguments")
        && let Ok(text) = args.utf8_text(source)
    {
        let reference = text
            .trim()
            .trim_start_matches('(')
            .trim_end_matches(')')
            .trim()
            .trim_matches(['\'', '"'])
            .to_owned();
        if !reference.is_empty() {
            references.push(reference);
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_es_dependency_references(child, source, references);
    }
}

fn normalize_dependency_reference(text: &str, language: SupportedLanguage) -> String {
    let text = text.trim();

    match language {
        SupportedLanguage::Rust => {
            if let Some(rest) = text.strip_prefix("use ") {
                return rest.trim_end_matches(';').trim().to_owned();
            }

            if let Some(rest) = text.strip_prefix("mod ") {
                return rest.trim_end_matches(';').trim().to_owned();
            }

            text.trim_end_matches(';').to_owned()
        }

        SupportedLanguage::Python => text.lines().next().unwrap_or(text).trim().to_owned(),

        SupportedLanguage::JavaScript | SupportedLanguage::TypeScript | SupportedLanguage::Tsx => {
            text.trim_end_matches(';').trim().to_owned()
        }
    }
}

pub fn dependency_edges(
    source_path: &str,
    references: &[String],
    files: &HashSet<String>,
) -> Vec<DependencyEdge> {
    references
        .iter()
        .map(|reference| {
            let target = resolve_dependency_path(reference, source_path, files)
                .unwrap_or_else(|| format!("${reference}"));
            DependencyEdge {
                source: source_path.to_owned(),
                target,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_supported_languages() {
        assert_eq!(
            language_from_path("src/main.rs"),
            Some(SupportedLanguage::Rust)
        );

        assert_eq!(
            language_from_path("app/main.py"),
            Some(SupportedLanguage::Python)
        );

        assert_eq!(
            language_from_path("src/app.js"),
            Some(SupportedLanguage::JavaScript)
        );

        assert_eq!(
            language_from_path("src/app.ts"),
            Some(SupportedLanguage::TypeScript)
        );

        assert_eq!(
            language_from_path("src/App.tsx"),
            Some(SupportedLanguage::Tsx)
        );
    }

    #[test]
    fn unsupported_extension_returns_none() {
        assert_eq!(language_from_path("README.md"), None);
        assert_eq!(language_from_path("image.png"), None);
    }

    #[test]
    fn analyzes_rust_complexity() {
        let source = r#"
            fn calculate(value: i32) -> i32 {
                if value > 10 {
                    value * 2
                } else {
                    value + 2
                }
            }
        "#;

        let parsed = parse(source, SupportedLanguage::Rust).expect("Rust source should parse");

        let analyzer = TreeSitterAnalyzer::new(SupportedLanguage::Rust);

        let result = crate::analysis::complexity::analyze(parsed.tree.root_node(), &analyzer);

        assert_eq!(result.functions, 1);
        assert!(result.cyclomatic >= 2);
        assert!(result.max_nesting_depth > 0);
    }

    #[test]
    fn extracts_rust_dependencies() {
        let source = r#"
            use crate::github::client::GithubClient;
            use crate::github::files::RepositoryFile;

            fn main() {}
        "#;

        let parsed = parse(source, SupportedLanguage::Rust).expect("Rust source should parse");

        let references = extract_dependency_references(source, &parsed);

        assert_eq!(references.len(), 2);
        assert!(
            references
                .iter()
                .any(|r| { r.contains("crate::github::client::GithubClient") })
        );
        assert!(
            references
                .iter()
                .any(|r| { r.contains("crate::github::files::RepositoryFile") })
        );
    }

    #[test]
    fn parses_rust_source() {
        let source = r#"
            fn main() {
                println!("hello");
            }
        "#;

        let parsed = parse(source, SupportedLanguage::Rust).expect("Rust source should parse");

        assert_eq!(parsed.language, SupportedLanguage::Rust);
        assert!(!parsed.tree.root_node().has_error());
    }

    #[test]
    fn resolves_dependency_to_matching_file() {
        let files = [
            "src/github/client.rs".to_owned(),
            "src/github/mod.rs".to_owned(),
            "src/main.rs".to_owned(),
        ]
        .into_iter()
        .collect();

        let resolved = resolve_dependency_path("crate::github::client", "src/main.rs", &files);

        assert_eq!(resolved, Some("src/github/client.rs".to_owned()));
    }

    #[test]
    fn unresolved_dependency_returns_none() {
        let files = ["src/main.rs".to_owned()].into_iter().collect();

        let resolved = resolve_dependency_path("std::collections::HashMap", "src/main.rs", &files);

        assert_eq!(resolved, None);
    }

    #[test]
    fn parses_python_source() {
        let source = r#"
def main():
    print("hello")
"#;

        let parsed = parse(source, SupportedLanguage::Python).expect("Python source should parse");

        assert_eq!(parsed.language, SupportedLanguage::Python);
        assert!(!parsed.tree.root_node().has_error());
    }

    #[test]
    fn parses_javascript_source() {
        let source = r#"
function main() {
    console.log("hello");
}
"#;

        let parsed =
            parse(source, SupportedLanguage::JavaScript).expect("JavaScript source should parse");

        assert_eq!(parsed.language, SupportedLanguage::JavaScript);
        assert!(!parsed.tree.root_node().has_error());
    }

    #[test]
    fn extracts_es_module_specifiers() {
        let source = r#"
import { fetchItems } from './api';
import config from "../config.json";
const yaml = require('yaml');
"#;

        let parsed =
            parse(source, SupportedLanguage::TypeScript).expect("TypeScript source should parse");

        let references = extract_dependency_references(source, &parsed);

        assert!(
            references.iter().any(|r| r == "./api"),
            "got {references:?}"
        );
        assert!(
            references.iter().any(|r| r == "../config.json"),
            "got {references:?}"
        );
        assert!(references.iter().any(|r| r == "yaml"), "got {references:?}");
    }

    #[test]
    fn parses_typescript_source() {
        let source = r#"
function greet(name: string): string {
    return `Hello ${name}`;
}
"#;

        let parsed =
            parse(source, SupportedLanguage::TypeScript).expect("TypeScript source should parse");

        assert_eq!(parsed.language, SupportedLanguage::TypeScript);
        assert!(!parsed.tree.root_node().has_error());
    }
}
