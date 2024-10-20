use serde::{Deserialize, Serialize};
use which::which;

pub trait ExecutableChecker {
    fn is_available(&self) -> bool;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LatexExecutableChecker {}

impl ExecutableChecker for LatexExecutableChecker {
    fn is_available(&self) -> bool {
        // Check for binaries
        which("dvisvgm").is_ok() && which("latex").is_ok()
    }
}
