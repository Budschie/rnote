use serde::{Deserialize, Serialize};

use crate::pens::equation::latex_equation_generator::{self, create_svg_from_latex, LatexContext};

use super::equation_provider::EquationProviderTrait;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename = "latex_equation_provider")]
pub struct LatexEquationProvider {}

impl EquationProviderTrait for LatexEquationProvider {
    fn generate_svg(
        &self,
        code: &String,
        font_size: u32,
        page_width: f64,
    ) -> Result<String, String> {
        let preamble = String::from(format!("{}{}{}\n", "\\usepackage{amsmath}\n\\usepackage{amssymb}\n\\usepackage[usenames]{color}\n\\usepackage{ifxetex}\n\\usepackage{ifluatex}\n\\usepackage{fix-cm}\n\\usepackage[fontsize=", font_size, "pt]{fontsize}\n"));

        let environment = latex_equation_generator::Environment {
            pre_preamble: String::from(&format!(
                "{}{}{}",
                "\\documentclass[varwidth=", page_width, "mm, border=10pt]{standalone}"
            )),
            pre_code: String::from("\\begin{document}"),
            post_code: String::from("\\end{document}"),
        };

        let latex_context = LatexContext {
            preamble,
            environment,
        };

        create_svg_from_latex(code, &latex_context)
    }
}
