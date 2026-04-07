//! Preset browser for discovering and managing presets

use super::{builtin_presets, Preset, PresetError};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Metadata for a preset without loading full content
#[derive(Debug, Clone)]
pub struct PresetInfo {
    /// Preset name
    pub name: String,
    /// Author if specified
    pub author: Option<String>,
    /// Category if specified
    pub category: Option<String>,
    /// File path (None for built-in presets)
    pub path: Option<PathBuf>,
    /// Whether this is a built-in preset
    pub builtin: bool,
}

impl PresetInfo {
    /// Create from a preset
    pub fn from_preset(preset: &Preset, path: Option<PathBuf>) -> Self {
        let builtin = path.is_none();
        Self {
            name: preset.name.clone(),
            author: preset.author.clone(),
            category: preset.category.clone(),
            path,
            builtin,
        }
    }
}

/// Preset browser for discovering and loading presets
pub struct PresetBrowser {
    /// Directories to scan for presets
    search_paths: Vec<PathBuf>,
    /// Cached preset info
    cache: Vec<PresetInfo>,
    /// Index by category
    by_category: HashMap<String, Vec<usize>>,
    /// Index by author
    by_author: HashMap<String, Vec<usize>>,
}

impl PresetBrowser {
    /// Create a new preset browser
    pub fn new() -> Self {
        let mut browser = Self {
            search_paths: Vec::new(),
            cache: Vec::new(),
            by_category: HashMap::new(),
            by_author: HashMap::new(),
        };
        
        // Load built-in presets
        browser.load_builtins();
        
        browser
    }

    /// Create browser with custom search paths
    pub fn with_paths(paths: Vec<PathBuf>) -> Self {
        let mut browser = Self::new();
        for path in paths {
            browser.add_search_path(path);
        }
        browser
    }

