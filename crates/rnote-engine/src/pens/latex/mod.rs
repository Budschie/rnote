mod latex_generator;

use nalgebra::Vector2;
use p2d::bounding_volume::Aabb;
use piet::RenderContext;
use rnote_compose::{eventresult::EventPropagation, penevent::PenProgress, EventResult, PenEvent};

use crate::{
    engine::{EngineView, EngineViewMut},
    strokes::{Stroke, VectorImage},
    DrawableOnDoc, WidgetFlags,
};

use self::latex_generator::{create_svg_from_latex, LatexContext, INLINE};

use super::{PenBehaviour, PenStyle};

#[derive(Debug, Clone)]
pub struct LatexCompileInstruction {
    pub code: String,
    pub position: Vector2<f64>,
}

#[derive(Debug, Clone)]
pub struct LatexDrawInstruction {
    pub position: Vector2<f64>,
}

#[derive(Debug, Clone)]
pub enum LatexState {
    Idle,
    ExpectingCode(LatexDrawInstruction),
    ReceivingCode(LatexDrawInstruction),
    Finished(LatexCompileInstruction),
}

#[derive(Debug, Clone)]
pub struct Latex {
    pub state: LatexState,
    pub latex_context: LatexContext,
}

impl Default for Latex {
    fn default() -> Self {
        Self {
			state: LatexState::Idle,
			latex_context: LatexContext {
				preamble: String::from("\\usepackage{amsmath}\n\\usepackage{amssymb}\n\\usepackage[usenames]{color}\n\\usepackage{ifxetex}\n\\usepackage{ifluatex}\n"),
				environment: INLINE,
			},
		}
    }
}

impl Latex {
    fn get_widget_flags_update() -> WidgetFlags {
        let mut default_widget_flags = WidgetFlags::default();
        default_widget_flags.refresh_ui = true;
        default_widget_flags
    }
}

impl PenBehaviour for Latex {
    fn init(&mut self, _engine_view: &EngineView) -> WidgetFlags {
        WidgetFlags::default()
    }

    fn deinit(&mut self) -> WidgetFlags {
        WidgetFlags::default()
    }

    fn style(&self) -> PenStyle {
        PenStyle::Latex
    }

    fn update_state(&mut self, engine_view: &mut EngineViewMut) -> WidgetFlags {
        let mut done = false;

        match &self.state {
            LatexState::Finished(compile_instructions) => {
                done = true;
                let svg_contents =
                    create_svg_from_latex(&compile_instructions.code, &self.latex_context);
                engine_view.store.insert_stroke(
                    Stroke::VectorImage(
                        VectorImage::from_svg_str(
                            &svg_contents,
                            compile_instructions.position,
                            None,
                        )
                        .unwrap(),
                    ),
                    None,
                );
                engine_view.store.regenerate_rendering_in_viewport_threaded(
                    engine_view.tasks_tx.clone(),
                    true,
                    engine_view.camera.viewport(),
                    engine_view.camera.image_scale(),
                );
            }
            _ => {}
        }

        if done {
            self.state = LatexState::Idle;
        }

        WidgetFlags::default()
    }

    fn handle_event(
        &mut self,
        event: rnote_compose::PenEvent,
        now: std::time::Instant,
        engine_view: &mut EngineViewMut,
    ) -> (
        rnote_compose::EventResult<rnote_compose::penevent::PenProgress>,
        WidgetFlags,
    ) {
        let result = match (event, &self.state) {
            (PenEvent::Down { element, .. }, LatexState::Idle) => {
                self.state = LatexState::ExpectingCode(LatexDrawInstruction {
                    position: element.pos,
                });

                (
                    EventResult {
                        handled: true,
                        propagate: EventPropagation::Stop,
                        progress: PenProgress::InProgress,
                    },
                    Latex::get_widget_flags_update(),
                )
            }
            (_, _) => (
                EventResult {
                    handled: false,
                    propagate: EventPropagation::Proceed,
                    progress: PenProgress::InProgress,
                },
                WidgetFlags::default(),
            ),
        };

        result
    }
}

impl DrawableOnDoc for Latex {
    fn bounds_on_doc(&self, engine_view: &EngineView) -> Option<Aabb> {
        let style = engine_view
            .pens_config
            .brush_config
            .style_for_current_options();

        match &self.state {
            LatexState::Idle => None,
            _ => None,
        }
    }

    fn draw_on_doc(
        &self,
        cx: &mut piet_cairo::CairoRenderContext,
        engine_view: &EngineView,
    ) -> anyhow::Result<()> {
        cx.save().map_err(|e| anyhow::anyhow!("{e:?}"))?;

        match &self.state {
            LatexState::Idle => {}
            _ => {}
        }

        cx.restore().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(())
    }
}
