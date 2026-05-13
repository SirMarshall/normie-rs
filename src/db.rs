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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    Contains,
    NotContains,
    InList,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Condition {
    pub col_idx: usize,
    pub col_name: String,
    pub operator: ConditionOperator,
    pub value: String,
    #[serde(default)]
    pub values: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TransformationStep {
    SimpleMap {
        column: String,
        original: String,
        normalized: String,
    },
    ConditionalMap {
        column: String,
        original: String,
        normalized: String,
        conditions: Vec<Condition>,
    },
}

#[derive(Serialize, Deserialize, Default)]
pub struct NormalizationState {
    pub pipeline: Vec<TransformationStep>,
}

impl NormalizationState {
    pub fn load(csv_path: &Path) -> Self {
        let mapping_path = csv_path.with_extension("normie.json");
        let content = match fs::read_to_string(&mapping_path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        // Try parsing the new format first
        if let Ok(state) = serde_json::from_str::<NormalizationState>(&content) {
            return state;
        }

        // Fallback to legacy HashMap format: Column Name -> (Original Value -> Normalized Value)
        if let Ok(legacy) = serde_json::from_str::<HashMap<String, HashMap<String, String>>>(&content) {
            let mut pipeline = Vec::new();
            for (col_name, mappings) in legacy {
                for (original, normalized) in mappings {
                    pipeline.push(TransformationStep::SimpleMap {
                        column: col_name.clone(),
                        original,
                        normalized,
                    });
                }
            }
            return NormalizationState { pipeline };
        }

        Self::default()
    }

    pub fn save(&self, csv_path: &Path) -> Result<()> {
        let mapping_path = csv_path.with_extension("normie.json");
        let content = serde_json::to_string_pretty(self)?;
        fs::write(mapping_path, content)?;
        Ok(())
    }
}
