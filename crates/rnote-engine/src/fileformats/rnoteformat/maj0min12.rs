// Imports
use super::{maj0min6::RnoteFileMaj0Min6, maj0min9::RnoteFileMaj0Min9};
use crate::{pens::equation::latex_equation_provider, Camera};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RnoteFileMaj0Min12 {
    /// A snapshot of the engine.
    #[serde(rename = "engine_snapshot")]
    pub engine_snapshot: ijson::IValue,
}

impl TryFrom<RnoteFileMaj0Min9> for RnoteFileMaj0Min12 {
    type Error = anyhow::Error;

    fn try_from(mut value: RnoteFileMaj0Min9) -> Result<Self, Self::Error> {
        let engine_snapsht = value
            .engine_snapshot
            .as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("engine snapshot is not a JSON object."))?;

        engine_snapsht
            .get_key_value_mut("stroke_components")
            .unwrap()
            .1
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .for_each(|elem| {
                let elem_obj_mut = elem.as_object_mut().unwrap();
                let value_key = elem_obj_mut.get_key_value_mut("value").unwrap().1;

                if value_key.is_object() {
                    let obj_value_key = value_key.as_object_mut().unwrap();

                    if obj_value_key.contains_key("equationimage") {
                        let equationimage = obj_value_key
                            .get_key_value_mut("equationimage")
                            .unwrap()
                            .1
                            .as_object_mut()
                            .unwrap();
                        let mut equation_provider_object = equationimage
                            .get_key_value_mut("equation_config")
                            .unwrap()
                            .1
                            .as_object_mut()
                            .unwrap()
                            .get_key_value_mut("equation_provider")
                            .unwrap()
                            .1
                            .as_object_mut()
                            .unwrap();

                        if equation_provider_object.contains_key("latex_equation_provider") {
                            let mut latex_equation_provider = equation_provider_object
                                .get_key_value_mut("latex_equation_provider")
                                .unwrap()
                                .1;

                            // If this is the case, replace it
                            if latex_equation_provider.is_array() {
                                *latex_equation_provider = ijson::IObject::new().into();
                            }
                        }
                    }
                }
            });

        Ok(Self {
            engine_snapshot: value.engine_snapshot,
        })
    }
}
