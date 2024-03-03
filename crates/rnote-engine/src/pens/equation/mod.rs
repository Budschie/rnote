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
    strokes::{equationstroke::EquationImage, Stroke, VectorImage},
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
pub struct EquationCompiledInstruction {
    pub equation_code: String,
    pub svg_code: String,
    pub position: Vector2<f64>,
}

#[derive(Debug, Clone)]
pub struct EquationDrawInstruction {
    pub initial_code: String,
    pub position: Vector2<f64>,
}

#[derive(Debug, Clone)]
pub struct EquationUpdateData {
    pub old_equation_key: StrokeKey,
}

#[derive(Debug, Clone)]
pub enum EquationState {
    Idle,
    InitialWidthPick(EquationInitialWidthPick),
    ExpectingCode(EquationDrawInstruction),
    Finished(EquationCompiledInstruction),
}

// CreateNew means that a new equation object will be created; UpdateOld will update an already existing one
#[derive(Debug, Clone)]
pub enum EquationReference {
    CreateNew,
    UpdateOld(EquationUpdateData),
}

#[derive(Debug, Clone)]
pub struct Equation {
    pub state: EquationState,
    pub reference: EquationReference,
    pub equation_width: Option<WidthPickerContext>,
}

impl Default for Equation {
    fn default() -> Self {
        Self {
            state: EquationState::Idle,
            reference: EquationReference::CreateNew,
            equation_width: None,
        }
    }
}

impl Equation {
    fn get_widget_flags_update() -> WidgetFlags {
        let mut default_widget_flags = WidgetFlags::default();
        default_widget_flags.refresh_ui = true;
        default_widget_flags
    }

    fn determine_equation_reference(
        element: &Element,
        engine_view: &mut EngineViewMut,
    ) -> EquationReference {
        if let Some(&stroke_key) = engine_view
            .store
            .stroke_hitboxes_contain_coord(engine_view.camera.viewport(), element.pos)
            .last()
        {
            let resolved = engine_view.store.get_stroke_ref(stroke_key);

            if let Some(stroke) = resolved {
                if let Stroke::EquationImage(_) = stroke {
                    return EquationReference::UpdateOld(EquationUpdateData {
                        old_equation_key: stroke_key,
                    });
                }
            }
        }

        EquationReference::CreateNew
    }

    fn determine_initial_code(
        equation_reference: &EquationReference,
        engine_view: &mut EngineViewMut,
    ) -> String {
        match equation_reference {
            EquationReference::CreateNew => String::from(""),
            EquationReference::UpdateOld(update_data) => {
                let stroke = engine_view
                    .store
                    .get_stroke_ref(update_data.old_equation_key)
                    .unwrap();

                if let Stroke::EquationImage(equation_image) = stroke {
                    equation_image.equation_code.clone()
                } else {
                    // TODO: Warn about the fact that the EquationReference is not valid here
                    String::from("")
                }
            }
        }
    }
}

impl PenBehaviour for Equation {
    fn init(&mut self, _engine_view: &EngineView) -> WidgetFlags {
        WidgetFlags::default()
    }

    fn deinit(&mut self) -> WidgetFlags {
        WidgetFlags::default()
    }

    fn style(&self) -> PenStyle {
        PenStyle::Equation
    }

