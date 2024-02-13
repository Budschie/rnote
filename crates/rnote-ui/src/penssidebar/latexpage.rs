use crate::penssidebar::latexeditor::{LatexCodeResult, LatexCodeResultBoxed, RnLatexEditor};
// Imports
use crate::{RnAppWindow, RnCanvasWrapper};
use crate::{RnCanvas, RnStrokeWidthPicker};
use adw::prelude::*;
use cairo::glib::closure_local;
use cairo::glib::gobject_ffi::GObject;
use gtk4::{glib, glib::clone, subclass::prelude::*, CompositeTemplate, ToggleButton};
use rnote_engine::pens::latex::{LatexCompileInstruction, LatexState};
use rnote_engine::pens::{Latex, Pen};

mod imp {
    use super::*;

    #[derive(Default, Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/penssidebar/latexpage.ui")]
    pub(crate) struct RnLatexPage {}

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

    pub(crate) fn init(&self, appwindow: &RnAppWindow) {}

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
                latex.state = LatexState::ReceivingCode(expecting_code);

                let editor = RnLatexEditor::new();
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
