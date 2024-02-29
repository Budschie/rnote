use serde::{Deserialize, Serialize};

use super::latex_equation_provider::LatexEquationProvider;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename = "equation_provider")]
pub enum EquationProvider {
    #[serde(rename = "latex_equation_provider")]
    LatexEquationProvider(LatexEquationProvider),
}

impl Default for EquationProvider {
    fn default() -> Self {
        EquationProvider::LatexEquationProvider(LatexEquationProvider {})
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

pub trait EquationProviderTrait {
    // Generates a preview svg and returns either the SVG or an error output.
    fn generate_svg(
        &self,
        code: &String,
        font_size: u32,
        page_width: f64,
    ) -> Result<String, String>;
}