    /// Add a directory to search for presets
    pub fn add_search_path(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref().to_path_buf();
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path.clone());
            self.scan_directory(&path);
        }
    }

    /// Refresh the preset cache by rescanning all directories
    pub fn refresh(&mut self) {
        self.cache.clear();
        self.by_category.clear();
        self.by_author.clear();
        
        self.load_builtins();
        
        let paths = self.search_paths.clone();
        for path in paths {
            self.scan_directory(&path);
        }
    }

    /// Get all presets
    pub fn all(&self) -> &[PresetInfo] {
        &self.cache
    }

    /// Get preset count
    pub fn count(&self) -> usize {
        self.cache.len()
    }

    /// Get presets by category
    pub fn by_category(&self, category: &str) -> Vec<&PresetInfo> {
        self.by_category
            .get(category)
            .map(|indices| indices.iter().map(|&i| &self.cache[i]).collect())
            .unwrap_or_default()
    }

    /// Get presets by author
    pub fn by_author(&self, author: &str) -> Vec<&PresetInfo> {
        self.by_author
            .get(author)
            .map(|indices| indices.iter().map(|&i| &self.cache[i]).collect())
            .unwrap_or_default()
    }

    /// Get all categories
    pub fn categories(&self) -> Vec<&str> {
        self.by_category.keys().map(|s| s.as_str()).collect()
    }

    /// Get all authors
    pub fn authors(&self) -> Vec<&str> {
        self.by_author.keys().map(|s| s.as_str()).collect()
    }

    /// Search presets by name (case-insensitive substring match)
    pub fn search(&self, query: &str) -> Vec<&PresetInfo> {
        let query_lower = query.to_lowercase();
        self.cache
            .iter()
            .filter(|p| p.name.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Get built-in presets only
    pub fn builtins(&self) -> Vec<&PresetInfo> {
        self.cache.iter().filter(|p| p.builtin).collect()
    }

    /// Get user presets only (non-builtin)
    pub fn user_presets(&self) -> Vec<&PresetInfo> {
        self.cache.iter().filter(|p| !p.builtin).collect()
    }

    /// Load a preset by info
    pub fn load(&self, info: &PresetInfo) -> Result<Preset, PresetError> {
        if info.builtin {
            // Find matching builtin preset
            builtin_presets()
                .into_iter()
                .find(|p| p.name == info.name)
                .ok_or_else(|| PresetError::Io(format!("Builtin preset '{}' not found", info.name)))
        } else if let Some(path) = &info.path {
            Preset::load(path)
        } else {
            Err(PresetError::Io("No path for preset".to_string()))
        }
    }

    /// Load a preset by name (searches all presets)
    pub fn load_by_name(&self, name: &str) -> Result<Preset, PresetError> {
        let info = self.cache
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| PresetError::Io(format!("Preset '{}' not found", name)))?;
        self.load(info)
    }

    /// Save a preset to a directory
    pub fn save(&mut self, preset: &Preset, directory: impl AsRef<Path>) -> Result<PathBuf, PresetError> {
        let dir = directory.as_ref();
        fs::create_dir_all(dir).map_err(|e| PresetError::Io(e.to_string()))?;
        
        // Create safe filename from preset name
        let filename = sanitize_filename(&preset.name) + ".json";
        let path = dir.join(&filename);
        
        preset.save(&path)?;
        
        // Add to cache if directory is in search paths
        if self.search_paths.iter().any(|p| p == dir) {
            let info = PresetInfo::from_preset(preset, Some(path.clone()));
            self.add_to_cache(info);
        }
        
        Ok(path)
    }

    /// Delete a user preset
    pub fn delete(&mut self, info: &PresetInfo) -> Result<(), PresetError> {
        if info.builtin {
            return Err(PresetError::Io("Cannot delete builtin preset".to_string()));
        }
        
        if let Some(path) = &info.path {
            fs::remove_file(path).map_err(|e| PresetError::Io(e.to_string()))?;
            
            // Remove from cache
            self.cache.retain(|p| p.path.as_ref() != Some(path));
            self.rebuild_indices();
        }
        
        Ok(())
    }

    /// Load built-in presets into cache
    fn load_builtins(&mut self) {
        for preset in builtin_presets() {
            let info = PresetInfo::from_preset(&preset, None);
            self.add_to_cache(info);
        }
    }

    /// Scan a directory for preset files
    fn scan_directory(&mut self, dir: &Path) {
        if !dir.exists() {
            return;
        }
        
        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(preset) = Preset::load(&path) {
                    let info = PresetInfo::from_preset(&preset, Some(path));
                    self.add_to_cache(info);
                }
            }
        }
    }

    /// Add a preset info to the cache and update indices
    fn add_to_cache(&mut self, info: PresetInfo) {
        let index = self.cache.len();
        
        if let Some(ref category) = info.category {
            self.by_category
                .entry(category.clone())
                .or_default()
                .push(index);
        }
        
        if let Some(ref author) = info.author {
            self.by_author
                .entry(author.clone())
                .or_default()
                .push(index);
        }
        
        self.cache.push(info);
    }

    /// Rebuild indices after cache modification
    fn rebuild_indices(&mut self) {
        self.by_category.clear();
        self.by_author.clear();
        
        for (index, info) in self.cache.iter().enumerate() {
            if let Some(ref category) = info.category {
                self.by_category
                    .entry(category.clone())
                    .or_default()
                    .push(index);
            }
            
            if let Some(ref author) = info.author {
                self.by_author
                    .entry(author.clone())
                    .or_default()
                    .push(index);
            }
        }
    }
}

impl Default for PresetBrowser {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a safe filename from a preset name
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            ' ' => '_',
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_browser_new_loads_builtins() {
        let browser = PresetBrowser::new();
        assert!(browser.count() >= 5);
        assert!(!browser.builtins().is_empty());
    }

    #[test]
    fn test_browser_search() {
        let browser = PresetBrowser::new();
        let results = browser.search("bass");
        assert!(!results.is_empty());
        assert!(results[0].name.to_lowercase().contains("bass"));
    }

    #[test]
    fn test_browser_search_case_insensitive() {
        let browser = PresetBrowser::new();
        let results_lower = browser.search("bass");
        let results_upper = browser.search("BASS");
        assert_eq!(results_lower.len(), results_upper.len());
    }

