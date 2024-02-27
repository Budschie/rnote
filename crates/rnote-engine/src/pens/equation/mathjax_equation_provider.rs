use mathjax::MathJax;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use usvg::{NonZeroRect, Rect, Size, Transform};

use super::equation_provider::EquationProviderTrait;

static RENDERER: OnceCell<MathJax> = OnceCell::new();

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, rename = "math_jax_equation_provider")]
pub struct MathJaxEquationProvider {}

impl EquationProviderTrait for MathJaxEquationProvider {
    fn generate_svg(&self, code: &String, font_size: u32) -> Result<String, String> {
        // TODO: Find way to integrate teh font size here
        let renderer = RENDERER.get_or_init(|| MathJax::new().unwrap());
        let result = renderer.render(code);

        let scale_factor = (font_size as f32) / 12.0;

        match result {
            Ok(svg) => {
                /*
                let svg_nodes = svg.into_svg().unwrap();
                let old_width = svg_nodes.size.width() as f32;
                let old_height = svg_nodes.size.height() as f32;
                let old_rect = svg_nodes.view_box.rect = svg_nodes.view_box.rect.bbox_transform(NonZeroRect::from_xywh(0.0, 0.0, scale_factor, scale_factor));
                 */
                Ok(svg.into_raw())
            }

            Err(render_error) => Result::Err(render_error.to_string()),
        }
    }
}
