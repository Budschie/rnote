use serde::{Deserialize, Serialize};

use crate::pens::equation::equation_provider::{EquationProvider, EquationProviderTrait};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename = "equation_config")]
pub struct EquationConfig {
    #[serde(rename = "equation_provider")]
    pub equation_provider: EquationProvider,
    #[serde(rename = "font_size")]
    pub font_size: u32,
    #[serde(rename = "page_width")]
    pub page_width: f64,
}

impl Default for EquationConfig {
    fn default() -> Self {
        EquationConfig {
            equation_provider: EquationProvider::default(),
            font_size: 12,
            page_width: 64.0,
        }
    }
}

impl EquationConfig {
    pub fn generate_svg(&self, code: &String) -> Result<String, String> {
        self.equation_provider
            .generate_svg(code, self.font_size, self.page_width)
    }
}
