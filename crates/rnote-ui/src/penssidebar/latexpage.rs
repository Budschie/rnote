use std::borrow::Borrow;
use std::thread::panicking;

use crate::penssidebar::latexeditor::{
    LatexCodeCompilationResult, LatexEditorResultBoxed, RnLatexEditor,
};
// Imports
use crate::{RnAppWindow, RnCanvasWrapper};
use adw::prelude::*;
use cairo::glib::closure_local;
use gtk4::{glib, glib::clone, subclass::prelude::*, CompositeTemplate};
use gtk4::{SpinButton, Widget, Window};
use rnote_engine::pens::equation::equation_provider::EquationProvider;
use rnote_engine::pens::equation::latex_equation_provider::LatexEquationProvider;
use rnote_engine::pens::equation::{LatexCompiledInstruction, LatexReference, LatexState};
use rnote_engine::pens::pensconfig::equationconfig::EquationConfig;
use rnote_engine::pens::Pen;

use super::latexeditor::LatexEditorResult;

mod imp {
    use adw::{glib::WeakRef, ActionRow, OverlaySplitView};
    use gtk4::{Button, ListBox, MenuButton, Popover, SpinButton};

    use crate::RnSidebar;

    use super::*;

    #[derive(Default, Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/penssidebar/latexpage.ui")]
    pub(crate) struct RnLatexPage {
        #[template_child]
        pub(crate) equationtype_popover: TemplateChild<Popover>,
        #[template_child]
        pub(crate) equationtype_popover_close_button: TemplateChild<Button>,
        #[template_child]
        pub(crate) equationtype_latex_row: TemplateChild<ActionRow>,
        #[template_child]
        pub(crate) equationtype_listbox: TemplateChild<ListBox>,
        #[template_child]
        pub(crate) equationtype_menubutton: TemplateChild<MenuButton>,
        #[template_child]
        pub(crate) font_size_spinbutton: TemplateChild<SpinButton>,
        #[template_child]
        pub(crate) edit_equation: TemplateChild<Button>,
        #[template_child]
        pub(crate) compile_equation: TemplateChild<Button>,
        // TODO: Condense to one weak ref
        pub(crate) latex_editor: WeakRef<RnLatexEditor>,
        pub(crate) sidebar: WeakRef<RnSidebar>,
        pub(crate) split_view: WeakRef<OverlaySplitView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RnLatexPage {
        const NAME: &'static str = "RnLatexPage";
        type Type = super::RnLatexPage;
        type ParentType = gtk4::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RnLatexPage {
        fn constructed(&self) {
            self.parent_constructed();
        }

        fn dispose(&self) {
            self.dispose_template();
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for RnLatexPage {}
}

glib::wrapper! {
    pub(crate) struct RnLatexPage(ObjectSubclass<imp::RnLatexPage>)
        @extends gtk4::Widget;
}

impl Default for RnLatexPage {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TargetState {
    font_size: f64,
    equation_provider: EquationProvider,
}

impl TargetState {
    fn apply(&self, latex: &RnLatexPage) {
        latex.imp().font_size_spinbutton.set_value(self.font_size);
        latex.read_equation_type(&self.equation_provider);
    }
}

impl RnLatexPage {
    pub(crate) fn new() -> Self {
        glib::Object::new()
    }

    pub(crate) fn equation_type(&self) -> Option<EquationProvider> {
        let currently_selected_row = self.imp().equationtype_listbox.selected_row();

        if let Some(some_row) = currently_selected_row {
            return Some(match some_row.index() {
                0 => EquationProvider::LatexEquationProvider(LatexEquationProvider {}),
                _ => panic!("More than two rows are currently not implemented yet."),
            });
        }

        None
    }

    pub(crate) fn read_equation_type(&self, equation_provider: &EquationProvider) {
        match equation_provider {
            EquationProvider::LatexEquationProvider(_) => {
                self.imp()
                    .equationtype_listbox
                    .select_row(Some(&*self.imp().equationtype_latex_row));
            }
        }
    }

    fn read_settings_from_pen(&self, equation_config: &EquationConfig) -> TargetState {
        TargetState {
            font_size: f64::from(equation_config.font_size),
            equation_provider: equation_config.equation_provider.clone(),
        }
    }

    // TODO: Not elegant at all, find a way to remove this later
    fn determine_window_of_widget(widget: Widget) -> Window {
        let mut current_widget = widget;

        loop {
            let parent_widget = current_widget.parent();

            match parent_widget {
                Some(parent_widget_some) => current_widget = parent_widget_some.clone(),
                None => break,
            }
        }

        current_widget.downcast_ref::<Window>().unwrap().clone()
    }

    pub(crate) fn init(&self, appwindow: &RnAppWindow) {
        let imp = self.imp();

        let equationtype_popover = imp.equationtype_popover.get();

        imp.equationtype_popover_close_button.connect_clicked(
            clone!(@weak equationtype_popover => move |_| {
                equationtype_popover.popdown();
            }),
        );

        imp.font_size_spinbutton.connect_value_changed(clone!(@weak self as latexpage, @weak appwindow => move |spin_button| {
			appwindow.active_tab_wrapper().canvas().engine_mut().pens_config.equation_config.font_size = u32::try_from(latexpage.imp().font_size_spinbutton.value_as_int()).unwrap();
		}));
        imp.equationtype_listbox.connect_row_selected(clone!(@weak self as latexpage, @weak appwindow => move |_, _| {
			if let Some(equation_type) = latexpage.equation_type() {
				let icon_name = match equation_type {
					EquationProvider::LatexEquationProvider(_) => {
						"face-cool"
					}
				};

				latexpage.imp().equationtype_menubutton.set_icon_name(icon_name);
				appwindow.active_tab_wrapper().canvas().engine_mut().pens_config.equation_config.equation_provider = equation_type;
			}
		}));

        imp.edit_equation.connect_clicked(
            clone!(@weak self as latexpage, @weak appwindow => move |_| {
                latexpage.show_equation_editor();
            }),
        );

        imp.compile_equation.connect_clicked(clone!(@weak self as latexpage, @weak appwindow => move |_| {
			let mut request_compilation = false;

			if let Pen::Latex(latex) = appwindow.active_tab_wrapper().canvas().engine_mut().penholder.current_pen_ref() {
				if let LatexState::ExpectingCode(..) = &latex.state {
					request_compilation = true;
				}
			}

			if request_compilation {
				appwindow.sidebar().latex_editor().request_compilation();
			}
		}));

        appwindow.sidebar().latex_editor().connect_closure(
            "latex-editor-compiled",
            false,
            closure_local!(@weak-allow-none appwindow => move |_latex: &RnLatexEditor, result: &LatexEditorResultBoxed| {
				let appwindow_resolved = appwindow.unwrap();

                RnLatexPage::apply_result_to_pen(
                    appwindow_resolved.active_tab_wrapper().canvas().engine_mut().penholder.current_pen_mut(),
                    result,
                );
                let _ = appwindow_resolved.active_tab_wrapper().canvas().engine_mut().current_pen_update_state();
            })
        );

        appwindow.sidebar().latex_editor().connect_closure(
            "latex-editor-request-compilation",
            false,
            closure_local!(@weak-allow-none appwindow => move |_latex: &RnLatexEditor, equation_code: String| {
                LatexCodeCompilationResult::from(RnLatexPage::compile_equation_code(
                    &equation_code,
                    &appwindow.unwrap().active_tab_wrapper().canvas()
                        .engine_ref()
                        .pens_config
                        .equation_config,
                ))
            }),
        );

        // TODO: Move as much as possible into the appwindow so that weak references don't have to be stored here...
        self.imp()
            .latex_editor
            .set(Some(&appwindow.sidebar().latex_editor()));
        self.imp().sidebar.set(Some(&appwindow.sidebar()));
        self.imp().split_view.set(Some(&appwindow.split_view()));
    }

    fn apply_result_to_pen(current_pen: &mut Pen, result: &LatexEditorResultBoxed) {
        if let Pen::Latex(latex) = current_pen {
            if let LatexState::ExpectingCode(draw_instructions) = latex.state.clone() {
                match result.clone().inner() {
                    LatexEditorResult::Compiled(svg, code) => {
                        latex.state = LatexState::Finished(LatexCompiledInstruction {
                            equation_code: code,
                            svg_code: svg,
                            position: draw_instructions.position,
                        });
                    }
                    LatexEditorResult::Skip => latex.state = LatexState::Idle,
                }
            }
        }
    }

    fn compile_equation_code(
        equation_code: &String,
        equation_config: &EquationConfig,
    ) -> Result<String, String> {
        equation_config.generate_svg(equation_code)
    }

    fn show_equation_editor(&self) {
        self.imp()
            .split_view
            .upgrade()
            .unwrap()
            .set_show_sidebar(true);
        self.imp()
            .sidebar
            .upgrade()
            .unwrap()
            .sidebar_stack()
            .set_visible_child(&self.imp().latex_editor.upgrade().unwrap());
    }

    pub(crate) fn refresh_ui(&self, active_tab: &RnCanvasWrapper) {
        // println!("Refreshing UI");

        let mut editor_update_text: Option<String> = None;
        if let Pen::Latex(latex) = active_tab.canvas().engine_mut().penholder.current_pen_mut() {
            if let LatexState::ExpectingCode(expecting_code) = latex.state.clone() {
                // let editor = RnLatexEditor::new(&expecting_code.initial_code.clone());
                // TODO: Find better way of determining parent window.
                // I am not using the main window from the init function because GObjects
                // are ref-counted and I fear that this would introduce a reference cycle
                /*
                    editor.set_transient_for(Some(&RnLatexPage::determine_window_of_widget(
                        active_tab.upcast_ref::<Widget>().clone(),
                )));
                     */

                // Open code window if Creating new stuff

                if let LatexReference::CreateNew = &latex.reference {
                    self.show_equation_editor();
                }

                editor_update_text = Some(expecting_code.initial_code.clone());
            }
        };

        let target_state = self
            .read_settings_from_pen(&active_tab.canvas().engine_mut().pens_config.equation_config);
        target_state.apply(self);

        if let Some(new_latex_code) = editor_update_text {
            let latex_editor_resolved: RnLatexEditor = self.imp().latex_editor.upgrade().unwrap();
            latex_editor_resolved.set_latex_code(&new_latex_code)
        }
    }
}
