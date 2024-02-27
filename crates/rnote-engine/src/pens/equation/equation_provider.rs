use serde::{Deserialize, Serialize};

use super::{
    latex_equation_provider::LatexEquationProvider,
    mathjax_equation_provider::MathJaxEquationProvider,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "equation_provider")]
pub enum EquationProvider {
    #[serde(rename = "math_jax_equation_provider")]
    MathJaxEquationProvider(MathJaxEquationProvider),
    #[serde(rename = "latex_equation_provider")]
    LatexEquationProvider(LatexEquationProvider),
}

impl Default for EquationProvider {
    fn default() -> Self {
        EquationProvider::MathJaxEquationProvider(MathJaxEquationProvider {})
    }
}

impl EquationProviderTrait for EquationProvider {
    fn generate_svg(&self, code: &String, font_size: u32) -> Result<String, String> {
        match self {
            EquationProvider::MathJaxEquationProvider(math_jax_equation_provider) => {
                math_jax_equation_provider.generate_svg(code, font_size)
            }
            EquationProvider::LatexEquationProvider(latex_equation_provider) => {
                latex_equation_provider.generate_svg(code, font_size)
            }
        }
    }
}

pub trait EquationProviderTrait {
    // Generates a preview svg and returns either the SVG or an error output.
    fn generate_svg(&self, code: &String, font_size: u32) -> Result<String, String>;
}
