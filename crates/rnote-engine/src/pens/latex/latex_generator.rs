use std::{io::Write, path::Path, process::Command};

#[derive(Debug, Clone)]
pub struct Environment {
    pre_preamble: &'static str,
    pre_code: &'static str,
    post_code: &'static str,
}

pub const INLINE: Environment = Environment {
    pre_preamble: "\\documentclass[varwidth=true, border=10pt]{standalone}",
    pre_code: "\\begin{document}\n",
    post_code: "\\end{document}\n",
};

#[derive(Debug, Clone)]
pub struct LatexContext {
    pub preamble: String,
    pub environment: Environment,
}

impl LatexContext {
    fn preprocess_code(&self, latex_code: &String) -> String {
        let mut complete_code = String::from(self.environment.pre_preamble);
        complete_code.push_str(&self.preamble);
        complete_code.push_str(self.environment.pre_code);
        complete_code.push_str(&latex_code);
        complete_code.push_str(self.environment.post_code);

        complete_code
    }
}

pub fn create_svg_from_latex(latex_code: &String, context: &LatexContext) -> String {
    // Create temporary directory
    let mut tmpdir = tempfile::tempdir().unwrap();

    // Preprocess latex code
    let preprocessed_code = context.preprocess_code(latex_code);

    // Open file
    let tex_path = tmpdir.path().join(Path::new("ezlatex.tex"));
    let dvi_path = tmpdir.path().join(Path::new("ezlatex.dvi"));
    let svg_path = tmpdir.path().join(Path::new("ezlatex.svg"));
    let mut file = std::fs::File::create(&tex_path).unwrap();
    // Write to file
    file.write_all(preprocessed_code.as_bytes()).unwrap();
    // Close file
    drop(file);

    // Compile and convert to DVI
    Command::new("latex")
        .current_dir(tmpdir.path())
        .arg(&tex_path)
        .status()
        .unwrap();

    // ezlatex.dvi will have been created
    Command::new("dvisvgm")
        .current_dir(tmpdir.path())
        .arg("-n")
        .arg(&dvi_path)
        .status()
        .unwrap();

    // ezlatex.svg will have been created

    std::fs::read_to_string(&svg_path).unwrap()
}
