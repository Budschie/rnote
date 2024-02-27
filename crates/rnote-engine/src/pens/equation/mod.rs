pub mod equation_provider;
pub mod latex_equation_generator;
pub mod latex_equation_provider;
pub mod mathjax_equation_provider;

use std::time::Instant;

use nalgebra::Vector2;
use p2d::bounding_volume::Aabb;
use piet::RenderContext;
use rnote_compose::{
    eventresult::EventPropagation, penevent::PenProgress, penpath::Element, EventResult, PenEvent,
};

use crate::{
    engine::{EngineView, EngineViewMut},
    store::StrokeKey,
    strokes::{latexstroke::LatexImage, Stroke, VectorImage},
    DrawableOnDoc, WidgetFlags,
};

use super::{PenBehaviour, PenStyle};

#[derive(Debug, Clone)]
pub struct LatexCompiledInstruction {
    pub equation_code: String,
    pub svg_code: String,
    pub position: Vector2<f64>,
}

#[derive(Debug, Clone)]
pub struct LatexDrawInstruction {
    pub initial_code: String,
    pub position: Vector2<f64>,
}

#[derive(Debug, Clone)]
pub struct LatexUpdateData {
    pub old_latex_key: StrokeKey,
}

#[derive(Debug, Clone)]
pub enum LatexState {
    Idle,
    ExpectingCode(LatexDrawInstruction),
    ReceivingCode(LatexDrawInstruction),
    Finished(LatexCompiledInstruction),
}

// CreateNew means that a new latex object will be created; UpdateOld will update an already existing one
#[derive(Debug, Clone)]
pub enum LatexReference {
    CreateNew,
    UpdateOld(LatexUpdateData),
}

#[derive(Debug, Clone)]
pub struct Latex {
    pub state: LatexState,
    pub reference: LatexReference,
}

impl Default for Latex {
    fn default() -> Self {
        Self {
            state: LatexState::Idle,
            reference: LatexReference::CreateNew,
        }
    }
}

impl Latex {
    fn get_widget_flags_update() -> WidgetFlags {
        let mut default_widget_flags = WidgetFlags::default();
        default_widget_flags.refresh_ui = true;
        default_widget_flags
    }

    fn determine_latex_reference(
        element: Element,
        engine_view: &mut EngineViewMut,
    ) -> LatexReference {
        if let Some(&stroke_key) = engine_view
            .store
            .stroke_hitboxes_contain_coord(engine_view.camera.viewport(), element.pos)
            .last()
        {
            LatexReference::UpdateOld(LatexUpdateData {
                old_latex_key: stroke_key,
            })
        } else {
            LatexReference::CreateNew
        }
    }

    fn determine_initial_code(
        latex_reference: &LatexReference,
        engine_view: &mut EngineViewMut,
    ) -> String {
        match latex_reference {
            LatexReference::CreateNew => String::from(""),
            LatexReference::UpdateOld(update_data) => {
                let stroke = engine_view
                    .store
                    .get_stroke_ref(update_data.old_latex_key)
                    .unwrap();

                if let Stroke::LatexImage(latex_image) = stroke {
                    latex_image.equation_code.clone()
                } else {
                    // TODO: Warn about the fact that the LatexReference is not valid here
                    String::from("")
                }
            }
        }
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

                let mut stroke: Option<Stroke> = None;

                if let LatexReference::UpdateOld(update_data) = &self.reference {
                    stroke = engine_view.store.remove_stroke(update_data.old_latex_key);
                }

                /*let mut latex_image = LatexImage::from_latex(
                        &compile_instructions.code,
                        &self.equation_config,
                        compile_instructions.position,
                        None,
                );
                     */

                let mut equation_image = LatexImage::new(
                    &compile_instructions.equation_code,
                    &compile_instructions.svg_code,
                    &engine_view.pens_config.equation_config,
                    compile_instructions.position,
                    None,
                );

                if let Some(some_stroke) = stroke {
                    if let Stroke::LatexImage(old_latex_image) = some_stroke {
                        equation_image.copy_transform(&old_latex_image);
                    }
                }

                engine_view
                    .store
                    .insert_stroke(Stroke::LatexImage(equation_image), None);

                engine_view.store.record(Instant::now());

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
                self.reference = Latex::determine_latex_reference(element, engine_view);

                // Copy settings from a UpdateOld LatexReference if they are present
                if let LatexReference::UpdateOld(update_data) = &self.reference {
                    let old_latex_stroke =
                        engine_view.store.get_stroke_ref(update_data.old_latex_key);

                    if let Some(some_stroke) = old_latex_stroke {
                        if let Stroke::LatexImage(latex_stroke) = some_stroke {
                            engine_view.pens_config.equation_config =
                                latex_stroke.equation_config.clone();
                        }
                    }
                }
                self.state = LatexState::ExpectingCode(LatexDrawInstruction {
                    initial_code: Latex::determine_initial_code(&self.reference, engine_view),
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
