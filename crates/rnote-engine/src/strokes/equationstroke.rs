use nalgebra::Vector2;
use p2d::bounding_volume::Aabb;
use rnote_compose::{
    shapes::{Rectangle, Shapeable},
    transform::Transformable,
    Transform,
};
use roxmltree::Error;
use serde::{Deserialize, Serialize};

use crate::{pens::pensconfig::equationconfig::EquationConfig, render, Drawable};

pub(crate) use super::equationstrokedefault::DEFAULT_SVG_CODE;

use super::{content::GeneratedContentImages, Content, VectorImage};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename = "vectorimage")]
pub struct EquationImage {
    #[serde(rename = "equation_code")]
    pub equation_code: String,
    #[serde(default, rename = "equation_config")]
    pub equation_config: EquationConfig,
    #[serde(rename = "vector_image")]
    pub vector_image: Option<VectorImage>,
}

impl Default for EquationImage {
    fn default() -> Self {
        Self {
            equation_code: String::default(),
            vector_image: Option::None,
            equation_config: EquationConfig::default(),
        }
    }
}

impl Content for EquationImage {
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

    // TODO: Image scale has to be adjusted somehow else, maybe by doing something with the vector image
    fn update_geometry(&mut self) {}
}

// Because it is currently not possible to render SVGs directly with piet, the default gen_svg() implementation is
// overwritten and called in `draw()` and `draw_to_cairo()`. There the rsvg renderer is used to generate bitmap
// images. This way it is ensured that an actual Svg is generated when calling `gen_svg()`, but it is also possible to
// to be drawn to piet.
impl Drawable for EquationImage {
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

impl Shapeable for EquationImage {
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

impl Transformable for EquationImage {
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

impl EquationImage {
    pub fn new(
        equation_code: &str,
        equation_config: &EquationConfig,
        pos: na::Vector2<f64>,
        size: Option<na::Vector2<f64>>,
    ) -> Self {
        let vector_image = VectorImage::from_svg_str(DEFAULT_SVG_CODE, pos, size).unwrap();

        Self {
            equation_code: String::from(equation_code),
            vector_image: Option::Some(vector_image),
            equation_config: equation_config.clone(),
        }
    }

    pub fn update_svg_code(&mut self, svg_code: &str) {
        let mut vector_image =
            VectorImage::from_svg_str(svg_code, self.access_rectangle().pos(), None);
        if let Ok(mut some_vector_image) = vector_image {
            EquationImage::copy_transform_preserve_position(
                &mut some_vector_image.rectangle,
                self.access_rectangle(),
            );
            self.vector_image = Some(some_vector_image);
        }

        // TODO: Print error out instead of ignoring it
    }

    pub fn access_rectangle(&self) -> &Rectangle {
        &self.vector_image.as_ref().unwrap().rectangle
    }

    fn copy_transform_preserve_position(
        this_rectangle: &mut Rectangle,
        other_rectangle: &Rectangle,
    ) {
        this_rectangle.transform = other_rectangle.transform.clone();

        let self_upper_left = this_rectangle.outline_lines()[0].start;
        let other_upper_left = other_rectangle.outline_lines()[0].start;
        this_rectangle
            .transform
            .translate(other_upper_left - self_upper_left);
    }
}