    #[test]
    fn test_browser_by_category() {
        let browser = PresetBrowser::new();
        let categories = browser.categories();
        assert!(categories.contains(&"Bass"));
        
        let bass_presets = browser.by_category("Bass");
        assert!(!bass_presets.is_empty());
    }

    #[test]
    fn test_browser_by_author() {
        let browser = PresetBrowser::new();
        let authors = browser.authors();
        assert!(authors.contains(&"Katie"));
        
        let katie_presets = browser.by_author("Katie");
        assert!(!katie_presets.is_empty());
    }

    #[test]
    fn test_browser_load_builtin() {
        let browser = PresetBrowser::new();
        let info = browser.builtins()[0];
        let preset = browser.load(info).unwrap();
        assert_eq!(preset.name, info.name);
    }

    #[test]
    fn test_browser_load_by_name() {
        let browser = PresetBrowser::new();
        let preset = browser.load_by_name("Classic Bass").unwrap();
        assert_eq!(preset.name, "Classic Bass");
    }

    #[test]
    fn test_browser_save_and_load() {
        let dir = tempdir().unwrap();
        let mut browser = PresetBrowser::new();
        browser.add_search_path(dir.path());
        
        let mut preset = Preset::default();
        preset.name = "Test Preset".to_string();
        preset.category = Some("Test".to_string());
        
        let path = browser.save(&preset, dir.path()).unwrap();
        assert!(path.exists());
        
        // Should be in cache now
        let results = browser.search("Test Preset");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_browser_delete() {
        let dir = tempdir().unwrap();
        let mut browser = PresetBrowser::new();
        browser.add_search_path(dir.path());
        
        let mut preset = Preset::default();
        preset.name = "To Delete".to_string();
        
        browser.save(&preset, dir.path()).unwrap();
        assert_eq!(browser.search("To Delete").len(), 1);
        
        let info = browser.search("To Delete")[0].clone();
        browser.delete(&info).unwrap();
        
        assert_eq!(browser.search("To Delete").len(), 0);
    }

    #[test]
    fn test_browser_cannot_delete_builtin() {
        let mut browser = PresetBrowser::new();
        let info = browser.builtins()[0].clone();
        let result = browser.delete(&info);
        assert!(result.is_err());
    }

    #[test]
    fn test_browser_user_presets() {
        let dir = tempdir().unwrap();
        let mut browser = PresetBrowser::new();
        browser.add_search_path(dir.path());
        
        let mut preset = Preset::default();
        preset.name = "User Sound".to_string();
        browser.save(&preset, dir.path()).unwrap();
        
        let user_presets = browser.user_presets();
        assert_eq!(user_presets.len(), 1);
        assert!(!user_presets[0].builtin);
    }

    #[test]
    fn test_browser_refresh() {
        let dir = tempdir().unwrap();
        let mut browser = PresetBrowser::new();
        browser.add_search_path(dir.path());
        
        let initial_count = browser.count();
        
        // Save a preset directly (not through browser)
        let preset = Preset::default();
        preset.save(dir.path().join("direct.json")).unwrap();
        
        // Refresh should pick it up
        browser.refresh();
        assert_eq!(browser.count(), initial_count + 1);
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("My Preset"), "My_Preset");
        assert_eq!(sanitize_filename("Test/Bad:Name"), "Test_Bad_Name");
        assert_eq!(sanitize_filename("Normal-Name_123"), "Normal-Name_123");
    }

    #[test]
    fn test_preset_info_from_preset() {
        let preset = Preset::default();
        let info = PresetInfo::from_preset(&preset, None);
        assert_eq!(info.name, preset.name);
        assert!(info.builtin);
        assert!(info.path.is_none());
        
        let path = PathBuf::from("/test/path.json");
        let info = PresetInfo::from_preset(&preset, Some(path.clone()));
        assert!(!info.builtin);
        assert_eq!(info.path, Some(path));
    }
}
