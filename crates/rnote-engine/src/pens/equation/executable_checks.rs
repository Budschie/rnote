pub trait ExecutableChecker {
    pub fn is_available() -> bool;
}

pub struct LatexExecutableChecker {}

impl ExecutableChecker for LatexExecutableChecker {
    pub fn is_available() -> bool {
        true
    }
}
