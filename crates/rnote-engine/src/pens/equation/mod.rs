pub mod equation_compiler;
pub mod equation_provider;
pub mod latex_equation_generator;
pub mod latex_equation_provider;

use std::{sync::Arc, thread, time::Duration};

use nalgebra::Vector2;
use p2d::bounding_volume::Aabb;
use piet::RenderContext;
use rnote_compose::{
    eventresult::EventPropagation,
    penevent::PenProgress,
    penpath::Element,
    shapes::{Rectangle, Shapeable},
    EventResult, PenEvent,
};

use crate::{
    document::format::MeasureUnit,
    engine::{EngineView, EngineViewMut},
    store::StrokeKey,
    strokes::{
        equationstroke::{EquationImage, DEFAULT_SVG_CODE},
        Stroke, VectorImage,
    },
    tasks::PeriodicTaskHandle,
    DrawableOnDoc, StrokeStore, WidgetFlags,
};

use self::equation_compiler::{
    CompilationTask, EquationCompilerMainThread, EquationCompilerTask, EquationCompilerTaskSender,
};

use super::{
    width_picker::{WidthPickerContext, WidthPickerState},
    PenBehaviour, PenStyle, PensConfig,
};

#[derive(Debug, Clone)]
pub struct CompilationInfo {
    pub stroke_key: StrokeKey,
    pub equation_compilation_policy: EquationCompilationPolicy,
}

#[derive(Debug, Clone)]
pub enum EquationState {
    Idle,
    InitialWidthPick(Vector2<f64>),
    AwaitCompilation(CompilationInfo),
}

#[derive(Debug, Clone)]
pub enum EquationCompilationPolicy {
    Allow,
    Deny,
}

pub const COMPILATION_DURATION: Duration = Duration::from_millis(1000);

#[derive(Debug)]
pub struct Equation {
    pub state: EquationState,
    pub equation_width: Option<WidthPickerContext>,
    equation_compiler_handle: Option<PeriodicTaskHandle>,
    equation_compiler: Option<EquationCompilerMainThread>,
    last_widget_event_propagation: EventPropagation,
}

impl Default for Equation {
    fn default() -> Self {
        Self {
            state: EquationState::Idle,
            equation_width: None,
            equation_compiler_handle: None,
            equation_compiler: None,
            last_widget_event_propagation: EventPropagation::Proceed,
        }
    }
}

impl Equation {
    /// Returns the stroke which is currently being hovered, but only if it is an EquationStroke
    fn determine_equation_reference(
        element: &Element,
        engine_view: &mut EngineViewMut,
    ) -> Option<StrokeKey> {
        if let Some(&stroke_key) = engine_view
            .store
            .stroke_hitboxes_contain_coord(engine_view.camera.viewport(), element.pos)
            .last()
        {
            let resolved = engine_view.store.get_stroke_ref(stroke_key);

            if let Some(stroke) = resolved {
                if let Stroke::EquationImage(_) = stroke {
                    return Some(stroke_key);
                }
            }
        }

        None
    }

    fn create_width_picker_for_rectangle(
        rectangle: &Rectangle,
        equation_width: f64,
        dpi: f64,
    ) -> WidthPickerContext {
        let transformed_vector = rectangle.transform.transform_vec(Vector2::new(1.0, 0.0));
        let scale = transformed_vector.magnitude();
        let direction = transformed_vector.normalize();
        // Get upper left corner of rectangle
        let upper_left = rectangle.outline_lines()[0].start;
        let transformed_position = Vector2::from(upper_left);

        let px_value = MeasureUnit::convert_measurement(
            equation_width,
            MeasureUnit::Mm,
            dpi,
            MeasureUnit::Px,
            dpi,
        );

        WidthPickerContext::new(
            transformed_position,
            transformed_position + direction * scale * px_value,
            transformed_position,
            direction,
            WidthPickerState::Idle,
        )
    }

    /// Submits a compilation request when the EquationCompilationPolicy is set to allow compilation
    pub fn check_equation_compilation(&self, stroke_store: &StrokeStore) {
        if let EquationState::AwaitCompilation(compilation_info) = &self.state {
            if let EquationCompilationPolicy::Allow = compilation_info.equation_compilation_policy {
                if let Some(stroke) = stroke_store.get_stroke_ref(compilation_info.stroke_key) {
                    if let Stroke::EquationImage(equation) = stroke {
                        self.equation_compiler
                            .as_ref()
                            .unwrap()
                            .tx
                            .as_ref()
                            .unwrap()
                            .send(EquationCompilerTask::Compile(
                                compilation_info.stroke_key,
                                CompilationTask::new(equation),
                            ));
                    }
                }
            }
        }
    }

