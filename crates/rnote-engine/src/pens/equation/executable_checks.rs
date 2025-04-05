use super::latex_equation_provider::LatexEquationProvider;
use serde::{Deserialize, Serialize};
use which::which;

pub trait ExecutableChecker {
    fn is_available(&self) -> bool;
}

impl ExecutableChecker for LatexEquationProvider {
    fn is_available(&self) -> bool {
        // Check for binaries
        which("dvisvgm").is_ok() && which("latex").is_ok()
    }
}