    fn update_state(&mut self, engine_view: &mut EngineViewMut) -> WidgetFlags {
        let mut done_data: Option<(EquationState, EquationReference)> = None;

        match &self.state {
            EquationState::Finished(compile_instructions) => {
                let mut stroke: Option<Stroke> = None;
                let draw_instruction = EquationDrawInstruction {
                    initial_code: compile_instructions.equation_code.clone(),
                    position: compile_instructions.position,
                };

                if let EquationReference::UpdateOld(update_data) = &self.reference {
                    stroke = engine_view
                        .store
                        .remove_stroke(update_data.old_equation_key);
                }

                let mut equation_image = EquationImage::new(
                    &compile_instructions.equation_code,
                    &compile_instructions.svg_code,
                    &engine_view.pens_config.equation_config,
                    compile_instructions.position,
                    None,
                );

                if let Some(some_stroke) = stroke {
                    if let Stroke::EquationImage(old_equation_image) = some_stroke {
                        equation_image.copy_transform_preserve_position(&old_equation_image);
                    }
                }

                let equation_key = engine_view
                    .store
                    .insert_stroke(Stroke::EquationImage(equation_image), None);

                engine_view.store.record(Instant::now());

                engine_view.store.regenerate_rendering_in_viewport_threaded(
                    engine_view.tasks_tx.clone(),
                    true,
                    engine_view.camera.viewport(),
                    engine_view.camera.image_scale(),
                );

                done_data = Some((
                    EquationState::ExpectingCode(draw_instruction),
                    EquationReference::UpdateOld(EquationUpdateData {
                        old_equation_key: equation_key,
                    }),
                ));
            }
            _ => {}
        }

        if let Some((new_equation_state, new_equation_reference)) = done_data {
            self.state = new_equation_state;
            self.reference = new_equation_reference;
            Equation::get_widget_flags_update()
        } else {
            WidgetFlags::default()
        }
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
        let initial_event_propagation = if let Some(equation_width_some) = &mut self.equation_width
        {
            equation_width_some.update(&event)
        } else {
            EventPropagation::Proceed
        };

        // If an already existing equation has been clicked, edit that equation. Else, create a new one.
        let result = match (&event, &self.state, initial_event_propagation) {
            (
                PenEvent::Down { element, .. },
                EquationState::Idle | EquationState::ExpectingCode(..),
                EventPropagation::Proceed,
            ) => {
                self.reference = Equation::determine_equation_reference(element, engine_view);

                // When updating, immediately open editor and skip dragging step
                let widget_flags = match &self.reference {
                    EquationReference::UpdateOld(old) => {
                        // Determine scale and rotation of current equation to accurately place the width picker widget.
                        // TODO: Make this a separate function
                        let position = if let Stroke::EquationImage(equation) = engine_view
                            .store
                            .get_stroke_mut(old.old_equation_key)
                            .unwrap()
                        {
                            let transformed_vector = equation
                                .access_rectangle()
                                .transform
                                .transform_vec(Vector2::new(1.0, 0.0));
                            let scale = transformed_vector.magnitude();
                            let direction = transformed_vector.normalize();
                            // Get upper left corner of rectangle
                            let upper_left = equation.access_rectangle().outline_lines()[0].start;
                            let transformed_position = Vector2::from(upper_left);

                            let px_value = MeasureUnit::convert_measurement(
                                equation.equation_config.page_width,
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

                            engine_view.pens_config.equation_config =
                                equation.equation_config.clone();

                            Some(equation.pos())
                        } else {
                            None
                        };

                        self.state = EquationState::ExpectingCode(EquationDrawInstruction {
                            initial_code: Equation::determine_initial_code(
                                &self.reference,
                                engine_view,
                            ),
                            position: position.unwrap(),
                        });

                        Equation::get_widget_flags_update()
                    }
                    EquationReference::CreateNew => {
                        self.equation_width = Some(WidthPickerContext::new(
                            element.pos,
                            element.pos,
                            element.pos,
                            Vector2::new(1.0, 0.0),
                            WidthPickerState::Dragging,
                        ));
                        self.state = EquationState::InitialWidthPick(EquationInitialWidthPick {
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
            (
                PenEvent::Up { element, .. },
                EquationState::InitialWidthPick(pos),
                EventPropagation::Proceed,
            ) => {
                self.state = EquationState::ExpectingCode(EquationDrawInstruction {
                    initial_code: Equation::determine_initial_code(&self.reference, engine_view),
                    position: pos.position,
                });

                (
                    EventResult {
                        handled: true,
                        propagate: EventPropagation::Stop,
                        progress: PenProgress::InProgress,
                    },
                    Equation::get_widget_flags_update(),
                )
            }
            (_, _, _) => (
                EventResult {
                    handled: false,
                    propagate: EventPropagation::Stop,
                    progress: PenProgress::InProgress,
                },
                WidgetFlags::default(),
            ),
        };

        if let Some(equation_width_some) = &mut self.equation_width {
            // Maybe shorten this down and make a let mut instead of trying to fit everyting into one expression, but that could result in me later forgetting some match arms...

            // This monstrosity determines the equation width (in px). It returns the raw equation width of the width widget, except when an equation is being updated:
            // If that is the case, the equation width will be scaled by (1 / <scale of the transform of the widget>).
            // TODO: Make this a separate function
            let magnitude = match &self.reference {
                EquationReference::CreateNew => equation_width_some.length(),
                EquationReference::UpdateOld(old) => {
                    match engine_view.store.get_stroke_ref(old.old_equation_key) {
                        Some(data) => {
                            if let Stroke::EquationImage(equation_image) = data {
                                let transformed_vector = equation_image
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

impl DrawableOnDoc for Equation {
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
            EquationState::Idle => {}
            _ => {}
        }

        if let Some(width_picker) = &self.equation_width {
            width_picker.draw_on_doc(cx, engine_view);
        }

        cx.restore().map_err(|e| anyhow::anyhow!("{e:?}"))?;
        Ok(())
    }
}
