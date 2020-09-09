use crate::utils::MeanAndStddev;
use hporecord::{Record, ValueDef};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use structopt::StructOpt;

pub type LuaScript = String;
pub type Studies = BTreeMap<String, BTreeMap<String, Study>>;

#[derive(Debug, StructOpt)]
pub struct CurveOpt {
    #[structopt(long, default_value = "0")]
    pub span_index: usize,

    #[structopt(long, default_value = "0")]
    pub objective_index: usize,

    #[structopt(long)]
    pub problem_name: LuaScript,

    #[structopt(long)]
    pub optimizer_name: LuaScript,
}

impl CurveOpt {
    pub fn calculate_optimization_curve(&self, records: &[Record]) -> anyhow::Result<Studies> {
        let studies = self.build_studies(records)?;
        Ok(studies)
    }

    fn build_studies(&self, records: &[Record]) -> anyhow::Result<Studies> {
        let mut problem_mapping = BTreeMap::new();
        let mut optimizer_mapping = BTreeMap::new();
        for record in records {
            if let Record::Study(study) = record {
                let lua = rlua::Lua::new();
                let new_id: String = lua.context(|lua_ctx| {
                    let globals = lua_ctx.globals();

                    // TODO
                    globals.set("attrs", study.attrs.clone())?;

                    lua_ctx.load(&self.problem_name).eval()
                })?;
                problem_mapping.insert(&study.id, new_id);
            }

            if let Record::Study(study) = record {
                let lua = rlua::Lua::new();
                let new_id: String = lua.context(|lua_ctx| {
                    let globals = lua_ctx.globals();

                    // TODO
                    globals.set("attrs", study.attrs.clone())?;

                    lua_ctx.load(&self.optimizer_name).eval()
                })?;
                optimizer_mapping.insert(&study.id, new_id);
            }
        }

        let mut studies: Studies = BTreeMap::new();
        let mut skipped_studies = BTreeSet::new();
        for record in records {
            match record {
                Record::Study(study) => {
                    let problem_id = problem_mapping.get(&study.id).expect("unreachable");
                    let optimizer_id = optimizer_mapping.get(&study.id).expect("unreachable");
                    studies
                        .entry(problem_id.clone())
                        .or_default()
                        .entry(optimizer_id.clone())
                        .or_insert_with(|| Study {
                            span_name: study.spans[self.span_index].name.clone(),
                            objective: study.values[self.objective_index].clone(),
                            best_values_avg: MeanAndStddev {
                                mean: Vec::new(),
                                stddev: Vec::new(),
                            },
                            best_values: Default::default(),
                            samples: 0,
                        })
                        .best_values
                        .insert(study.id.clone(), vec![None]);
                }
                Record::Eval(eval) => {
                    if !eval.state.is_complete() {
                        continue;
                    }
                    if !problem_mapping.contains_key(&eval.study) {
                        if !skipped_studies.contains(&eval.study) {
                            eprintln!("[WARN] Unknown study: {:?}", eval.study);
                            skipped_studies.insert(&eval.study);
                        }
                        continue;
                    }

                    let problem_id = problem_mapping.get(&eval.study).expect("TODO");
                    let optimizer_id = optimizer_mapping.get(&eval.study).expect("TODO");

                    let study = studies
                        .get_mut(problem_id)
                        .expect("unreachable")
                        .get_mut(optimizer_id)
                        .expect("unreachable");
                    let best_values = study.best_values.get_mut(&eval.study).expect("TODO");

                    let span = eval.spans[self.span_index];
                    let value = eval.values[self.objective_index];
                    let current = span.end.round() as usize;
                    while best_values.len() <= current {
                        // TODO: optimize
                        best_values.push(best_values[best_values.len() - 1]);
                    }

                    for v in &mut best_values[current..] {
                        if let Some(v) = v {
                            *v = study.objective.direction.better(*v, value);
                            if *v != value {
                                break;
                            }
                        } else {
                            *v = Some(value);
                        }
                    }
                }
            }
        }

        for studies in studies.values_mut() {
            for study in studies.values_mut() {
                study.samples = study.best_values.len();

                let size = study
                    .best_values
                    .values()
                    .map(|vs| vs.len())
                    .min()
                    .expect("unreachable");
                for i in 0..size {
                    if study.best_values.values().any(|vs| vs[i].is_none()) {
                        study.best_values_avg.mean.push(None);
                        study.best_values_avg.stddev.push(None);
                        continue;
                    }

                    let total = study
                        .best_values
                        .values()
                        .map(|vs| vs[i].expect("unreachable"))
                        .sum::<f64>();
                    let mean = total / study.best_values.len() as f64;
                    let stddev = (study
                        .best_values
                        .values()
                        .map(|vs| (vs[i].expect("unreachable") - mean).powi(2))
                        .sum::<f64>()
                        / study.best_values.len() as f64)
                        .sqrt();
                    study.best_values_avg.mean.push(Some(mean));
                    study.best_values_avg.stddev.push(Some(stddev));
                }
            }
        }

        Ok(studies)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Study {
    pub span_name: String,
    pub objective: ValueDef,
    pub best_values_avg: MeanAndStddev<Vec<Option<f64>>>,
    pub samples: usize,

    // TODO: delete
    #[serde(skip_serializing, default)]
    best_values: BTreeMap<String, Vec<Option<f64>>>,
}
