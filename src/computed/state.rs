// src/computed/state.rs
//! Computed field state: caching and dependency graph.
//!
//! This module holds the internal state necessary to track which fields are
//! computed, their dependencies, and cached computed values. It is used by the
//! editor to avoid unnecessary recomputation and to present computed fields as
//! read-only.

use std::collections::{HashMap, HashSet};

/// Internal state for computed field management
#[derive(Debug, Clone)]
pub struct ComputedState {
    /// Cached computed values (field_index -> computed_value)
    computed_values: HashMap<usize, String>,
    /// Field dependency graph (field_index -> depends_on_fields)
    dependencies: HashMap<usize, Vec<usize>>,
    /// Track which fields are computed (display-only)
    computed_fields: HashSet<usize>,
}

impl ComputedState {
    /// Create a new, empty computed state
    pub fn new() -> Self {
        Self {
            computed_values: HashMap::new(),
            dependencies: HashMap::new(),
            computed_fields: HashSet::new(),
        }
    }

    /// Register a field as computed with its dependencies
    ///
    /// - `field_index`: the field that is computed (display-only)
    /// - `dependencies`: indices of fields this computed field depends on
    pub fn register_computed_field(&mut self, field_index: usize, mut dependencies: Vec<usize>) {
        // Deduplicate dependencies to keep graph lean
        dependencies.sort_unstable();
        dependencies.dedup();

        self.computed_fields.insert(field_index);
        self.dependencies.insert(field_index, dependencies);
    }

    /// Check if a field is computed (read-only, skip editing/navigation)
    pub fn is_computed_field(&self, field_index: usize) -> bool {
        self.computed_fields.contains(&field_index)
    }

    /// Get cached computed value for a field, if available
    pub fn get_computed_value(&self, field_index: usize) -> Option<&String> {
        self.computed_values.get(&field_index)
    }

    /// Update cached computed value for a field
    pub fn set_computed_value(&mut self, field_index: usize, value: String) {
        self.computed_values.insert(field_index, value);
    }

    /// Get fields that should be recomputed when `changed_field` changed
    ///
    /// This scans the dependency graph and returns all computed fields
    /// that list `changed_field` as a dependency.
    pub fn fields_to_recompute(&self, changed_field: usize) -> Vec<usize> {
        self.dependencies
            .iter()
            .filter_map(|(field, deps)| {
                if deps.contains(&changed_field) {
                    Some(*field)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Iterator over all computed field indices
    pub fn computed_fields(&self) -> impl Iterator<Item = usize> + '_ {
        self.computed_fields.iter().copied()
    }
}

impl Default for ComputedState {
    fn default() -> Self {
        Self::new()
    }
}
