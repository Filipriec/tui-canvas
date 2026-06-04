// src/editor/features/computed.rs

use crate::computed::{ComputedContext, ComputedProvider, ComputedState};
use crate::editor::FormEditor;
use crate::DataProvider;

impl<D: DataProvider> FormEditor<D> {
    #[cfg(feature = "computed")]
    pub fn register_computed_provider<C>(&mut self, provider: &C)
    where
        C: ComputedProvider,
    {
        self.ui_state.computed = Some(ComputedState::new());

        let field_count = self.data_provider.field_count();
        for field_index in 0..field_count {
            if provider.handles_field(field_index) {
                let deps = provider.field_dependencies(field_index);
                if let Some(computed_state) = &mut self.ui_state.computed {
                    computed_state.register_computed_field(field_index, deps);
                }
            }
        }
    }

    #[cfg(feature = "computed")]
    pub fn set_computed_provider<C>(&mut self, mut provider: C)
    where
        C: ComputedProvider,
    {
        self.register_computed_provider(&provider);
        self.recompute_all_fields(&mut provider);
    }

    #[cfg(feature = "computed")]
    pub fn recompute_fields<C>(&mut self, provider: &mut C, field_indices: &[usize])
    where
        C: ComputedProvider,
    {
        if let Some(computed_state) = &mut self.ui_state.computed {
            let field_values: Vec<String> = (0..self.data_provider.field_count())
                .map(|i| {
                    if computed_state.is_computed_field(i) {
                        computed_state
                            .get_computed_value(i)
                            .cloned()
                            .unwrap_or_default()
                    } else {
                        self.data_provider.field_value(i).to_string()
                    }
                })
                .collect();

            let field_refs: Vec<&str> = field_values.iter().map(|s| s.as_str()).collect();

            for &field_index in field_indices {
                if provider.handles_field(field_index) {
                    let context = ComputedContext {
                        field_values: &field_refs,
                        target_field: field_index,
                        current_field: Some(self.ui_state.current_field),
                    };

                    let computed_value = provider.compute_field(context);
                    computed_state.set_computed_value(field_index, computed_value);
                }
            }
        }
    }

    #[cfg(feature = "computed")]
    pub fn recompute_all_fields<C>(&mut self, provider: &mut C)
    where
        C: ComputedProvider,
    {
        if let Some(computed_state) = &self.ui_state.computed {
            let computed_fields: Vec<usize> = computed_state.computed_fields().collect();
            self.recompute_fields(provider, &computed_fields);
        }
    }

    #[cfg(feature = "computed")]
    pub fn on_field_changed<C>(&mut self, provider: &mut C, changed_field: usize)
    where
        C: ComputedProvider,
    {
        if let Some(computed_state) = &self.ui_state.computed {
            let fields_to_update = computed_state.fields_to_recompute(changed_field);
            if !fields_to_update.is_empty() {
                self.recompute_fields(provider, &fields_to_update);
            }
        }
    }

    #[cfg(feature = "computed")]
    pub fn effective_field_value(&self, field_index: usize) -> String {
        if let Some(computed_state) = &self.ui_state.computed {
            if let Some(computed_value) = computed_state.get_computed_value(field_index) {
                return computed_value.clone();
            }
        }
        self.data_provider.field_value(field_index).to_string()
    }
}
