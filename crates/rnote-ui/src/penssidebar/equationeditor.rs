use adw::ffi::AdwWindow;
use adw::prelude::*;
use adw::Window;
use gtk4::ffi::GtkWidget;
use gtk4::ffi::GtkWindow;
use gtk4::{glib, glib::clone, subclass::prelude::*, CompositeTemplate, ToggleButton};

#[derive(Debug, Clone)]
pub enum EquationEditorResult {
    Skip,
    Compiled(String, String),
}

#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "EquationCodeCompilationResult")]
pub struct EquationCodeCompilationResult(Result<String, String>);

impl From<Result<String, String>> for EquationCodeCompilationResult {
    fn from(value: Result<String, String>) -> Self {
        Self(value)
    }
}

impl From<EquationCodeCompilationResult> for Result<String, String> {
    fn from(value: EquationCodeCompilationResult) -> Self {
        value.0
    }
}

impl EquationCodeCompilationResult {
    pub(crate) fn inner(self) -> Result<String, String> {
        self.0
    }
}

#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "EquationEditorResultBoxed")]
pub struct EquationEditorResultBoxed(EquationEditorResult);

impl From<EquationEditorResult> for EquationEditorResultBoxed {
    fn from(value: EquationEditorResult) -> Self {
        Self(value)
    }
}

impl From<EquationEditorResultBoxed> for EquationEditorResult {
    fn from(value: EquationEditorResultBoxed) -> Self {
        value.0
    }
}

impl EquationEditorResultBoxed {
    pub(crate) fn inner(self) -> EquationEditorResult {
        self.0
    }
}

mod imp {

    use std::sync::OnceLock;

    use adw::{
        subclass::{application_window::AdwApplicationWindowImpl, window::AdwWindowImpl},
        OverlaySplitView, Toast, ToastOverlay,
    };
    use cairo::glib::{gobject_ffi::GObject, subclass::Signal};
    use gtk4::{Button, TextBuffer, TextView};

    use super::*;
    #[derive(Default, Debug, CompositeTemplate)]
    #[template(resource = "/com/github/flxzt/rnote/ui/equationeditor.ui")]
    pub(crate) struct RnEquationEditor {
        #[template_child]
        pub(crate) equation_code: TemplateChild<TextBuffer>,
        #[template_child]
        pub(crate) error_message: TemplateChild<TextBuffer>,
        #[template_child]
        pub(crate) show_errors_button: TemplateChild<Button>,
        #[template_child]
        pub(crate) error_code_split: TemplateChild<OverlaySplitView>,
        #[template_child]
        pub(crate) compilation_failed_toast_overlay: TemplateChild<ToastOverlay>,
        #[template_child]
        pub(crate) compilation_failed_toast: TemplateChild<Toast>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RnEquationEditor {
        const NAME: &'static str = "RnEquationEditor";
        type Type = super::RnEquationEditor;
        type ParentType = gtk4::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RnEquationEditor {
        fn constructed(&self) {
            self.parent_constructed();

            let error_code_split = self.error_code_split.get();
            self.show_errors_button.connect_clicked(
                clone!(@weak error_code_split => move |_button| {
                    error_code_split.set_show_sidebar(!error_code_split.shows_sidebar());
                }),
            );

            self.equation_code
                .connect_changed(clone!(@weak self as equation_editor => move |_| {
                    // equation_editor.request_compilation();
                    equation_editor.obj().request_compilation();
                }));
        }

        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();

            SIGNALS.get_or_init(|| {
                vec![Signal::builder("equation-editor-request-compilation")
                    .param_types([String::static_type()])
                    .return_type_from(EquationCodeCompilationResult::static_type())
                    .build()]
            })
        }
    }

    impl WidgetImpl for RnEquationEditor {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);
        }
    }
}

glib::wrapper! {
    pub(crate) struct RnEquationEditor(ObjectSubclass<imp::RnEquationEditor>)
        @extends gtk4::Widget,
    @implements gtk4::Buildable;
}

impl RnEquationEditor {
    pub(crate) fn new(initial_equation_code: &String) -> Self {
        glib::Object::new()
    }

    pub fn set_equation_code(&self, equation_code: &String) {
        self.imp().equation_code.set_text(equation_code.as_str());
    }

    pub fn set_equation_error(&self, equation_error: Option<String>) {
        if let Some(some_equation_error) = equation_error {
            self.imp()
                .compilation_failed_toast_overlay
                .add_toast(self.imp().compilation_failed_toast.get());
            self.imp().error_message.set_text(&some_equation_error);
        } else {
            self.imp().error_message.set_text("");
        }
    }

    fn request_compilation(&self) {
        let equation_editor = self.imp();
        let equation_code = &equation_editor.equation_code;
        let text = String::from_utf8(
            equation_code
                .text(&equation_code.start_iter(), &equation_code.end_iter(), true)
                .as_bytes()
                .to_vec(),
        )
        .unwrap();

        self.emit_by_name::<EquationCodeCompilationResult>(
            "equation-editor-request-compilation",
            &[&text],
        );
    }
}
