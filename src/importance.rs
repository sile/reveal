use anyhow::ensure;
use hporecord::{EvalState, Record, StudyId, StudyRecord};
use itertools::Itertools;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct ImportanceOpt {
    #[structopt(long, default_value = "0")]
    pub objective_value_index: usize,

    #[structopt(long, default_value = "1")]
    pub max_dimension: NonZeroUsize,
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
                for indices in (0..study.param_names.len()).combinations(dim) {
                    let importance = fanova.quantify_importance(&indices);
                    importances.push(Importance {
                        params: indices
                            .into_iter()
                            .map(|i| study.param_names[i].clone())
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
        // TODO: Handle categorical and log scale
        let mut studies = BTreeMap::new();
        for record in records {
            match record {
                Record::Study(study) => {
                    if !studies.contains_key(&study.id) {
                        studies.insert(study.id.clone(), Study::new(study));
                    }
                }
                Record::Eval(eval) => {
                    ensure!(
                        studies.contains_key(&eval.study),
                        "unknown study {:?}",
                        eval.study
                    );
                    if eval.state != EvalState::Complete {
                        continue;
                    }

                    let study = studies.get_mut(&eval.study).expect("unreachable");
                    for (&p, ps) in eval.params.iter().zip(study.params.iter_mut()) {
                        ps.push(p);
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
    param_names: Vec<String>,
    params: Vec<Vec<f64>>,
    values: Vec<f64>,
}

impl Study {
    fn new(record: &StudyRecord) -> Self {
        Self {
            param_names: record.params.iter().map(|p| p.name.clone()).collect(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeanAndStddev {
    pub mean: f64,
    pub stddev: f64,
}
