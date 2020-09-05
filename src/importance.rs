use anyhow::ensure;
use hporecord::{EvalState, ParamDef, Record, StudyId, StudyRecord};
//use indicatif::ProgressBar;
use crate::utils::MeanAndStddev;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use structopt::StructOpt;

pub type Importances = BTreeMap<StudyId, Vec<Importance>>;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct ImportanceOpt {
    #[structopt(long, default_value = "0")]
    pub objective_value_index: usize,

    #[structopt(long, default_value = "1")]
    pub max_dimension: NonZeroUsize,

    #[structopt(long)]
    pub convert_log_param: bool,

    #[structopt(long)]
    pub key_script: Option<String>,
}

impl ImportanceOpt {
    pub fn calculate_importances(
        &self,
        records: &[Record],
    ) -> anyhow::Result<BTreeMap<StudyId, Vec<Importance>>> {
        let studies = self.build_studies(records)?;

        let mut result = BTreeMap::new();
        for (study_id, study) in studies {
            let mut importances = Vec::new();

            let mut fanova = fanova::FanovaOptions::new().parallel().fit(
                study.params.iter().map(|p| p.as_slice()).collect(),
                &study.values,
            )?;

            for dim in 1..=self.max_dimension.get() {
                for indices in (0..study.param_defs.len()).combinations(dim) {
                    let importance = fanova.quantify_importance(&indices);
                    importances.push(Importance {
                        params: indices
                            .into_iter()
                            .map(|i| study.param_defs[i].name.clone())
                            .collect(),
                        importance: MeanAndStddev {
                            mean: importance.mean,
                            stddev: importance.stddev,
                        },
                    });
                }
            }

            importances.sort_by_key(|i| OrderedFloat(i.importance.mean));
            importances.reverse();
            result.insert(study_id, importances);
        }
        Ok(result)
    }

    fn build_studies(&self, records: &[Record]) -> anyhow::Result<BTreeMap<StudyId, Study>> {
        let mut id_mapping = BTreeMap::new();
        if let Some(script) = &self.key_script {
            for record in records {
                if let Record::Study(study) = record {
                    let lua = rlua::Lua::new();
                    let new_id: String = lua.context(|lua_ctx| {
                        let globals = lua_ctx.globals();

                        // TODO
                        globals.set("attrs", study.attrs.clone())?;

                        lua_ctx.load(&script).eval()
                    })?;
                    id_mapping.insert(&study.id, new_id);
                }
            }
        }

        // TODO: Handle categorical and log scale
        let mut studies = BTreeMap::new();
        for record in records {
            match record {
                Record::Study(study) => {
                    let study_id = id_mapping.get(&study.id).unwrap_or(&study.id);
                    if !studies.contains_key(study_id) {
                        studies.insert(study_id.clone(), Study::new(study));
                    } else {
                        // TODO: Check whether the parameter definitions of the both studies are the same.
                    }
                }
                Record::Eval(eval) => {
                    let study_id = id_mapping.get(&eval.study).unwrap_or(&eval.study);

                    ensure!(
                        studies.contains_key(study_id),
                        "unknown study {:?}",
                        eval.study
                    );
                    if eval.state != EvalState::Complete {
                        continue;
                    }

                    let study = studies.get_mut(study_id).expect("unreachable");
                    for ((&p, ps), def) in eval
                        .params
                        .iter()
                        .zip(study.params.iter_mut())
                        .zip(study.param_defs.iter())
                    {
                        use hporecord::{ParamRange, Scale};

                        if self.convert_log_param
                            && matches!(def.range, ParamRange::Numerical{scale: Scale::Log,..})
                        {
                            ps.push(p.ln());
                        } else {
                            ps.push(p);
                        }
                    }
                    ensure!(
                        self.objective_value_index < eval.values.len(),
                        "the objective value index {} is out of range (must be less than {})",
                        self.objective_value_index,
                        eval.values.len()
                    );
                    study.values.push(eval.values[self.objective_value_index]);
                }
            }
        }
        Ok(studies)
    }
}

#[derive(Debug)]
pub struct Study {
    param_defs: Vec<ParamDef>,
    params: Vec<Vec<f64>>,
    values: Vec<f64>,
}

impl Study {
    fn new(record: &StudyRecord) -> Self {
        Self {
            param_defs: record.params.clone(),
            params: vec![Vec::new(); record.params.len()],
            values: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Importance {
    pub params: Vec<String>,
    pub importance: MeanAndStddev,
}
