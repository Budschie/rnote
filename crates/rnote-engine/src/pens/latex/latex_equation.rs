use super::{
    equation_provider::EquationProviderTrait,
    latex_generator::{self, LatexContext, INLINE},
};

#[derive(Debug, Clone)]
pub struct LatexEquationProvider {}

impl EquationProviderTrait for LatexEquationProvider {
    fn generate_svg(&self, code: &String, font_size: u32) -> Result<String, String> {
        // TODO: Find way to integrate the font size here
        let latex_context = LatexContext {
			preamble: String::from("\\usepackage{amsmath}\n\\usepackage{amssymb}\n\\usepackage[usenames]{color}\n\\usepackage{ifxetex}\n\\usepackage{ifluatex}\n"),
			environment: INLINE,
		};

        latex_generator::create_svg_from_latex(code, &latex_context)
    }
}
