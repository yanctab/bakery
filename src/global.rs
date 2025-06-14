use once_cell::sync::OnceCell;

pub struct TestMode;

static TEST_MODE: OnceCell<bool> = OnceCell::new();

impl TestMode {
    pub fn set_test_mode(enabled: bool) {
        // Test mode can only be set once
        TEST_MODE.set(enabled).unwrap();
    }

    pub fn is_test_mode() -> bool {
        // If not set we return false
        *TEST_MODE.get().unwrap_or(&false)
    }
}