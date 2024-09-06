use serde::{Deserialize, Serialize};

use super::{
    executable_checks::{ExecutableChecker, LatexExecutableChecker},
    latex_equation_provider::LatexEquationProvider,
};

/// An equation provider compiles equations such as LaTeX and returns SVG code.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "equation_provider")]
pub enum EquationProvider {
    #[serde(rename = "latex_equation_provider")]
    LatexEquationProvider(LatexEquationProvider, LatexExecutableChecker),
}

impl Default for EquationProvider {
    fn default() -> Self {
        EquationProvider::LatexEquationProvider(LatexEquationProvider {})
    }
}

impl ExecutableChecker for EquationProvider {
    pub fn is_available() -> bool {
        match self {
            EquationProvider::LatexEquationProvider(_, exec) => exec.is_available(),
        }
    }
}

impl EquationProviderTrait for EquationProvider {
    fn generate_svg(
        &self,
        code: &String,
        font_size: u32,
        page_width: f64,
    ) -> Result<String, String> {
        match self {
            EquationProvider::LatexEquationProvider(latex_equation_provider) => {
                latex_equation_provider.generate_svg(code, font_size, page_width)
            }
        }
    }
}

/// The equation provider trait is an interface which defines how equations should be processed.
pub trait EquationProviderTrait {
    /// Generates a preview svg and returns either the SVG or an error output.
    fn generate_svg(
        &self,
        code: &String,
        font_size: u32,
        page_width: f64,
    ) -> Result<String, String>;
}
