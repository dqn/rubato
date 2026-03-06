// Translated from SpinnerCell.java

use crate::util::controller_config_view_model::ControllerConfigViewModel;
use crate::util::numeric_spinner::{NumericSpinner, NumericValue};

/// TableCell with NumericSpinner for ControllerConfigViewModel.
/// Java: SpinnerCell extends TableCell<ControllerConfigViewModel, Integer>
/// Contains a NumericSpinner that updates the cell's property on value change.
/// Renders the spinner inline via egui.
#[derive(Clone, Debug)]
pub struct SpinnerCell {
    pub spinner: NumericSpinner,
    /// The current item value (corresponds to Java's updateItem value)
    pub current_item: Option<i32>,
    /// Whether the cell is empty
    pub empty: bool,
}

impl SpinnerCell {
    /// Creates a new SpinnerCell.
    /// Java: SpinnerCell(int min, int max, int initial, int step)
    /// - Creates NumericSpinner with IntegerSpinnerValueFactory(min, max, initial, step)
    /// - Sets editable to true
    /// - Adds valueProperty listener that updates the cell's ControllerConfigViewModel property
    pub fn new(min: i32, max: i32, initial: i32, step: i32) -> Self {
        let mut spinner = NumericSpinner::new();
        spinner.set_value_factory_values(
            NumericValue::Integer(min),
            NumericValue::Integer(max),
            NumericValue::Integer(initial),
            NumericValue::Integer(step),
        );
        spinner.set_editable(true);
        // Java: spinner.valueProperty().addListener((o, oldValue, newValue) -> {
        //     WritableValue<Integer> cellProperty = (WritableValue<Integer>)
        //         getTableColumn().getCellObservableValue((ControllerConfigViewModel)getTableRow().getItem());
        //     cellProperty.setValue(newValue);
        // });
        // In Rust, the value change callback is handled during egui rendering.

        SpinnerCell {
            spinner,
            current_item: None,
            empty: true,
        }
    }

    /// Updates the cell item.
    /// Java: updateItem(Integer item, boolean empty)
    /// - Calls super.updateItem(item, empty)
    /// - If empty, sets graphic to null
    /// - Otherwise, sets spinner value to item and sets graphic to spinner
    pub fn update_item(&mut self, item: Option<i32>, empty: bool) {
        self.empty = empty;
        self.current_item = item;
        if !empty && let Some(value) = item {
            self.spinner.set_value(NumericValue::Integer(value));
        }
        // Java: setGraphic(spinner) — deferred to egui rendering
        // Java: if empty, setGraphic(null) — deferred to egui rendering
    }

    /// Gets the current spinner value as integer.
    pub fn value(&self) -> Option<i32> {
        if let NumericValue::Integer(v) = &self.spinner.value {
            Some(*v)
        } else {
            None
        }
    }

    /// Applies the current spinner value to a ControllerConfigViewModel.
    /// This replaces the Java valueProperty listener callback.
    pub fn apply_to_view_model(
        &self,
        view_model: &mut ControllerConfigViewModel,
        column_name: &str,
    ) {
        if let Some(value) = self.value() {
            match column_name {
                "analogScratchThreshold" => view_model.analog_scratch_threshold = value,
                "analogScratchMode" => view_model.analog_scratch_mode = value,
                _ => {}
            }
        }
    }

    /// Renders the spinner cell via egui. Returns true if the value changed.
    pub fn show(&mut self, ui: &mut egui::Ui) -> bool {
        if self.empty {
            return false;
        }
        self.spinner.show(ui)
    }
}
