use adw::ffi::AdwWindow;
use adw::prelude::*;
use adw::Window;
use gtk4::ffi::GtkWidget;
use gtk4::ffi::GtkWindow;
use gtk4::{glib, glib::clone, subclass::prelude::*, CompositeTemplate, ToggleButton};

#[derive(Debug, Clone)]
pub enum LatexCodeResult {
    Skip,
    Compile(String),
}

#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "LatexCodeResultBoxed")]
pub struct LatexCodeResultBoxed(LatexCodeResult);

impl From<LatexCodeResult> for LatexCodeResultBoxed {
    fn from(value: LatexCodeResult) -> Self {
        Self(value)
    }
}

impl From<LatexCodeResultBoxed> for LatexCodeResult {
    fn from(value: LatexCodeResultBoxed) -> Self {
        value.0
    }
}

impl LatexCodeResultBoxed {
    pub(crate) fn inner(self) -> LatexCodeResult {
        self.0
    }
}

mod imp {

    use std::sync::OnceLock;

    use adw::subclass::{application_window::AdwApplicationWindowImpl, window::AdwWindowImpl};
    use cairo::glib::{gobject_ffi::GObject, subclass::Signal};
    use gtk4::{Button, TextBuffer, TextView};

    use super::*;
    #[derive(Default, Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/latexeditor.ui")]
    pub(crate) struct RnLatexEditor {
        #[template_child]
        pub(crate) latex_code: TemplateChild<TextBuffer>,
        #[template_child]
        pub(crate) compile_button: TemplateChild<Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RnLatexEditor {
        const NAME: &'static str = "RnLatexEditor";
        type Type = super::RnLatexEditor;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RnLatexEditor {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let latex_code = self.latex_code.clone();

            self.compile_button.connect_clicked(clone!(@strong obj, @strong latex_code => move |_button| {
				let text = latex_code.text(&latex_code.start_iter(), &latex_code.end_iter(), true);
				obj.emit_by_name::<()>("latex-editor-result", &[&LatexCodeResultBoxed(LatexCodeResult::Compile(String::from(text)))]);
				obj.close();
			}));
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }

            // TODO: This isn't being called (or not properly handled), figure out why
            self.obj().emit_by_name::<()>(
                "latex-editor-result",
                &[&LatexCodeResultBoxed(LatexCodeResult::Skip)],
            );
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![Signal::builder("latex-editor-result")
                    .param_types([LatexCodeResultBoxed::static_type()])
                    .build()]
            })
        }
    }

    impl WidgetImpl for RnLatexEditor {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);
        }
    }

    impl AdwWindowImpl for RnLatexEditor {}

    impl WindowImpl for RnLatexEditor {}

    impl ApplicationWindowImpl for RnLatexEditor {}

    impl AdwApplicationWindowImpl for RnLatexEditor {}
}

glib::wrapper! {
    pub(crate) struct RnLatexEditor(ObjectSubclass<imp::RnLatexEditor>)
        @extends adw::ApplicationWindow, adw::Window, gtk4::Window, gtk4::Widget,
    @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl RnLatexEditor {
    pub(crate) fn new() -> Self {
        glib::Object::new()
    }
}
