// FILE: ./core/engine/src/validator_plugin.rs

//! Pluggable validator trait and plugin registry.
//!
//! This module allows custom validation tags to be added without modifying
//! the core validator. Plugins implement the `PluggableValidator` trait
//! and register themselves with the `PluginRegistry`.

use crate::blacklist::LanguageHint;
use crate::validator::ValidationEntry;

/// Trait implemented by pluggable validators that can handle custom CC tags.
pub trait PluggableValidator {
    /// Tags this plugin knows how to validate, e.g. ["CC-CYCLE", "CC-IMPORTS"].
    fn supported_tags(&self) -> &'static [&'static str];

    /// Perform validation for a file.
    /// 
    /// # Arguments
    /// * `code` - The source code to validate
    /// * `path` - The file path (normalized)
    /// * `language` - Language hint for the file
    fn validate(
        &self,
        code: &str,
        path: &str,
        language: LanguageHint,
    ) -> Vec<ValidationEntry>;
}

/// Registry storing a set of plugins.
///
/// The core `Validator` embeds one of these and forwards matching tags.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn PluggableValidator>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        PluginRegistry { plugins: Vec::new() }
    }

    pub fn register<P: PluggableValidator + 'static>(&mut self, plugin: P) {
        self.plugins.push(Box::new(plugin));
    }

    pub fn plugins(&self) -> &Vec<Box<dyn PluggableValidator>> {
        &self.plugins
    }

    /// Run all plugins whose `supported_tags` contain any of `requested_tags`.
    pub fn run_for_tags(
        &self,
        requested_tags: &[String],
        code: &str,
        path: &str,
        language: LanguageHint,
    ) -> Vec<ValidationEntry> {
        let mut out = Vec::new();
        for plugin in &self.plugins {
            let supported = plugin.supported_tags();
            if requested_tags.iter().any(|t| supported.contains(&t.as_str())) {
                let mut entries = plugin.validate(code, path, language);
                out.append(&mut entries);
            }
        }
        out
    }
    
    /// Check if any plugin supports the given tag
    pub fn has_plugin_for_tag(&self, tag: &str) -> bool {
        self.plugins.iter().any(|p| p.supported_tags().contains(&tag))
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestPlugin;

    impl PluggableValidator for TestPlugin {
        fn supported_tags(&self) -> &'static [&'static str] {
            &["CC-TEST"]
        }

        fn validate(
            &self,
            _code: &str,
            _path: &str,
            _language: LanguageHint,
        ) -> Vec<ValidationEntry> {
            vec![ValidationEntry {
                tag: "CC-TEST".to_string(),
                passed: true,
                message: "Test plugin validation passed".to_string(),
                path: "test.rs".to_string(),
                line: 1,
                column: 1,
                severity: crate::validator::Severity::Info,
            }]
        }
    }

    #[test]
    fn test_plugin_registry() {
        let mut registry = PluginRegistry::new();
        registry.register(TestPlugin);

        assert!(registry.has_plugin_for_tag("CC-TEST"));
        assert!(!registry.has_plugin_for_tag("CC-UNKNOWN"));

        let entries = registry.run_for_tags(
            &vec!["CC-TEST".to_string()],
            "fn main() {}",
            "test.rs",
            LanguageHint::Rust,
        );

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].tag, "CC-TEST");
    }
}