    pub fn get_current_equation_code(
        stroke_key: StrokeKey,
        stroke_store: &StrokeStore,
    ) -> Option<String> {
        if let Some(some_stroke) = stroke_store.get_stroke_ref(stroke_key) {
            if let Stroke::EquationImage(equation) = some_stroke {
                return Some(equation.equation_code.clone());
            }
        }

        None
    }

    fn process_marked_updated_penconfig(
        &self,
        stroke_store: &mut StrokeStore,
        pens_config: &mut PensConfig,
        compilation_info: &CompilationInfo,
    ) {
        if let Some(some_stroke) = stroke_store.get_stroke_mut(compilation_info.stroke_key) {
            if let Stroke::EquationImage(equation) = some_stroke {
                equation.equation_config = pens_config.equation_config.clone();
            }
        }
    }

    fn process_marked_updated_text(
        &self,
        stroke_store: &mut StrokeStore,
        compilation_info: &CompilationInfo,
        equation_code: String,
    ) {
        if let Some(some_stroke) = stroke_store.get_stroke_mut(compilation_info.stroke_key) {
            if let Stroke::EquationImage(equation) = some_stroke {
                equation.equation_code = equation_code;
            }
        }
    }

    /// Updates the penconfig and marks it as such
    pub fn mark_updated_penconfig(
        &self,
        stroke_store: &mut StrokeStore,
        pens_config: &mut PensConfig,
    ) {
        if let EquationState::AwaitCompilation(compilation_info) = &self.state {
            self.process_marked_updated_penconfig(stroke_store, pens_config, &compilation_info);
        }
    }

    /// Updates the text and marks it as such using a String slice
    pub fn mark_updated_text_str_ref(
        &self,
        stroke_store: &mut StrokeStore,
        new_equation_code: &str,
    ) {
        if let EquationState::AwaitCompilation(compilation_info) = &self.state {
            self.process_marked_updated_text(
                stroke_store,
                &compilation_info,
                String::from(new_equation_code),
            );
        }
    }

    /// Updates the text and marks it as such using a String
    pub fn mark_updated_text(&self, stroke_store: &mut StrokeStore, new_equation_code: String) {
        if let EquationState::AwaitCompilation(compilation_info) = &self.state {
            self.process_marked_updated_text(stroke_store, &compilation_info, new_equation_code);
        }
    }

    /// Sets the equation compilation policy
    fn enable_compilation(&mut self, stroke_store: &mut StrokeStore) {
        if let EquationState::AwaitCompilation(compilation_info) = &mut self.state {
            if let EquationCompilationPolicy::Deny =
                compilation_info.equation_compilation_policy.clone()
            {
                compilation_info.equation_compilation_policy = EquationCompilationPolicy::Allow;
                self.check_equation_compilation(&stroke_store);
            }
        }
    }

    /// Disables compilation of the equation
    fn disable_compilation(&mut self, stroke_store: &mut StrokeStore) {
        if let EquationState::AwaitCompilation(compilation_info) = &mut self.state {
            compilation_info.equation_compilation_policy = EquationCompilationPolicy::Deny;
        }
    }

    /// Toggles the compilation of the equation. Returns whether the equation may be compiled after the compilation has been toggled.
    pub fn toggle_compilation(
        &mut self,
        stroke_store: &mut StrokeStore,
    ) -> Option<EquationCompilationPolicy> {
        let should_enable =
            if let EquationState::AwaitCompilation(compilation_info) = &mut self.state {
                match compilation_info.equation_compilation_policy {
                    EquationCompilationPolicy::Allow => false,
                    EquationCompilationPolicy::Deny => true,
                }
            } else {
                return None;
            };

        match should_enable {
            true => {
                self.enable_compilation(stroke_store);
                Some(EquationCompilationPolicy::Allow)
            }
            false => {
                self.disable_compilation(stroke_store);
                Some(EquationCompilationPolicy::Deny)
            }
        }
    }
}

impl PenBehaviour for Equation {
    fn init(&mut self, engine_view: &EngineView) -> WidgetFlags {
        let tasks_tx = engine_view.tasks_tx.clone();
        let compile_task = move || -> crate::tasks::PeriodicTaskResult {
            tasks_tx.send(crate::engine::EngineTask::CheckEquationCompilation);
            crate::tasks::PeriodicTaskResult::Continue
        };
        self.equation_compiler_handle = Some(crate::tasks::PeriodicTaskHandle::new(
            compile_task,
            COMPILATION_DURATION,
        ));

        self.equation_compiler = engine_view.equation_compiler.clone();

        WidgetFlags::default()
    }

    fn deinit(&mut self) -> WidgetFlags {
        self.equation_compiler_handle = None;

        WidgetFlags::default()
    }

    fn style(&self) -> PenStyle {
        PenStyle::Equation
    }

