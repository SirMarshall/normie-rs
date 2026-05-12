use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::Path,
};

pub struct Database {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub unique_values: Vec<Vec<(String, String)>>,
    pub row_registry: Vec<HashMap<String, Vec<usize>>>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct NormalizationState {
    /// Column Name -> (Original Value -> Normalized Value)
    pub mappings: HashMap<String, HashMap<String, String>>,
}

impl NormalizationState {
    pub fn load(csv_path: &Path) -> Self {
        let mapping_path = csv_path.with_extension("normie.json");
        fs::read_to_string(mapping_path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, csv_path: &Path) -> Result<()> {
        let mapping_path = csv_path.with_extension("normie.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(mapping_path, content)?;
        Ok(())
    }
}
