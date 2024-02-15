use p2d::bounding_volume::Aabb;
use rnote_compose::{shapes::Shapeable, transform::Transformable};
use serde::{Deserialize, Serialize};

use crate::{
    pens::latex::latex_generator::{self, LatexContext},
    render, Drawable,
};

use super::{content::GeneratedContentImages, Content, VectorImage};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename = "vectorimage")]
pub struct LatexImage {
    #[serde(rename = "latex_data")]
    pub latex_code: String,
    #[serde(rename = "vector_image")]
    pub vector_image: Option<VectorImage>,
}

impl Default for LatexImage {
    fn default() -> Self {
        Self {
            latex_code: String::default(),
            vector_image: Option::None,
        }
    }
}

impl Content for LatexImage {
    fn gen_svg(&self) -> Result<render::Svg, anyhow::Error> {
        self.vector_image.as_ref().unwrap().gen_svg()
    }

    fn gen_images(
        &self,
        viewport: Aabb,
        image_scale: f64,
    ) -> Result<GeneratedContentImages, anyhow::Error> {
        self.vector_image
            .as_ref()
            .unwrap()
            .gen_images(viewport, image_scale)
    }

    fn update_geometry(&mut self) {}
}

// Because it is currently not possible to render SVGs directly with piet, the default gen_svg() implementation is
// overwritten and called in `draw()` and `draw_to_cairo()`. There the rsvg renderer is used to generate bitmap
// images. This way it is ensured that an actual Svg is generated when calling `gen_svg()`, but it is also possible to
// to be drawn to piet.
impl Drawable for LatexImage {
    fn draw(&self, cx: &mut impl piet::RenderContext, image_scale: f64) -> anyhow::Result<()> {
        self.vector_image.as_ref().unwrap().draw(cx, image_scale)
    }

    fn draw_to_cairo(&self, cx: &cairo::Context, image_scale: f64) -> anyhow::Result<()> {
        self.vector_image
            .as_ref()
            .unwrap()
            .draw_to_cairo(cx, image_scale)
    }
}

impl Shapeable for LatexImage {
    fn bounds(&self) -> Aabb {
        self.vector_image.as_ref().unwrap().bounds()
    }

    fn hitboxes(&self) -> Vec<Aabb> {
        self.vector_image.as_ref().unwrap().hitboxes()
    }

    fn outline_path(&self) -> kurbo::BezPath {
        self.vector_image.as_ref().unwrap().outline_path()
    }
}

impl Transformable for LatexImage {
    fn translate(&mut self, offset: na::Vector2<f64>) {
        self.vector_image.as_mut().unwrap().translate(offset);
    }

    fn rotate(&mut self, angle: f64, center: na::Point2<f64>) {
        self.vector_image.as_mut().unwrap().rotate(angle, center);
    }

    fn scale(&mut self, scale: na::Vector2<f64>) {
        self.vector_image.as_mut().unwrap().scale(scale);
    }
}

impl LatexImage {
    pub fn from_latex(
        latex_code: &String,
        latex_context: &LatexContext,
        pos: na::Vector2<f64>,
        size: Option<na::Vector2<f64>>,
    ) -> Self {
        let svg_code = latex_generator::create_svg_from_latex(latex_code, latex_context);
        let vector_image = VectorImage::from_svg_str(&svg_code, pos, size).unwrap();

        Self {
            latex_code: latex_code.clone(),
            vector_image: Option::Some(vector_image),
        }
    }

    pub fn copy_transform(&mut self, latex_image: &LatexImage) {
        self.vector_image.as_mut().unwrap().rectangle =
            latex_image.vector_image.as_ref().unwrap().rectangle.clone();
    }
}
