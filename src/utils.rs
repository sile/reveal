use hporecord::Record;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MeanAndStddev<T = f64> {
    pub mean: T,
    pub stddev: T,
}

pub fn build_id_mapping(
    script: &str,
    records: &[Record],
) -> anyhow::Result<BTreeMap<String, String>> {
    let mut id_mapping = BTreeMap::new();
    for record in records {
        if let Record::Study(study) = record {
            let lua = rlua::Lua::new();
            let new_id: String = lua.context(|lua_ctx| {
                let globals = lua_ctx.globals();

                // TODO
                globals.set("attrs", study.attrs.clone())?;

                lua_ctx.load(&script).eval()
            })?;
            id_mapping.insert(study.id.clone(), new_id);
        }
    }
    Ok(id_mapping)
}
