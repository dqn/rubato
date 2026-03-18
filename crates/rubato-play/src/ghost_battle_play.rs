use std::sync::Mutex;

use rubato_types::sync_utils::lock_or_recover;

static BATTLE: Mutex<Option<Settings>> = Mutex::new(None);

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub random: i32,
    pub lanes: i32,
}

pub fn consume() -> Option<Settings> {
    let mut lock = lock_or_recover(&BATTLE);
    lock.take()
}

pub fn setup(random: i32, lane_sequence: i32) {
    let mut lock = lock_or_recover(&BATTLE);
    *lock = Some(Settings {
        random,
        lanes: lane_sequence,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    // Tests share a global static (BATTLE), so they must not run in parallel.
    // This mutex serializes all tests in this module.
    static TEST_LOCK: StdMutex<()> = StdMutex::new(());

    /// Helper: ensure the global is empty before each test.
    fn reset() {
        let _ = consume();
    }

    #[test]
    fn setup_then_consume_returns_correct_settings() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();

        setup(42, 7);

        let result = consume();
        assert!(result.is_some());
        let s = result.unwrap();
        assert_eq!(s.random, 42);
        assert_eq!(s.lanes, 7);
    }

    #[test]
    fn consume_on_empty_returns_none() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();

        assert!(consume().is_none());
    }

    #[test]
    fn second_consume_returns_none() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();

        setup(1, 2);
        let first = consume();
        assert!(first.is_some());

        let second = consume();
        assert!(second.is_none());
    }

    #[test]
    fn multiple_setup_calls_last_one_wins() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();

        setup(10, 20);
        setup(30, 40);
        setup(50, 60);

        let result = consume().unwrap();
        assert_eq!(result.random, 50);
        assert_eq!(result.lanes, 60);
    }

    #[test]
    fn thread_safety_setup_from_one_thread_consume_from_another() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset();

        let producer = std::thread::spawn(|| {
            setup(99, 88);
        });
        producer.join().unwrap();

        let consumer = std::thread::spawn(consume);
        let result = consumer.join().unwrap();

        let s = result.unwrap();
        assert_eq!(s.random, 99);
        assert_eq!(s.lanes, 88);
    }
}
