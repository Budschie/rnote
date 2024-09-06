use std::borrow::BorrowMut;
use std::cell::{Ref, RefMut};
use std::ops::DerefMut;

use crate::penssidebar::equationeditor::{EquationCodeCompilationResult, RnEquationEditor};
// Imports
use crate::{RnAppWindow, RnCanvas, RnCanvasWrapper};
use adw::glib::GString;
use adw::prelude::*;
use cairo::glib::closure_local;
use gtk4::{glib, glib::clone, subclass::prelude::*, CompositeTemplate};
use gtk4::{Widget, Window};
use rnote_engine::engine::EngineViewMut;
use rnote_engine::pens::equation::equation_provider::EquationProvider;
use rnote_engine::pens::equation::executable_checks::LatexExecutableChecker;
use rnote_engine::pens::equation::latex_equation_provider::LatexEquationProvider;
use rnote_engine::pens::equation::{EquationCompilationPolicy, EquationState};
use rnote_engine::pens::pensconfig::equationconfig::EquationConfig;
use rnote_engine::pens::{Equation, Pen};
use rnote_engine::store::StrokeKey;
use rnote_engine::Engine;

mod imp {
    use adw::{glib::WeakRef, ActionRow, OverlaySplitView};
    use gtk4::{Button, ListBox, MenuButton, Popover, SpinButton};

    use crate::RnSidebar;

    use super::*;

    #[derive(Default, Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/penssidebar/equationpage.ui")]
    pub(crate) struct RnEquationPage {
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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RnEquationPage {
        const NAME: &'static str = "RnEquationPage";
        type Type = super::RnEquationPage;
        type ParentType = gtk4::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RnEquationPage {
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

    impl WidgetImpl for RnEquationPage {}
}

glib::wrapper! {
    pub(crate) struct RnEquationPage(ObjectSubclass<imp::RnEquationPage>)
        @extends gtk4::Widget;
}

impl Default for RnEquationPage {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TargetState {
    font_size: f64,
    equation_provider: EquationProvider,
}

impl TargetState {
    fn apply(&self, equation: &RnEquationPage) {
        equation
            .imp()
            .font_size_spinbutton
            .set_value(self.font_size);
        equation.read_equation_type(&self.equation_provider);
    }
}

impl RnEquationPage {
    pub(crate) fn new() -> Self {
        glib::Object::new()
    }

    pub(crate) fn equation_type(&self) -> Option<EquationProvider> {
        let currently_selected_row = self.imp().equationtype_listbox.selected_row();

        if let Some(some_row) = currently_selected_row {
            return Some(match some_row.index() {
                0 => EquationProvider::LatexEquationProvider(
                    LatexEquationProvider {},
                    LatexExecutableChecker {},
                ),
                _ => panic!("More than one row is currently not implemented yet."),
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

    fn update_penconfig(&self, appwindow: &RnAppWindow) {
        appwindow
            .active_tab_wrapper()
            .canvas()
            .engine_mut()
            .mark_updated_penconfig();
    }

    fn update_text(&self, appwindow: &RnAppWindow, new_equation_code: String) {
        appwindow
            .active_tab_wrapper()
            .canvas()
            .engine_mut()
            .mark_updated_text(new_equation_code);
    }

    pub(crate) fn init(&self, appwindow: &RnAppWindow) {
        let imp = self.imp();

        let equationtype_popover = imp.equationtype_popover.get();

        imp.equationtype_popover_close_button.connect_clicked(
            clone!(@weak equationtype_popover => move |_| {
                equationtype_popover.popdown();
            }),
        );

        imp.font_size_spinbutton.connect_value_changed(clone!(@weak self as equationpage, @weak appwindow => move |spin_button| {
			appwindow.active_tab_wrapper().canvas().engine_mut().pens_config.equation_config.font_size = u32::try_from(equationpage.imp().font_size_spinbutton.value_as_int()).unwrap();
			equationpage.update_penconfig(&appwindow);
		}));
        imp.equationtype_listbox.connect_row_selected(clone!(@weak self as equationpage, @weak appwindow => move |_, _| {
			if let Some(equation_type) = equationpage.equation_type() {
				let icon_name = match equation_type {
					EquationProvider::LatexEquationProvider(_) => {
						"face-cool"
					}
				};

				equationpage.imp().equationtype_menubutton.set_icon_name(icon_name);
				appwindow.active_tab_wrapper().canvas().engine_mut().pens_config.equation_config.equation_provider = equation_type;

				equationpage.update_penconfig(&appwindow);
			}
		}));

        imp.edit_equation.connect_clicked(
            clone!(@weak self as equationpage, @weak appwindow => move |_| {
                appwindow.show_equation_sidebar();
            }),
        );

        imp.compile_equation.connect_clicked(clone!(@weak self as equationpage, @weak appwindow => move |button| {
			let mut inverted_option: Option<EquationCompilationPolicy> = None;

			if let Ok(policy) = appwindow.active_tab_wrapper().canvas().engine_mut().get_equation_compilation_policy() {
				inverted_option = Some(EquationCompilationPolicy::invert(policy.clone()));
			}

			if let Some(inverted) = inverted_option {
				let result = appwindow.active_tab_wrapper().canvas().engine_mut().set_equation_compilation_policy(inverted);

				if let Ok(_) = result {
					equationpage.adjust_compilation_graphics(&mut appwindow.active_tab_wrapper().canvas().engine_mut());
				}
			}
		}));

        appwindow.sidebar().equation_editor().connect_closure(
            "equation-editor-request-compilation",
            false,
            closure_local!(@weak-allow-none appwindow, @weak-allow-none self as equationpage => move |_equation: &RnEquationEditor, equation_code: String| {
                equationpage.unwrap().update_text(&appwindow.unwrap(), equation_code);

				EquationCodeCompilationResult::from(Result::Ok(String::from("nothing")))
            }),
        );
    }

    fn set_compilation_graphics(&self, compilation_policy: EquationCompilationPolicy) {
        match compilation_policy {
            EquationCompilationPolicy::Allow => self.display_pause_compilation_graphics(),
            EquationCompilationPolicy::Deny => self.display_play_compilation_graphics(),
        }
    }

    fn adjust_compilation_graphics(&self, engine: &mut RefMut<Engine>) {
        if let Ok(policy) = engine.get_equation_compilation_policy() {
            self.set_compilation_graphics(policy.clone());
        }
    }

    fn display_pause_compilation_graphics(&self) {
        self.display_compilation_graphics(
            "media-playback-pause",
            &["destructive-action", "sidebar_action_button"],
        );
    }

    fn display_play_compilation_graphics(&self) {
        self.display_compilation_graphics(
            "media-playback-start",
            &["suggested-action", "sidebar_action_button"],
        );
    }

    fn display_compilation_graphics(&self, icon_name: &str, classes: &[&str]) {
        self.imp().compile_equation.set_icon_name(icon_name);
        self.imp().compile_equation.css_classes().clear();
        self.imp().compile_equation.set_css_classes(classes);
    }

    pub(crate) fn refresh_ui(&self, active_tab: &RnCanvasWrapper) {
        let target_state = self
            .read_settings_from_pen(&active_tab.canvas().engine_ref().pens_config.equation_config);

        self.read_equation_type(&target_state.equation_provider);
        self.imp()
            .font_size_spinbutton
            .set_value(target_state.font_size);

        // TODO: Replace this with more direct logic to directly retrieve the compilation policy
        self.display_play_compilation_graphics();
    }
}
