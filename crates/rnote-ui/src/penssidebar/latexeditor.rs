use adw::ffi::AdwWindow;
use adw::prelude::*;
use adw::Window;
use gtk4::ffi::GtkWidget;
use gtk4::ffi::GtkWindow;
use gtk4::{glib, glib::clone, subclass::prelude::*, CompositeTemplate, ToggleButton};

#[derive(Debug, Clone)]
pub enum LatexEditorResult {
    Skip,
    Compiled(String, String),
}

#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "LatexCodeCompilationResult")]
pub struct LatexCodeCompilationResult(Result<String, String>);

impl From<Result<String, String>> for LatexCodeCompilationResult {
    fn from(value: Result<String, String>) -> Self {
        Self(value)
    }
}

impl From<LatexCodeCompilationResult> for Result<String, String> {
    fn from(value: LatexCodeCompilationResult) -> Self {
        value.0
    }
}

impl LatexCodeCompilationResult {
    pub(crate) fn inner(self) -> Result<String, String> {
        self.0
    }
}

#[derive(Clone, Debug, glib::Boxed)]
#[boxed_type(name = "LatexEditorResultBoxed")]
pub struct LatexEditorResultBoxed(LatexEditorResult);

impl From<LatexEditorResult> for LatexEditorResultBoxed {
    fn from(value: LatexEditorResult) -> Self {
        Self(value)
    }
}

impl From<LatexEditorResultBoxed> for LatexEditorResult {
    fn from(value: LatexEditorResultBoxed) -> Self {
        value.0
    }
}

impl LatexEditorResultBoxed {
    pub(crate) fn inner(self) -> LatexEditorResult {
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
    #[template(resource = "/com/github/flxzt/rnote/ui/latexeditor.ui")]
    pub(crate) struct RnLatexEditor {
        #[template_child]
        pub(crate) latex_code: TemplateChild<TextBuffer>,
        #[template_child]
        pub(crate) error_message: TemplateChild<TextBuffer>,
        #[template_child]
        pub(crate) compile_button: TemplateChild<Button>,
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

    impl WindowImpl for RnLatexEditor {
        fn close_request(&self) -> glib::Propagation {
            self.obj().emit_by_name::<()>(
                "latex-editor-compiled",
                &[&LatexEditorResultBoxed(LatexEditorResult::Skip)],
            );
            glib::Propagation::Proceed
        }
    }

    impl ObjectImpl for RnLatexEditor {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let latex_code = self.latex_code.clone();

            let error_code_split = self.error_code_split.get();
            self.show_errors_button.connect_clicked(
                clone!(@weak error_code_split => move |_button| {
                    error_code_split.set_show_sidebar(!error_code_split.shows_sidebar());
                }),
            );

            self.compile_button.connect_clicked(clone!(@weak self as latex_editor, @strong obj, @strong latex_code => move |_button| {
				let text = String::from_utf8(latex_code.text(&latex_code.start_iter(), &latex_code.end_iter(), true).as_bytes().to_vec()).unwrap();

				let result = obj.compile_equation(&text);

				match result {
					Ok(svg_code) => {
						obj.emit_by_name::<()>("latex-editor-compiled", &[&LatexEditorResultBoxed(LatexEditorResult::Compiled(svg_code, text))]);
						obj.close();
					}
					Err(error_message) => {
						latex_editor.error_message.get().set_text(error_message.as_str());
						latex_editor.compilation_failed_toast_overlay.add_toast(latex_editor.compilation_failed_toast.get());
					}
				}

				// obj.emit_by_name::<()>("latex-editor-result", &[&LatexEditorResultBoxed(LatexEditorResult::Compile(String::from(text)))]);
				// obj.close();
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
                vec![
                    Signal::builder("latex-editor-compiled")
                        .param_types([LatexEditorResultBoxed::static_type()])
                        .build(),
                    Signal::builder("latex-editor-request-compilation")
                        .param_types([String::static_type()])
                        .return_type_from(LatexCodeCompilationResult::static_type())
                        .build(),
                ]
            })
        }
    }

    impl WidgetImpl for RnLatexEditor {
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.parent_size_allocate(width, height, baseline);
        }
    }

    impl AdwWindowImpl for RnLatexEditor {}

    impl ApplicationWindowImpl for RnLatexEditor {}

    impl AdwApplicationWindowImpl for RnLatexEditor {}
}

glib::wrapper! {
    pub(crate) struct RnLatexEditor(ObjectSubclass<imp::RnLatexEditor>)
        @extends adw::ApplicationWindow, adw::Window, gtk4::Window, gtk4::Widget,
    @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl RnLatexEditor {
    pub(crate) fn new(initial_latex_code: &String) -> Self {
        let latex_editor: RnLatexEditor = glib::Object::new();
        latex_editor
            .imp()
            .latex_code
            .set_text(initial_latex_code.as_str());
        latex_editor
    }

    fn compile_equation(&self, code: &String) -> Result<String, String> {
        let emitted_result: LatexCodeCompilationResult =
            self.emit_by_name("latex-editor-request-compilation", &[code]);
        emitted_result.inner()
    }
}
