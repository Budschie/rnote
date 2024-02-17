use mathjax::MathJax;
use once_cell::sync::OnceCell;

use super::equation_provider::EquationProviderTrait;

static RENDERER: OnceCell<MathJax> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct MathJaxEquationProvider {}

impl EquationProviderTrait for MathJaxEquationProvider {
    fn generate_svg(&self, code: &String, font_size: u32) -> Result<String, String> {
        // TODO: Find way to integrate teh font size here
        let renderer = RENDERER.get_or_init(|| MathJax::new().unwrap());

        let result = renderer.render(code);

        match result {
            Ok(svg) => Result::Ok(svg.into_raw()),

            Err(render_error) => Result::Err(render_error.to_string()),
        }
    }
}
