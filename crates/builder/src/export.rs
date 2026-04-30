use ahash::AHashMap;
use anyhow::Result;
use pl_stress_core::DictEntry;
use std::path::Path;

pub fn write_bincode(map: &AHashMap<String, DictEntry>, path: &Path) -> Result<()> {
    // bincode needs a std HashMap with borrowed keys
    let std_map: std::collections::HashMap<&str, &DictEntry> =
        map.iter().map(|(k, v)| (k.as_str(), v)).collect();
    let bytes = bincode::serialize(&std_map)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

