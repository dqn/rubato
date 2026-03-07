// Translated from EditableTableView.java

/// TableView extension with add/remove/moveUp/moveDown operations.
/// Java: EditableTableView<T> extends TableView<T>
/// In Rust, this wraps a Vec<T> with selected indices tracking.
/// Renders control buttons (add/remove/up/down) via egui.
#[derive(Clone, Debug)]
pub struct EditableTableView<T: Clone> {
    /// The items in the table (Java: getItems())
    pub items: Vec<T>,
    /// The currently selected indices (Java: getSelectionModel().getSelectedIndices())
    pub selected_indices: Vec<usize>,
}

impl<T: Clone> EditableTableView<T> {
    /// Creates a new empty EditableTableView.
    pub fn new() -> Self {
        EditableTableView {
            items: Vec::new(),
            selected_indices: Vec::new(),
        }
    }

    /// Adds an item to the table.
    /// Java: addItem(T item) { getItems().add(item); }
    pub fn add_item(&mut self, item: T) {
        self.items.push(item);
    }

    /// Removes the selected items from the table.
    /// Java: removeSelectedItems() { getItems().removeAll(getSelectionModel().getSelectedItems()); }
    pub fn remove_selected_items(&mut self) {
        // Remove in reverse order to preserve indices
        let mut indices = self.selected_indices.clone();
        indices.sort();
        indices.dedup();
        for &index in indices.iter().rev() {
            if index < self.items.len() {
                self.items.remove(index);
            }
        }
        self.selected_indices.clear();
    }

    /// Moves selected items up by one position.
    /// Java: moveSelectedItemsUp()
    /// Preserves the exact block-swap algorithm from Java.
    pub fn move_selected_items_up(&mut self) {
        // Java: int[] indices = getSelectionModel().getSelectedIndices().stream().mapToInt(i -> i).toArray();
        let mut indices: Vec<i32> = self.selected_indices.iter().map(|&i| i as i32).collect();
        if indices.is_empty() {
            return;
        }
        // Java: Arrays.sort(indices);
        indices.sort();

        // Java: int lastBlockIndex = 0;
        let mut last_block_index: usize = 0;
        // Java: for (int i = 1; i <= indices.length; i++)
        let len = indices.len();
        for i in 1..=len {
            // Java: if (i == indices.length || indices[i] > indices[i-1] + 1)
            if i == len || indices[i] > indices[i - 1] + 1 {
                // Java: if (indices[lastBlockIndex] > 0)
                if indices[last_block_index] > 0 {
                    // Java: T item = getItems().get(indices[lastBlockIndex] - 1);
                    let remove_idx = indices[last_block_index] as usize - 1;
                    let item = self.items.remove(remove_idx);
                    // Java: getItems().add(indices[i-1], item);
                    let insert_idx = indices[i - 1] as usize;
                    self.items.insert(insert_idx, item);
                }
                // Java: lastBlockIndex = i;
                last_block_index = i;
            }
        }

        // Java: for (int i = 0; i < indices.length; i++) { indices[i] -= 1; }
        for idx in indices.iter_mut() {
            *idx -= 1;
        }
        // Java: if (indices[0] == -1)
        if indices[0] == -1 {
            indices[0] = 0;
            // Java: int j = 1;
            let mut j: usize = 1;
            // Java: while (j < indices.length && indices[j] == indices[j-1])
            while j < indices.len() && indices[j] == indices[j - 1] {
                // Java: indices[j] += 1;
                indices[j] += 1;
                // Java: j++;
                j += 1;
            }
        }
        // Java: getSelectionModel().selectIndices(-1, indices);
        // Update selected indices
        self.selected_indices = indices.iter().map(|&i| i as usize).collect();
    }

    /// Moves selected items down by one position.
    /// Java: moveSelectedItemsDown()
    /// Preserves the exact block-swap algorithm from Java.
    pub fn move_selected_items_down(&mut self) {
        // Java: int[] indices = getSelectionModel().getSelectedIndices().stream().mapToInt(i -> i).toArray();
        let mut indices: Vec<i32> = self.selected_indices.iter().map(|&i| i as i32).collect();
        if indices.is_empty() {
            return;
        }
        // Java: Arrays.sort(indices);
        indices.sort();
        // Java: final int numItems = getItems().size();
        let num_items = self.items.len() as i32;

        // Java: int lastBlockIndex = indices.length - 1;
        let len = indices.len();
        let mut last_block_index: i32 = len as i32 - 1;
        // Java: for (int i = indices.length - 2; i >= -1; i--)
        let mut i: i32 = len as i32 - 2;
        while i >= -1 {
            // Java: if (i == -1 || indices[i] < indices[i+1] - 1)
            if i == -1 || indices[i as usize] < indices[(i + 1) as usize] - 1 {
                // Java: if (indices[lastBlockIndex] < numItems - 1)
                if indices[last_block_index as usize] < num_items - 1 {
                    // Java: T item = getItems().get(indices[lastBlockIndex] + 1);
                    let remove_idx = indices[last_block_index as usize] as usize + 1;
                    let item = self.items.remove(remove_idx);
                    // Java: getItems().add(indices[i+1], item);
                    let insert_idx = indices[(i + 1) as usize] as usize;
                    self.items.insert(insert_idx, item);
                }
                // Java: lastBlockIndex = i;
                last_block_index = i;
            }
            // Java: i--
            i -= 1;
        }

        // Java: for (int i = 0; i < indices.length; i++) { indices[i] += 1; }
        for idx in indices.iter_mut() {
            *idx += 1;
        }
        // Java: if (indices[indices.length - 1] == numItems)
        if indices[indices.len() - 1] == num_items {
            let last = indices.len() - 1;
            indices[last] = num_items - 1;
            // Java: int j = indices.length - 2;
            let mut j: i32 = last as i32 - 1;
            // Java: while (j >= 0 && indices[j] == indices[j+1])
            while j >= 0 && indices[j as usize] == indices[(j + 1) as usize] {
                // Java: indices[j] -= 1;
                indices[j as usize] -= 1;
                // Java: j--;
                j -= 1;
            }
        }
        // Java: getSelectionModel().selectIndices(-1, indices);
        // Update selected indices
        self.selected_indices = indices.iter().map(|&i| i as usize).collect();
    }

    /// Renders the add/remove/move-up/move-down control buttons via egui.
    /// `add_item_fn` is called when the user clicks "Add" and should return the new item.
    /// Returns true if the items list was modified.
    pub fn show_controls(
        &mut self,
        ui: &mut egui::Ui,
        add_item_fn: Option<&dyn Fn() -> T>,
    ) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            if let Some(f) = add_item_fn
                && ui.button("Add").clicked()
            {
                self.add_item(f());
                changed = true;
            }
            if ui.button("Remove").clicked() && !self.selected_indices.is_empty() {
                self.remove_selected_items();
                changed = true;
            }
            if ui.button("Up").clicked() && !self.selected_indices.is_empty() {
                self.move_selected_items_up();
                changed = true;
            }
            if ui.button("Down").clicked() && !self.selected_indices.is_empty() {
                self.move_selected_items_down();
                changed = true;
            }
        });
        changed
    }
}

impl<T: Clone> Default for EditableTableView<T> {
    fn default() -> Self {
        Self::new()
    }
}
