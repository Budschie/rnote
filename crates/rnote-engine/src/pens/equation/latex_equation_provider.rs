use serde::{Deserialize, Serialize};

use crate::pens::equation::latex_equation_generator::{
    create_svg_from_latex, LatexContext, INLINE,
};

use super::equation_provider::EquationProviderTrait;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename = "latex_equation_provider")]
pub struct LatexEquationProvider {}

impl EquationProviderTrait for LatexEquationProvider {
    fn generate_svg(&self, code: &String, font_size: u32) -> Result<String, String> {
        // TODO: Find way to integrate the font size here

        let preamble = String::from(format!("{}{}{}\n", "\\usepackage{amsmath}\n\\usepackage{amssymb}\n\\usepackage[usenames]{color}\n\\usepackage{ifxetex}\n\\usepackage{ifluatex}\n\\usepackage{fix-cm}\n\\usepackage[fontsize=", font_size, "pt]{fontsize}\n"));

        println!("{}", preamble);

        let latex_context = LatexContext {
            preamble,
            environment: INLINE,
        };

        create_svg_from_latex(code, &latex_context)
    }
}
