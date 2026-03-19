use super::*;
use crate::property::float_property::FloatProperty;
use crate::property::timer_property::TimerProperty;
use crate::reexports::Timer;

/// Minimal mock MainState for Lua property tests.
struct MockMainState {
    timer: Timer,
}

impl Default for MockMainState {
    fn default() -> Self {
        Self {
            timer: Timer::default(),
        }
    }
}

impl rubato_types::timer_access::TimerAccess for MockMainState {
    fn now_time(&self) -> i64 {
        self.timer.now_time()
    }
    fn now_micro_time(&self) -> i64 {
        self.timer.now_micro_time()
    }
    fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.micro_timer(timer_id)
    }
    fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.timer(timer_id)
    }
    fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.timer.now_time_for(timer_id)
    }
    fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
        self.timer.is_timer_on(timer_id)
    }
}

impl rubato_types::skin_render_context::SkinRenderContext for MockMainState {}

impl MainState for MockMainState {}

#[test]
fn boolean_property_works_on_creation_thread() {
    let accessor = SkinLuaAccessor::new(true);
    // load_boolean_property_from_script prepends "return ", so the script
    // becomes "return true" which evaluates to a boolean value directly.
    let prop = accessor
        .load_boolean_property_from_script("true")
        .expect("should load boolean property");
    let state = MockMainState::default();
    assert!(prop.get(&state));
}

#[test]
fn integer_property_works_on_creation_thread() {
    let accessor = SkinLuaAccessor::new(true);
    let prop = accessor
        .load_integer_property_from_script("42")
        .expect("should load integer property");
    let state = MockMainState::default();
    assert_eq!(prop.get(&state), 42);
}

#[test]
fn float_property_works_on_creation_thread() {
    let accessor = SkinLuaAccessor::new(true);
    let prop = accessor
        .load_float_property_from_script("3.14")
        .expect("should load float property");
    let state = MockMainState::default();
    let expected: f32 = "3.14".parse().unwrap();
    assert!((prop.get(&state) - expected).abs() < 0.01);
}

#[test]
fn string_property_works_on_creation_thread() {
    let accessor = SkinLuaAccessor::new(true);
    let prop = accessor
        .load_string_property_from_script("'hello'")
        .expect("should load string property");
    let state = MockMainState::default();
    assert_eq!(prop.get(&state), "hello");
}

#[test]
fn timer_property_works_on_creation_thread() {
    let accessor = SkinLuaAccessor::new(true);
    // Timer property has special handling: if the script returns a function,
    // that function is used as the timer function (trial call mechanism).
    let prop = accessor
        .load_timer_property_from_script("function() return 1000000 end")
        .expect("should load timer property");
    let state = MockMainState::default();
    assert_eq!(prop.get_micro(&state), 1000000);
}

#[test]
fn event_works_on_creation_thread() {
    let accessor = SkinLuaAccessor::new(true);
    // load_event_from_script loads the script directly (no "return " prefix).
    // The script must be a valid Lua chunk that compiles to a function.
    // Use "return function(a, b) end" so the chunk returns a callable function.
    let func = accessor
        .lua()
        .load("return function(a, b) end")
        .into_function()
        .expect("should compile event chunk");
    let result: LuaValue = func.call(()).expect("should call chunk");
    if let LuaValue::Function(inner) = result {
        let event = accessor
            .load_event_from_function(inner)
            .expect("should load event");
        let mut state = MockMainState::default();
        // Should not panic
        event.exec(&mut state, 1, 2);
    } else {
        panic!("Expected Lua function from chunk");
    }
}

#[test]
fn float_writer_works_on_creation_thread() {
    let accessor = SkinLuaAccessor::new(true);
    let func = accessor
        .lua()
        .load("return function(v) end")
        .into_function()
        .expect("should compile float writer chunk");
    let result: LuaValue = func.call(()).expect("should call chunk");
    if let LuaValue::Function(inner) = result {
        let writer = accessor
            .load_float_writer_from_function(inner)
            .expect("should load float writer");
        let mut state = MockMainState::default();
        // Should not panic
        writer.set(&mut state, 1.0);
    } else {
        panic!("Expected Lua function from chunk");
    }
}

