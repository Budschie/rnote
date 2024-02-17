use crate::penssidebar::latexeditor::{LatexCodeResult, LatexCodeResultBoxed, RnLatexEditor};
// Imports
use crate::{RnAppWindow, RnCanvasWrapper};
use crate::{RnCanvas, RnStrokeWidthPicker};
use adw::prelude::*;
use anyhow::Context;
use cairo::glib::closure_local;
use cairo::glib::gobject_ffi::GObject;
use gtk4::{glib, glib::clone, subclass::prelude::*, CompositeTemplate, ToggleButton};
use rnote_engine::pens::latex::equation_provider::{EquationProvider, EquationProviderTrait};
use rnote_engine::pens::latex::latex_equation::LatexEquationProvider;
use rnote_engine::pens::latex::mathjax_equation_provider::MathJaxEquationProvider;
use rnote_engine::pens::latex::{LatexCompileInstruction, LatexState};
use rnote_engine::pens::{Latex, Pen};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, num_derive::FromPrimitive, num_derive::ToPrimitive,
)]
#[serde(rename = "equation_type")]
pub enum EquationType {
    #[serde(rename = "latex")]
    Latex,
    #[serde(rename = "math_jax")]
    MathJax,
}

impl TryFrom<u32> for EquationType {
    type Error = anyhow::Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        num_traits::FromPrimitive::from_u32(value)
            .with_context(|| format!("EquationType try_from::<u32>() for value {value} failed"))
    }
}

mod imp {
    use adw::ActionRow;
    use gtk4::{Button, ListBox, MenuButton, Popover};

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
        pub(crate) equationtype_mathjax_row: TemplateChild<ActionRow>,
        #[template_child]
        pub(crate) equationtype_listbox: TemplateChild<ListBox>,
        #[template_child]
        pub(crate) equationtype_menubutton: TemplateChild<MenuButton>,
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

impl RnLatexPage {
    pub(crate) fn new() -> Self {
        glib::Object::new()
    }

    pub(crate) fn equation_type(&self) -> Option<EquationType> {
        EquationType::try_from(self.imp().equationtype_listbox.selected_row()?.index() as u32).ok()
    }

    pub(crate) fn set_equation_type(&self, equation_type: EquationType) {
        match equation_type {
            EquationType::Latex => {
                self.imp()
                    .equationtype_listbox
                    .select_row(Some(&*self.imp().equationtype_latex_row));
            }
            EquationType::MathJax => {
                self.imp()
                    .equationtype_listbox
                    .select_row(Some(&*self.imp().equationtype_mathjax_row));
            }
        }
    }

    pub(crate) fn init(&self, appwindow: &RnAppWindow) {
        let imp = self.imp();

        let equationtype_popover = imp.equationtype_popover.get();

        self.set_equation_type(EquationType::Latex);

        imp.equationtype_popover_close_button.connect_clicked(
            clone!(@weak equationtype_popover => move |_| {
                equationtype_popover.popdown();
            }),
        );

        // For the changing icons
        imp.equationtype_listbox.connect_row_selected(clone!(@weak self as latexpage, @weak appwindow => move |_, _| {
			// Did not fall for funne cargo cult
			// TODO: Add equation_config

			if let Some(equation_type) = latexpage.equation_type() {
				let (icon_name, equation_provider) = match equation_type {
					EquationType::Latex => {
						("face-cool", EquationProvider::LatexEquationProvider(LatexEquationProvider{}))
					}
					EquationType::MathJax => {
						("face-angel", EquationProvider::MathJaxEquationProvider(MathJaxEquationProvider{}))
					}
				};

				latexpage.imp().equationtype_menubutton.set_icon_name(icon_name);

				if let Pen::Latex(latex) = appwindow.active_tab_wrapper().canvas().engine_mut().penholder.current_pen_mut() {
						latex.equation_provider = equation_provider;
					}
			}
		}));
    }

    fn apply_result_to_pen(current_pen: &mut Pen, result: &LatexCodeResultBoxed) {
        if let Pen::Latex(latex) = current_pen {
            if let LatexState::ReceivingCode(draw_instructions) = latex.state.clone() {
                match result.clone().inner() {
                    LatexCodeResult::Compile(code) => {
                        latex.state = LatexState::Finished(LatexCompileInstruction {
                            code,
                            position: draw_instructions.position,
                        })
                    }
                    LatexCodeResult::Skip => latex.state = LatexState::Idle,
                }
            }
        }
    }

    pub(crate) fn refresh_ui(&self, active_tab: &RnCanvasWrapper) {
        let imp = self.imp();

        if let Pen::Latex(latex) = active_tab.canvas().engine_mut().penholder.current_pen_mut() {
            if let LatexState::ExpectingCode(expecting_code) = latex.state.clone() {
                let editor = RnLatexEditor::new(&expecting_code.initial_code);
                latex.state = LatexState::ReceivingCode(expecting_code);
                editor.present();

                let borrowed_canvas = active_tab.canvas();
                editor.connect_closure(
                    "latex-editor-result",
                    false,
                    closure_local!(|_latex: &RnLatexEditor, result: &LatexCodeResultBoxed| {
                        RnLatexPage::apply_result_to_pen(
                            borrowed_canvas.engine_mut().penholder.current_pen_mut(),
                            result,
                        );
                        let _ = borrowed_canvas.engine_mut().current_pen_update_state();
                    }),
                );
            }
        }
    }
}