    fn update_state(&mut self, engine_view: &mut EngineViewMut) -> WidgetFlags {
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
        let mut widget_flags = WidgetFlags::default();

        let initial_event_propagation = if let Some(equation_width_some) = &mut self.equation_width
        {
            equation_width_some.update(&event)
        } else {
            EventPropagation::Proceed
        };

        let result = match (&event, &self.state, initial_event_propagation) {
            (PenEvent::Down { element, .. }, EquationState::Idle, EventPropagation::Proceed) => {
                let currently_hovered =
                    Equation::determine_equation_reference(element, engine_view);

                match currently_hovered {
                    Some(stroke_key) => {
                        if let Stroke::EquationImage(equation) =
                            engine_view.store.get_stroke_mut(stroke_key).unwrap()
                        {
                            self.equation_width =
                                Some(Equation::create_width_picker_for_rectangle(
                                    equation.access_rectangle(),
                                    equation.equation_config.page_width,
                                    engine_view.document.format.dpi(),
                                ));

                            engine_view.pens_config.equation_config =
                                equation.equation_config.clone();

                            // Transition to awaiting compilation
                            self.state = EquationState::AwaitCompilation(CompilationInfo {
                                stroke_key,
                                equation_compilation_policy: EquationCompilationPolicy::Deny,
                            });
                            widget_flags.refresh_equation_ui = true;
                        }
                    }
                    None => {
                        self.equation_width = Some(WidthPickerContext::new(
                            element.pos,
                            element.pos,
                            element.pos,
                            Vector2::new(1.0, 0.0),
                            WidthPickerState::Dragging,
                        ));

                        self.state = EquationState::InitialWidthPick(element.pos);
                        widget_flags.refresh_ui = true;
                    }
                }

                EventResult {
                    handled: false,
                    propagate: EventPropagation::Stop,
                    progress: PenProgress::InProgress,
                }
            }

            (
                PenEvent::Up { element, .. },
                EquationState::InitialWidthPick(position),
                EventPropagation::Proceed,
            ) => {
                let equation_image = EquationImage::new(
                    "",
                    &engine_view.pens_config.equation_config,
                    *position,
                    None,
                );
                let stroke_key = engine_view
                    .store
                    .insert_stroke(Stroke::EquationImage(equation_image), None);
                widget_flags |= engine_view.store.record(now);
                engine_view.store.regenerate_rendering_in_viewport_threaded(
                    engine_view.tasks_tx.clone(),
                    true,
                    engine_view.camera.viewport(),
                    engine_view.camera.image_scale(),
                );
                self.state = EquationState::AwaitCompilation(CompilationInfo {
                    stroke_key,
                    equation_compilation_policy: EquationCompilationPolicy::Deny,
                });

                widget_flags.show_equation_sidebar_ui = true;
                widget_flags.refresh_equation_ui = true;

                EventResult {
                    handled: false,
                    propagate: EventPropagation::Stop,
                    progress: PenProgress::InProgress,
                }
            }

            (_, _, _) => EventResult {
                handled: false,
                propagate: EventPropagation::Stop,
                progress: PenProgress::InProgress,
            },
        };

        // Only perform this at the end of dragging
        if let (EventPropagation::Stop, EventPropagation::Proceed) = (
            self.last_widget_event_propagation,
            initial_event_propagation,
        ) {
            if let Some(equation_width_some) = &mut self.equation_width {
                // Maybe shorten this down and make a let mut instead of trying to fit everyting into one expression, but that could result in me later forgetting some match arms...

                // This monstrosity determines the equation width (in px). It returns the raw equation width of the width widget, except when an equation is being updated:
                // If that is the case, the equation width will be scaled by (1 / <scale of the transform of the widget>).
                // TODO: Make this a separate function
                let mut magnitude = equation_width_some.length();

                let stroke_key = match &self.state {
                    EquationState::AwaitCompilation(compilation_info) => {
                        Some(compilation_info.stroke_key)
                    }
                    _ => None,
                };

                if let Some(some_stroke_key) = stroke_key {
                    if let Some(some_stroke) = engine_view.store.get_stroke_ref(some_stroke_key) {
                        if let Stroke::EquationImage(equation) = some_stroke {
                            let transformed_vector = equation
                                .access_rectangle()
                                .transform
                                .transform_vec(Vector2::new(1.0, 0.0));
                            let scale = transformed_vector.magnitude();

                            magnitude /= scale;
                        }
                    }
                }

                let mm_value = MeasureUnit::convert_measurement(
                    magnitude,
                    MeasureUnit::Px,
                    engine_view.document.format.dpi(),
                    MeasureUnit::Mm,
                    engine_view.document.format.dpi(),
                );

                engine_view.pens_config.equation_config.page_width = mm_value;

                self.mark_updated_penconfig(engine_view.store, engine_view.pens_config);
            }
        }

        self.last_widget_event_propagation = initial_event_propagation;

        (result, widget_flags)
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
