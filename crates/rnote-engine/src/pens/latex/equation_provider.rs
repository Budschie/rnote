use super::{
    latex_equation::LatexEquationProvider, mathjax_equation_provider::MathJaxEquationProvider,
};

#[derive(Debug, Clone)]
pub enum EquationProvider {
    MathJaxEquationProvider(MathJaxEquationProvider),
    LatexEquationProvider(LatexEquationProvider),
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
