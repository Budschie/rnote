pub mod equation_provider;
pub mod latex_equation_generator;
pub mod latex_equation_provider;

use std::{ops::Add, time::Instant};

use nalgebra::{dvector, Point2, Vector2};
use p2d::bounding_volume::Aabb;
use piet::RenderContext;
use rnote_compose::{
    eventresult::EventPropagation, penevent::PenProgress, penpath::Element, shapes::Shapeable,
    EventResult, PenEvent,
};

use crate::{
    document::format::MeasureUnit,
    engine::{EngineView, EngineViewMut},
    store::StrokeKey,
    strokes::{latexstroke::LatexImage, Stroke, VectorImage},
    DrawableOnDoc, WidgetFlags,
};

use super::{
    width_picker::{WidthPickerContext, WidthPickerState},
    PenBehaviour, PenStyle,
};

#[derive(Debug, Clone)]
pub struct EquationInitialWidthPick {
    pub position: Vector2<f64>,
}

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
    InitialWidthPick(EquationInitialWidthPick),
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
    pub equation_width: Option<WidthPickerContext>,
}

impl Default for Latex {
    fn default() -> Self {
        Self {
            state: LatexState::Idle,
            reference: LatexReference::CreateNew,
            equation_width: None,
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
        element: &Element,
        engine_view: &mut EngineViewMut,
    ) -> LatexReference {
        if let Some(&stroke_key) = engine_view
            .store
            .stroke_hitboxes_contain_coord(engine_view.camera.viewport(), element.pos)
            .last()
        {
            let resolved = engine_view.store.get_stroke_ref(stroke_key);

            if let Some(stroke) = resolved {
                if let Stroke::LatexImage(_) = stroke {
                    return LatexReference::UpdateOld(LatexUpdateData {
                        old_latex_key: stroke_key,
                    });
                }
            }
        }

        LatexReference::CreateNew
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

                let mut equation_image = LatexImage::new(
                    &compile_instructions.equation_code,
                    &compile_instructions.svg_code,
                    &engine_view.pens_config.equation_config,
                    compile_instructions.position,
                    None,
                );

                if let Some(some_stroke) = stroke {
                    if let Stroke::LatexImage(old_latex_image) = some_stroke {
                        equation_image.copy_transform_preserve_position(&old_latex_image);
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
        // If an already existing equation has been clicked, edit that equation. Else, create a new one.
        let result = match (&event, &self.state) {
            (PenEvent::Down { element, .. }, LatexState::Idle) => {
                self.reference = Latex::determine_latex_reference(element, engine_view);

                // When updating, immediately open editor and skip dragging step
                let widget_flags = match &self.reference {
                    LatexReference::UpdateOld(old) => {
                        // Determine scale and rotation of current equation to accurately place the width picker widget.
                        // TODO: Make this a separate function
                        let position = if let Stroke::LatexImage(latex) = engine_view
                            .as_im()
                            .store
                            .get_stroke_ref(old.old_latex_key)
                            .unwrap()
                        {
                            let transformed_vector = latex
                                .access_rectangle()
                                .transform
                                .transform_vec(Vector2::new(1.0, 0.0));
                            let scale = transformed_vector.magnitude();
                            let direction = transformed_vector.normalize();
                            // Get upper left corner of rectangle
                            let upper_left = latex.access_rectangle().outline_lines()[0].start;
                            let transformed_position = Vector2::from(upper_left);

                            let px_value = MeasureUnit::convert_measurement(
                                latex.equation_config.page_width,
                                MeasureUnit::Mm,
                                engine_view.document.format.dpi(),
                                MeasureUnit::Px,
                                engine_view.document.format.dpi(),
                            );
                            self.equation_width = Some(WidthPickerContext::new(
                                transformed_position,
                                transformed_position + direction * scale * px_value,
                                transformed_position,
                                direction,
                                WidthPickerState::Idle,
                            ));

                            Some(latex.pos())
                        } else {
                            None
                        };

                        self.state = LatexState::ExpectingCode(LatexDrawInstruction {
                            initial_code: Latex::determine_initial_code(
                                &self.reference,
                                engine_view,
                            ),
                            position: position.unwrap(),
                        });

                        Latex::get_widget_flags_update()
                    }
                    LatexReference::CreateNew => {
                        self.equation_width = Some(WidthPickerContext::new(
                            element.pos,
                            element.pos,
                            element.pos,
                            Vector2::new(1.0, 0.0),
                            WidthPickerState::Dragging,
                        ));
                        self.state = LatexState::InitialWidthPick(EquationInitialWidthPick {
                            position: element.pos,
                        });

                        WidgetFlags::default()
                    }
                };

                (
                    EventResult {
                        handled: true,
                        propagate: EventPropagation::Stop,
                        progress: PenProgress::InProgress,
                    },
                    widget_flags,
                )
            }
            (PenEvent::Up { element, .. }, LatexState::InitialWidthPick(pos)) => {
                self.state = LatexState::ExpectingCode(LatexDrawInstruction {
                    initial_code: Latex::determine_initial_code(&self.reference, engine_view),
                    position: pos.position,
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
                    propagate: EventPropagation::Stop,
                    progress: PenProgress::InProgress,
                },
                WidgetFlags::default(),
            ),
        };

        if let Some(equation_width_some) = &mut self.equation_width {
            equation_width_some.update(&event);

            // Maybe shorten this down and make a let mut instead of trying to fit everyting into one expression, but that could result in me later forgetting some match arms...

            // This monstrosity determines the equation width (in px). It returns the raw equation width of the width widget, except when an equation is being updated:
            // If that is the case, the equation width will be scaled by (1 / <scale of the transform of the widget>).
            // TODO: Make this a separate function
            let magnitude = match &self.reference {
                LatexReference::CreateNew => equation_width_some.length(),
                LatexReference::UpdateOld(old) => {
                    match engine_view.store.get_stroke_ref(old.old_latex_key) {
                        Some(data) => {
                            if let Stroke::LatexImage(latex_image) = data {
                                let transformed_vector = latex_image
                                    .access_rectangle()
                                    .transform
                                    .transform_vec(Vector2::new(1.0, 0.0));
                                let scale = transformed_vector.magnitude();
                                equation_width_some.length() / scale
                            } else {
                                equation_width_some.length()
                            }
                        }
                        None => equation_width_some.length(),
                    }
                }
            };

            let mm_value = MeasureUnit::convert_measurement(
                magnitude,
                MeasureUnit::Px,
                engine_view.document.format.dpi(),
                MeasureUnit::Mm,
                engine_view.document.format.dpi(),
            );

            engine_view.pens_config.equation_config.page_width = mm_value;
        }

        result
    }
}

impl DrawableOnDoc for Latex {
    fn bounds_on_doc(&self, engine_view: &EngineView) -> Option<Aabb> {
        match &self.equation_width {
            Some(equation_width_some) => {
                Some(equation_width_some.aabb(engine_view.camera.total_zoom()))
            }
            None => None,
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

        if let Some(width_picker) = &self.equation_width {
            width_picker.draw_on_doc(cx, engine_view);
        }

        cx.restore().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(())
    }
}
