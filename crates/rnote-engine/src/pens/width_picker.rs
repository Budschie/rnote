use gtk4::gdk::CairoContext;
use nalgebra::{Point2, Vector2};
use p2d::{
    bounding_volume::{Aabb, BoundingSphere},
    query::PointQuery,
};
use rnote_compose::{
    eventresult::EventPropagation, ext::Vector2Ext, penevent::PenState, serialize::f64_dp3,
    shapes::Shapeable, style::indicators, PenEvent,
};

use crate::DrawableOnDoc;

#[derive(Debug, Clone)]
struct WidthPickerNode {
    pub pos: Vector2<f64>,
    pub state: PenState,
}

impl WidthPickerNode {
    fn new(pos: Vector2<f64>, state: PenState) -> WidthPickerNode {
        WidthPickerNode { pos, state }
    }

    fn draw(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &crate::engine::EngineView,
    ) {
        let zoom = engine_view.camera.total_zoom();
        let size_scaled = self.size_scaled(1.0 / zoom);
        indicators::draw_circular_node(cx, self.state, size_scaled, zoom);
    }

    fn size(&self) -> BoundingSphere {
        self.size_scaled(1.0)
    }

    fn size_scaled(&self, scale: f64) -> BoundingSphere {
        BoundingSphere::new(Point2::from(self.pos), 7.0 * scale)
    }

    fn bounding_points_scaled(&self, scale: f64) -> (Point2<f64>, Point2<f64>) {
        let size_scaled = self.size_scaled(scale);

        (
            size_scaled.center - Vector2::new(size_scaled.radius, size_scaled.radius),
            size_scaled.center + Vector2::new(size_scaled.radius, size_scaled.radius),
        )
    }
}

#[derive(Debug, Clone)]
pub enum WidthPickerState {
    Idle,
    Dragging,
}

#[derive(Debug, Clone)]
pub struct WidthPickerContext {
    pub begin: WidthPickerNode,
    pub end: WidthPickerNode,
    pub projection_start: Vector2<f64>,
    pub projection_direction: Vector2<f64>,
    state: WidthPickerState,
}

// Use vector projection for determining the real vector
impl WidthPickerContext {
    pub fn new(
        starting_coordinates: Vector2<f64>,
        end_coordinates: Vector2<f64>,
        projection_start: Vector2<f64>,
        projection_direction: Vector2<f64>,
        initial_state: WidthPickerState,
    ) -> WidthPickerContext {
        WidthPickerContext {
            begin: WidthPickerNode::new(starting_coordinates, PenState::Up),
            end: WidthPickerNode::new(end_coordinates, PenState::Proximity),
            projection_start,
            projection_direction,
            state: initial_state,
        }
    }

    pub fn update(&mut self, pen_event: &PenEvent) -> EventPropagation {
        let mut event_propagation = EventPropagation::Proceed;

        match self.state {
            WidthPickerState::Idle => {
                match pen_event {
                    // Check if within bounds
                    PenEvent::Down { element, .. } => {
                        if self
                            .end
                            .size()
                            .contains_local_point(&Point2::from(element.pos))
                        {
                            self.state = WidthPickerState::Dragging;
                            event_propagation = EventPropagation::Stop;
                        }
                    }
                    _ => {}
                }
            }
            WidthPickerState::Dragging => {
                // Set positions
                match pen_event {
                    PenEvent::Down { element, .. } => {
                        self.end.state = PenState::Down;
                        self.end.pos = self.project_vector(element.pos);
                        event_propagation = EventPropagation::Stop;
                    }
                    PenEvent::Up { element, .. } => {
                        self.end.state = PenState::Proximity;
                        self.state = WidthPickerState::Idle;
                        event_propagation = EventPropagation::Stop;
                    }
                    _ => {}
                }
            }
        }

        event_propagation
    }

    pub fn draw_on_doc(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &crate::engine::EngineView,
    ) -> anyhow::Result<()> {
        let total_zoom = engine_view.camera.total_zoom();

        self.begin.draw(cx, engine_view);
        self.end.draw(cx, engine_view);

        indicators::draw_vec_indicator(
            cx,
            PenState::Down,
            self.begin.pos,
            self.end.pos,
            total_zoom,
        );

        anyhow::Result::Ok(())
    }

    pub fn aabb(&self, total_zoom: f64) -> Aabb {
        let radius = 10.0 / total_zoom;
        let (begin_a, begin_b) = self.begin.bounding_points_scaled(1.0 / total_zoom);
        let (end_a, end_b) = self.end.bounding_points_scaled(1.0 / total_zoom);
        let all_points = [&begin_a, &begin_b, &end_a, &end_b];
        Aabb::from_points(all_points.into_iter())
    }

    pub fn clip_lambda(lambda: f64) -> f64 {
        f64::max(15.0, lambda)
    }

    pub fn project_vector(&self, vec_to_project: Vector2<f64>) -> Vector2<f64> {
        // Perform vector projection; formula figured out by doing Maths

        let lambda = WidthPickerContext::clip_lambda(
            (vec_to_project.x * self.projection_direction.x
                + vec_to_project.y * self.projection_direction.y
                - self.projection_start.x * self.projection_direction.x
                - self.projection_start.y * self.projection_direction.y)
                / (self.projection_direction.x * self.projection_direction.x
                    + self.projection_direction.y * self.projection_direction.y),
        );

        self.projection_start + self.projection_direction * lambda
    }

    pub fn length(&self) -> f64 {
        (self.begin.pos - self.end.pos).magnitude()
    }
}
