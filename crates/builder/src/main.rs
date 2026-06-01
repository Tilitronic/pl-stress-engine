//! Rust builder — compiles exceptions.json → exceptions.bin
//!
//! The heavy lifting (Wiktionary parsing, PoliMorf propagation) is done in
//! Python.  This crate only converts the JSON produced by `python -m
//! pl_stress.builder` into a compact bincode blob that gets embedded into the
//! WASM and PyO3 runtime crates at compile time.
//!
//! Run after the Python pipeline:
//!   cargo run -p builder --release
//!
//! Export the public lookup contract schema:
//!   cargo run -p builder -- export-schemas

mod export;

use ahash::AHashMap;
use anyhow::{Context, Result};
use pl_stress_core::{DictEntry, WordLookupResult};
use serde::Deserialize;
use std::path::PathBuf;

/// Shape of one entry in exceptions.json as written by `pl_stress.builder`.
#[derive(Deserialize)]
struct JsonEntry {
    stress_idx: u8,
    ipa: Option<String>,
}

fn export_schemas(root_dir: &std::path::Path) -> Result<()> {
    let schema_dir = root_dir.join("crates/wasm/generated");
    let lookup_schema_path = schema_dir.join("word-lookup-result.schema.json");

    std::fs::create_dir_all(&schema_dir)?;
    export::write_json_schema::<WordLookupResult>(&lookup_schema_path)?;

    eprintln!("Exported {:?}.", lookup_schema_path);
    Ok(())
}

fn build_exceptions(root_dir: &std::path::Path) -> Result<()> {
    let data_dir = root_dir.join("data");

    let json_path = data_dir.join("processed/exceptions.json");
    let bin_path  = data_dir.join("processed/exceptions.bin");

    std::fs::create_dir_all(data_dir.join("processed"))?;

    eprintln!("[1/2] Reading {:?} …", json_path);
    let json_bytes = std::fs::read(&json_path)
        .with_context(|| format!(
            "exceptions.json not found at {:?}.\n\
             Run the Python pipeline first:\n\
             python -m pl_stress.builder --dump data/raw/plwiktionary-latest-pages-articles.xml.bz2",
            json_path
        ))?;

    let raw: std::collections::HashMap<String, JsonEntry> =
        serde_json::from_slice(&json_bytes).context("Failed to parse exceptions.json")?;

    let map: AHashMap<String, DictEntry> = raw
        .into_iter()
        .map(|(word, e)| (word, DictEntry { stress_idx: e.stress_idx, ipa: e.ipa }))
        .collect();

    eprintln!("[2/2] Writing {:?} ({} entries) …", bin_path, map.len());
    export::write_bincode(&map, &bin_path)?;

    eprintln!("Done.  Rebuild the wasm/python crates to embed the new dict.");
    Ok(())
}

fn main() -> Result<()> {
    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let command = std::env::args().nth(1);

    match command.as_deref() {
        Some("export-schemas") => export_schemas(&root_dir),
        Some(other) => anyhow::bail!(
            "Unknown command: {other}. Supported commands: export-schemas"
        ),
        None => build_exceptions(&root_dir),
    }
}

