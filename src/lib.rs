//! This module provides an interface for managing a rolling index.
//!
//! Note that the rolling index is runtime-specific and ephemeral,
//! meaning it is reset every time the application starts.

// Utilize Mutex from std::sync for concurrency safety
use std::sync::Mutex;

use lazy_static::lazy_static;

lazy_static! {
    /// The rolling index. This is increased with each call to `rolling_idx`.
    static ref _ROLLING_IDX: Mutex<u8> = Mutex::new(0);
}

/// Returns the current rolling index and then increases it by 1.
///
/// The rolling index is ephemeral and runtime-specific,
/// meaning it is reset every time the application starts.
pub fn rolling_idx() -> u8 {
    // Using lock() to access the guarded data safely
    let mut this = crate::_ROLLING_IDX.lock().unwrap();
    // Taking the value under the lock rather than dereferencing
    let val = *this;
    // Increase the index under the lock so that race conditions can be prevented
    *this += 1;
    // Return the taken value, this is safe as we have ensured no other thread can
    // manipulate the index at the same time
    val
}
