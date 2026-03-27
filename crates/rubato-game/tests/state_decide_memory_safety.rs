// Phase 60c: Verify MainControllerRef no longer leaks memory.
//
// Previously input_processor() used Box::leak to return references.
// InputSnapshot migration removed the input processor entirely,
// so there is nothing to leak. This test validates construction is safe.

use rubato_game::state::decide::NullMainController;
use rubato_game::state::decide::main_controller_ref::MainControllerRef;

/// MainControllerRef can be constructed and dropped without leak.
#[test]
fn construction_and_drop_is_safe() {
    let mc = MainControllerRef::new(Box::new(NullMainController));
    drop(mc);
}
