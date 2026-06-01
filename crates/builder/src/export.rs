use ahash::AHashMap;
use anyhow::Result;
use pl_stress_core::DictEntry;
use schemars::{schema_for, JsonSchema};
use std::path::Path;

pub fn write_bincode(map: &AHashMap<String, DictEntry>, path: &Path) -> Result<()> {
    // bincode needs a std HashMap with borrowed keys
    let std_map: std::collections::HashMap<&str, &DictEntry> =
        map.iter().map(|(k, v)| (k.as_str(), v)).collect();
    let bytes = bincode::serialize(&std_map)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

pub fn write_json_schema<T: JsonSchema>(path: &Path) -> Result<()> {
    let schema = schema_for!(T);
    let json = serde_json::to_string_pretty(&schema)?;
    std::fs::write(path, format!("{json}\n"))?;
    Ok(())
}

