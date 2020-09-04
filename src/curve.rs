use hporecord::{Record, ValueDef};
use serde::Serialize;
use std::collections::BTreeMap;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct CurveOpt {
    #[structopt(long, default_value = "0")]
    pub span_index: usize,

    #[structopt(long, default_value = "0")]
    pub objective_index: usize,

    // TODO:
    #[structopt(long)]
    pub problem_name: Option<String>,

    #[structopt(long)]
    pub optimizer_name: Option<String>,

    #[structopt(long)]
    pub key_script: Option<String>,
}

impl CurveOpt {
    pub fn calculate_optimization_curve(
        &self,
        records: &[Record],
    ) -> anyhow::Result<BTreeMap<String, Study>> {
        let studies = self.build_studies(records)?;
        Ok(studies)
    }

    fn build_studies(&self, records: &[Record]) -> anyhow::Result<BTreeMap<String, Study>> {
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

        let mut studies = BTreeMap::new();
        for record in records {
            match record {
                Record::Study(study) => {
                    let study_id = id_mapping.get(&study.id).unwrap_or(&study.id);
                    studies
                        .entry(study_id.clone())
                        .or_insert_with(|| Study {
                            span_name: study.spans[self.span_index].name.clone(),
                            objective: study.values[self.objective_index].clone(),
                            best_values: Default::default(),
                        })
                        .best_values
                        .insert(study.id.clone(), vec![None]);
                }
                Record::Eval(eval) => {
                    if !eval.state.is_complete() {
                        continue;
                    }

                    let study_id = id_mapping.get(&eval.study).unwrap_or(&eval.study);
                    let study = studies.get_mut(study_id).expect("TODO");
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

        Ok(studies)
    }
}

#[derive(Debug, Serialize)]
pub struct Study {
    span_name: String,
    objective: ValueDef,
    best_values: BTreeMap<String, Vec<Option<f64>>>,
}
