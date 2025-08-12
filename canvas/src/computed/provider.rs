// ================================================================================================
// COMPUTED FIELDS - Provider and Context
// ================================================================================================

/// Context information provided to computed field calculations
#[derive(Debug, Clone)]
pub struct ComputedContext<'a> {
    /// All field values in the form (index -> value)
    pub field_values: &'a [&'a str],
    /// The field index being computed
    pub target_field: usize,
    /// Current field that user is editing (if any)
    pub current_field: Option<usize>,
}

/// User implements this to provide computed field logic
pub trait ComputedProvider {
    /// Compute value for a field based on other field values.
    /// Called automatically when any field changes.
    fn compute_field(&mut self, context: ComputedContext) -> String;

    /// Check if this provider handles the given field.
    fn handles_field(&self, field_index: usize) -> bool;

    /// Get list of field dependencies for optimization.
    /// If field A depends on fields [1, 3], only recompute A when fields 1 or 3 change.
    /// Default: depend on all fields (always recompute) with a reasonable upper bound.
    fn field_dependencies(&self, field_index: usize) -> Vec<usize> {
        (0..100).collect()
    }
}
