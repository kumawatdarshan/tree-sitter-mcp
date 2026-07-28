//! AI GENERATED TEST FIXTURE
//! Sample Rust source file used for tree-sitter MCP query and symbol extraction tests.
//!
//! Valid portion: lines 1 to before `this_function_has_missing_semicolon`.
//! Buggy portion: from `this_function_has_missing_semicolon` onwards.

use std::collections::{HashMap, HashSet};
use std::fmt::{self, Debug, Display};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

pub use std::collections::hash_map::Entry as MapEntry;

// Constants & Statics
pub const MAX_CONNECTIONS: u32 = 1024;
const DEFAULT_TIMEOUT_MS: u64 = 5_000;
static GLOBAL_REQUEST_COUNT: AtomicUsize = AtomicUsize::new(0);
static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

// Type Aliases
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;
pub type CrateResult<T> = Result<T, CrateError>;

// Enums & Variants (with serde and derive annotations)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum RetryStrategy {
    None,
    Fixed { interval_ms: u64 },
    Exponential { base_ms: u64, max_ms: u64, jitter: bool },
    Custom { max_retries: u32 },
}

#[derive(Debug)]
pub enum CrateError {
    Io { path: PathBuf, source: std::io::Error },
    Serialization(String),
    Validation { field: String, reason: String },
    Timeout { operation: String, elapsed: std::time::Duration },
    Custom(String),
}

// Structs & Fields (Public, Private, Crate, Super, Generics, Lifetimes)
#[derive(Debug, Clone)]
pub struct Config {
    pub name: String,
    pub version: u32,
    pub debug: bool,
    pub features: HashSet<String>,
    pub max_retries: u32,
    pub timeout_ms: u64,
}

#[derive(Debug)]
pub struct Container<T: Debug> {
    value: T,
    pub label: String,
    created_at: std::time::Instant,
    metadata: HashMap<String, String>,
}

pub(crate) struct InternalState {
    pub(super) active_connections: usize,
    flag: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct LinuxSpecificHandle {
    pub fd: i32,
}

// Traits with Associated Constants, Types, and Methods
pub trait Geometric: Debug {
    const DIMENSIONS: u32 = 2;
    type Output;
    fn area(&self) -> f64;
    fn perimeter(&self) -> f64;
    fn is_origin(&self) -> bool {
        false
    }
}

// Struct Implementations
impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.x += dx;
        self.y += dy;
    }

    fn private_distance(&self, other: &Point) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

// Trait Implementations
impl Geometric for Point {
    type Output = f64;

    fn area(&self) -> f64 {
        0.0
    }

    fn perimeter(&self) -> f64 {
        0.0
    }

    fn is_origin(&self) -> bool {
        self.x == 0.0 && self.y == 0.0
    }
}

impl Display for RetryStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RetryStrategy::None => write!(f, "none"),
            RetryStrategy::Fixed { interval_ms } => write!(f, "fixed({interval_ms}ms)"),
            RetryStrategy::Exponential { base_ms, max_ms, jitter } => {
                write!(f, "exponential({base_ms}ms..{max_ms}ms, jitter={jitter})")
            }
            RetryStrategy::Custom { max_retries } => write!(f, "custom({max_retries})"),
        }
    }
}

// Submodules
pub mod internal {
    use super::*;

    pub struct SubSystem {
        pub id: u64,
    }

    impl SubSystem {
        pub fn run(&self) -> bool {
            true
        }
    }
}

// Functions (Async, Const, Standard, Attributes, Calls)
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub const fn const_add(a: i32, b: i32) -> i32 {
    a + b
}

#[tracing::instrument(level = "info")]
pub async fn fetch_data(url: &str) -> CrateResult<Vec<u8>> {
    let p = Point::new(1.0, 2.0);
    let area = p.area();
    let _ = area;
    Ok(url.as_bytes().to_vec())
}

#[deprecated(since = "2.0", note = "use add instead")]
#[must_use = "returns result"]
pub fn legacy_calculator(val: i32) -> i32 {
    add(val, 10)
}

// ============================================================================
// PLATFORM-SPECIFIC FUNCTIONS & CFG MACROS
// (For TDD queries like "what functions are unique to linux")
// ============================================================================

#[cfg(target_os = "linux")]
#[tracing::instrument]
pub fn linux_only_system_call() -> i32 {
    0
}

#[cfg(target_os = "windows")]
#[tracing::instrument]
pub fn windows_only_system_call() -> i32 {
    1
}

#[cfg(target_os = "macos")]
pub fn macos_only_system_call() -> i32 {
    2
}

#[cfg(feature = "extra_metrics")]
#[tracing::instrument(skip(metrics_data))]
pub fn process_extra_metrics(metrics_data: &[u8]) {
    let _ = metrics_data;
}

// ============================================================================
// ATTRIBUTE MACROS & TRACING ANNOTATIONS
// (For TDD queries like "find all functions that don't have tracing instrument")
// ============================================================================

#[tracing::instrument(level = "debug")]
pub fn instrumented_function(input: &str) -> usize {
    input.len()
}

#[tracing::instrument(name = "custom_trace_span", skip(secret))]
pub fn instrumented_with_options(secret: &str, public_val: i32) -> i32 {
    let _ = secret;
    public_val * 2
}

#[inline(always)]
pub fn uninstrumented_utility_function(val: i32) -> i32 {
    val + 1
}

#[cold]
pub fn uninstrumented_error_handler(err: &str) {
    eprintln!("Error occurred: {err}");
}

// High-complexity function for testing cognitive/cyclomatic complexity metrics
pub fn complex_control_flow<T, U>(x: T, y: U) -> i32
where
    T: Into<i32> + Copy,
    U: TryInto<i32> + Add<Output = U> + Mul<Output = U> + Copy,
    <U as TryInto<i32>>::Error: std::fmt::Debug,
{

    let mut result = 0;
    for i in 0..x {
        if i % 2 == 0 {
            result += i;
            if i > 10 {
                result *= 2;
                if i > 20 {
                    result -= 1;
                }
            }
        } else {
            match i % 5 {
                0 => result += 100,
                1 => result += 50,
                2 => result += 25,
                3 => result += 10,
                _ => result += y,
            }
        }
    }
    result
}

// Logical bug patterns for bug identification testing
pub fn this_function_has_infinite_recursion() -> i32 {
    this_function_has_infinite_recursion()
}

pub fn this_function_has_unreachable_code() -> i32 {
    return 42;
    let x = 5;
    x + 1
}

pub fn this_function_has_dead_code() {
    if false {
        println!("this never runs");
    }
}

pub fn this_function_has_infinite_loop() {
    while true {
        // infinite loop
    }
}

// Test functions
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_creation() {
        let p = Point::new(3.0, 4.0);
        assert_eq!(p.x, 3.0);
    }
}

// ============================================================================
// BUGGY PORTION (Produces syntax ERROR nodes for tree-sitter error testing)
// ============================================================================

pub fn this_function_has_missing_semicolon() -> i32 {
    let x = 5
    x + 1
}

pub fn this_function_has_unmatched_brace() -> i32 {
    if true {
        42
}

pub fn this_function_has_invalid_token() {
    let x = @invalid;
}

pub fn this_function_has_double_colon_in_path() {
    let x = std::io::::stdin();
}

pub fn this_function_has_missing_comma_in_params(x: i32 y: i32) -> i32 {
    x + y
}

pub fn this_function_has_unclosed_string() {
    let s = "hello;
}