/// Verify that the thread-safety assert fires when a Lua property is accessed from a
/// different thread than where it was created.
#[test]
fn boolean_property_panics_on_wrong_thread() {
    let accessor = SkinLuaAccessor::new(true);
    let prop = accessor
        .load_boolean_property_from_script("true")
        .expect("should load boolean property");
    let state = MockMainState::default();

    // Access from a different thread should panic due to thread-safety assert
    let handle = std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            prop.get(&state);
        }));
        assert!(
            result.is_err(),
            "Expected panic when accessing LuaBooleanProperty from wrong thread"
        );
    });
    handle.join().expect("thread should complete");
}

/// Verify that the thread-safety assert fires for integer property on wrong thread.
#[test]
fn integer_property_panics_on_wrong_thread() {
    let accessor = SkinLuaAccessor::new(true);
    let prop = accessor
        .load_integer_property_from_script("42")
        .expect("should load integer property");
    let state = MockMainState::default();

    let handle = std::thread::spawn(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            prop.get(&state);
        }));
        assert!(
            result.is_err(),
            "Expected panic when accessing LuaIntegerProperty from wrong thread"
        );
    });
    handle.join().expect("thread should complete");
}

/// Verify that the Lua sandbox blocks os.execute (arbitrary command execution).
#[test]
fn lua_sandbox_blocks_os_execute() {
    let accessor = SkinLuaAccessor::new(true);
    let result = accessor.exec("return os ~= nil");
    // os library should not be available; the expression should either
    // return nil/false or fail entirely
    match result {
        Some(LuaValue::Boolean(true)) => {
            panic!("SECURITY: os library is accessible in Lua sandbox");
        }
        _ => {
            // os is nil or not available -- correct behavior
        }
    }
}

/// Verify that the Lua sandbox blocks io.open (file system access).
#[test]
fn lua_sandbox_blocks_io_open() {
    let accessor = SkinLuaAccessor::new(true);
    let result = accessor.exec("return io ~= nil");
    match result {
        Some(LuaValue::Boolean(true)) => {
            panic!("SECURITY: io library is accessible in Lua sandbox");
        }
        _ => {
            // io is nil or not available -- correct behavior
        }
    }
}

/// Verify that safe libraries (table, string, math) remain available.
#[test]
fn lua_sandbox_allows_safe_libraries() {
    let accessor = SkinLuaAccessor::new(true);

    // table library
    let result = accessor.exec("return table ~= nil");
    assert_eq!(
        result,
        Some(LuaValue::Boolean(true)),
        "table library should be available"
    );

    // string library
    let result = accessor.exec("return string ~= nil");
    assert_eq!(
        result,
        Some(LuaValue::Boolean(true)),
        "string library should be available"
    );

    // math library
    let result = accessor.exec("return math ~= nil");
    assert_eq!(
        result,
        Some(LuaValue::Boolean(true)),
        "math library should be available"
    );

    // Base functions (print, pairs, ipairs, type, tonumber, tostring)
    let result = accessor.exec("return type(print) == 'function'");
    assert_eq!(
        result,
        Some(LuaValue::Boolean(true)),
        "print should be available"
    );

    let result = accessor.exec("return type(pairs) == 'function'");
    assert_eq!(
        result,
        Some(LuaValue::Boolean(true)),
        "pairs should be available"
    );
}

/// Verify that require() and package.loaded still work (needed for module loading).
#[test]
fn lua_sandbox_allows_require() {
    let accessor = SkinLuaAccessor::new(false);
    // package library should still be available for require/package.loaded
    let result = accessor.exec("return package ~= nil");
    assert_eq!(
        result,
        Some(LuaValue::Boolean(true)),
        "package library should be available for skin module loading"
    );
}
