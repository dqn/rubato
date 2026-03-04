// Translated from NumericSpinner.java

/// Numeric value type for the spinner (Integer or Double in Java)
#[derive(Clone, Debug, PartialEq)]
pub enum NumericValue {
    Integer(i32),
    Double(f64),
}

/// JavaFX Spinner extension for numeric values.
/// In Java, this extends Spinner<T> and handles focus-loss commit,
/// value factory configuration, and value clamping.
/// Renders as an egui DragValue with step buttons.
#[derive(Clone, Debug)]
pub struct NumericSpinner {
    pub value: NumericValue,
    pub min: NumericValue,
    pub max: NumericValue,
    pub amount_to_step_by: NumericValue,
    pub editable: bool,
    pub editor_text: String,
}

impl NumericSpinner {
    /// Creates a new NumericSpinner with default integer values.
    /// Java: NumericSpinner() constructor registers focusedProperty listener
    /// that calls commitEditorText on focus loss.
    pub fn new() -> Self {
        NumericSpinner {
            value: NumericValue::Integer(0),
            min: NumericValue::Integer(0),
            max: NumericValue::Integer(100),
            amount_to_step_by: NumericValue::Integer(1),
            editable: false,
            editor_text: String::new(),
        }
    }

    /// Sets the spinner's value factory values.
    /// Java: setValueFactoryValues(T min, T max, T initialValue, T amountToStepBy)
    /// Checks if min is Integer or Double and casts accordingly.
    pub fn set_value_factory_values(
        &mut self,
        min: NumericValue,
        max: NumericValue,
        initial_value: NumericValue,
        amount_to_step_by: NumericValue,
    ) {
        match (&min, &max, &initial_value, &amount_to_step_by) {
            (
                NumericValue::Integer(min_val),
                NumericValue::Integer(max_val),
                NumericValue::Integer(init_val),
                NumericValue::Integer(step_val),
            ) => {
                self.min = NumericValue::Integer(*min_val);
                self.max = NumericValue::Integer(*max_val);
                self.value = NumericValue::Integer(*init_val);
                self.amount_to_step_by = NumericValue::Integer(*step_val);
            }
            (
                NumericValue::Double(min_val),
                NumericValue::Double(max_val),
                NumericValue::Double(init_val),
                NumericValue::Double(step_val),
            ) => {
                self.min = NumericValue::Double(*min_val);
                self.max = NumericValue::Double(*max_val);
                self.value = NumericValue::Double(*init_val);
                self.amount_to_step_by = NumericValue::Double(*step_val);
            }
            _ => {
                // Mixed types — should not happen in normal usage
            }
        }
    }

    /// Sets whether the spinner is editable.
    pub fn set_editable(&mut self, editable: bool) {
        self.editable = editable;
    }

    /// Returns whether the spinner is editable.
    pub fn is_editable(&self) -> bool {
        self.editable
    }

    /// Commits the editor text, parsing and clamping the value.
    /// Java: commitEditorText(Spinner<T> spinner)
    /// - If not editable, returns immediately
    /// - Gets editor text, converts via StringConverter
    /// - Calls setValue which clamps to min/max
    /// - On parse error, resets editor text to current value
    pub fn commit_editor_text(&mut self) {
        if !self.editable {
            return;
        }
        let text = &self.editor_text;
        match &self.value {
            NumericValue::Integer(_) => {
                match text.parse::<i32>() {
                    Ok(parsed) => {
                        self.set_value_integer(parsed);
                    }
                    Err(_) => {
                        // Reset editor text to current value (Java: spinner.getEditor().setText(...))
                        if let NumericValue::Integer(v) = self.value {
                            self.editor_text = v.to_string();
                        }
                    }
                }
            }
            NumericValue::Double(_) => {
                match text.parse::<f64>() {
                    Ok(parsed) => {
                        self.set_value_double(parsed);
                    }
                    Err(_) => {
                        // Reset editor text to current value
                        if let NumericValue::Double(v) = self.value {
                            self.editor_text = v.to_string();
                        }
                    }
                }
            }
        }
    }

    /// Sets an integer value, clamping to min/max.
    /// Java: setValue(IntegerSpinnerValueFactory, Integer)
    /// valueFactory.setValue(Math.min(Math.max(value, min), max))
    fn set_value_integer(&mut self, value: i32) {
        if let (NumericValue::Integer(min), NumericValue::Integer(max)) = (&self.min, &self.max) {
            let clamped = value.max(*min).min(*max);
            self.value = NumericValue::Integer(clamped);
        }
    }

    /// Sets a double value, clamping to min/max.
    /// Java: setValue(DoubleSpinnerValueFactory, Double)
    /// valueFactory.setValue(Math.min(Math.max(value, min), max))
    fn set_value_double(&mut self, value: f64) {
        if let (NumericValue::Double(min), NumericValue::Double(max)) = (&self.min, &self.max) {
            let clamped = value.max(*min).min(*max);
            self.value = NumericValue::Double(clamped);
        }
    }

    /// Gets the current value.
    pub fn get_value(&self) -> &NumericValue {
        &self.value
    }

    /// Sets the value directly (used by external callers).
    /// Java: getValueFactory().setValue(item)
    pub fn set_value(&mut self, value: NumericValue) {
        match &value {
            NumericValue::Integer(v) => self.set_value_integer(*v),
            NumericValue::Double(v) => self.set_value_double(*v),
        }
    }

    /// Gets the editor text.
    pub fn get_editor_text(&self) -> &str {
        &self.editor_text
    }

    /// Sets the editor text.
    pub fn set_editor_text(&mut self, text: String) {
        self.editor_text = text;
    }

    /// Renders the spinner as an egui DragValue widget.
    /// Returns true if the value changed.
    pub fn show(&mut self, ui: &mut egui::Ui) -> bool {
        match (
            &mut self.value,
            &self.min,
            &self.max,
            &self.amount_to_step_by,
        ) {
            (
                NumericValue::Integer(val),
                NumericValue::Integer(min),
                NumericValue::Integer(max),
                NumericValue::Integer(step),
            ) => {
                let old = *val;
                ui.add(egui::DragValue::new(val).range(*min..=*max).speed(*step));
                *val != old
            }
            (
                NumericValue::Double(val),
                NumericValue::Double(min),
                NumericValue::Double(max),
                NumericValue::Double(step),
            ) => {
                let old = *val;
                ui.add(egui::DragValue::new(val).range(*min..=*max).speed(*step));
                *val != old
            }
            _ => false,
        }
    }
}

impl Default for NumericSpinner {
    fn default() -> Self {
        Self::new()
    }
}
