//! Project serialization with versioning and migration.
//!
//! Uses JSON with a schema version field for forward-compatible persistence.

use proedit_core::{ProEditError, Result};
use serde::{Deserialize, Serialize};

use crate::project::Project;

/// Current schema version.
pub const CURRENT_VERSION: u32 = 1;

/// Versioned project file wrapper.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectFile {
    /// Schema version for migration.
    pub version: u32,
    /// The project data.
    pub project: Project,
    /// Application version that wrote this file.
    pub app_version: String,
}

impl ProjectFile {
    /// Create a new project file from a project.
    pub fn new(project: Project) -> Self {
        Self {
            version: CURRENT_VERSION,
            project,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Serialize to JSON bytes.
    pub fn to_json(&self) -> Result<Vec<u8>> {
        serde_json::to_vec_pretty(self)
            .map_err(|e| ProEditError::Serialization(format!("Failed to serialize project: {}", e)))
    }

    /// Deserialize from JSON bytes, applying migrations if needed.
    pub fn from_json(data: &[u8]) -> Result<Self> {
        // First, try to read just the version
        let raw: serde_json::Value = serde_json::from_slice(data)
            .map_err(|e| ProEditError::Serialization(format!("Invalid JSON: {}", e)))?;

        let version = raw.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u32;

        if version > CURRENT_VERSION {
            return Err(ProEditError::Serialization(format!(
                "Project file version {} is newer than supported version {}",
                version, CURRENT_VERSION
            )));
        }

        // Apply migrations
        let migrated = migrate(raw, version)?;

        serde_json::from_value(migrated)
            .map_err(|e| ProEditError::Serialization(format!("Failed to parse project: {}", e)))
    }

    /// Save project to a file path.
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<()> {
        let data = self.to_json()?;
        std::fs::write(path, data)?;
        Ok(())
    }

    /// Load project from a file path.
    pub fn load_from_file(path: &std::path::Path) -> Result<Self> {
        let data = std::fs::read(path)?;
        Self::from_json(&data)
    }
}

/// Apply sequential migrations from `from_version` to CURRENT_VERSION.
fn migrate(mut data: serde_json::Value, from_version: u32) -> Result<serde_json::Value> {
    let mut version = from_version;

    while version < CURRENT_VERSION {
        match version {
            0 => {
                // v0 → v1: Add version field, wrap project if needed
                if data.get("version").is_none() {
                    // The entire value IS the project (old format)
                    data = serde_json::json!({
                        "version": 1,
                        "project": data,
                        "app_version": "0.1.0",
                    });
                }
                version = 1;
            }
            _ => {
                return Err(ProEditError::Serialization(format!(
                    "No migration path from version {}",
                    version
                )));
            }
        }
    }

    Ok(data)
}

/// Recent projects list.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecentProjects {
    /// Most recent first.
    pub entries: Vec<RecentEntry>,
    /// Maximum entries to keep.
    pub max_entries: usize,
}

/// A recent project entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentEntry {
    /// File path.
    pub path: String,
    /// Project name.
    pub name: String,
    /// Last opened timestamp (unix seconds).
    pub last_opened: u64,
}

impl RecentProjects {
    /// Create with default max of 10 entries.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: 10,
        }
    }

    /// Record that a project was opened.
    pub fn record(&mut self, path: String, name: String, timestamp: u64) {
        // Remove existing entry for this path
        self.entries.retain(|e| e.path != path);

        // Add at front
        self.entries.insert(
            0,
            RecentEntry {
                path,
                name,
                last_opened: timestamp,
            },
        );

        // Trim to max
        self.entries.truncate(self.max_entries);
    }

    /// Remove an entry by path.
    pub fn remove(&mut self, path: &str) {
        self.entries.retain(|e| e.path != path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::Project;

    #[test]
    fn test_project_roundtrip() {
        let project = Project::new("Test Project");
        let file = ProjectFile::new(project);

        let json = file.to_json().unwrap();
        let loaded = ProjectFile::from_json(&json).unwrap();

        assert_eq!(loaded.version, CURRENT_VERSION);
        assert_eq!(loaded.project.name, "Test Project");
    }

    #[test]
    fn test_migration_v0() {
        // Simulate a v0 project file (no version wrapper)
        let project = Project::new("Old Project");
        let raw_json = serde_json::to_vec(&project).unwrap();

        let loaded = ProjectFile::from_json(&raw_json).unwrap();
        assert_eq!(loaded.version, CURRENT_VERSION);
        assert_eq!(loaded.project.name, "Old Project");
    }

    #[test]
    fn test_future_version_rejected() {
        let json = serde_json::json!({
            "version": 999,
            "project": {},
            "app_version": "99.0.0",
        });
        let data = serde_json::to_vec(&json).unwrap();
        let result = ProjectFile::from_json(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_recent_projects() {
        let mut recent = RecentProjects::new();
        recent.record("a.proj".into(), "A".into(), 1000);
        recent.record("b.proj".into(), "B".into(), 2000);
        recent.record("a.proj".into(), "A".into(), 3000);

        assert_eq!(recent.entries.len(), 2);
        assert_eq!(recent.entries[0].path, "a.proj"); // most recent
        assert_eq!(recent.entries[1].path, "b.proj");
    }
}
