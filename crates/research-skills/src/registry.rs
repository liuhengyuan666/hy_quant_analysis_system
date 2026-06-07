use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use super::skill::Skill;

/// Registry for loading and querying skills
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
    skill_dir: PathBuf,
}

impl SkillRegistry {
    /// Create a new registry and load skills from directory
    pub fn new(skill_dir: PathBuf) -> Result<Self> {
        let mut registry = Self {
            skills: HashMap::new(),
            skill_dir,
        };
        registry.load_all()?;
        Ok(registry)
    }

    /// Load all skills from the skill directory
    fn load_all(&mut self) -> Result<()> {
        if !self.skill_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.skill_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let skill_md = path.join("SKILL.md");
                if skill_md.exists() {
                    match self.load_skill(&skill_md) {
                        Ok(skill) => {
                            self.skills
                                .insert(skill.definition.name.clone(), skill);
                        }
                        Err(e) => {
                            eprintln!(
                                "WARN: Failed to load skill from {:?}: {}",
                                skill_md, e
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Load a single skill from a SKILL.md file
    fn load_skill(&self, path: &Path) -> Result<Skill> {
        let content = std::fs::read_to_string(path)?;
        Skill::from_markdown(&content)
    }

    /// Get a skill by name
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// List all loaded skill names
    pub fn list(&self) -> Vec<&str> {
        self.skills.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a skill exists
    pub fn has(&self, name: &str) -> bool {
        self.skills.contains_key(name)
    }

    /// Get the number of loaded skills
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Reload all skills from disk
    pub fn reload(&mut self) -> Result<()> {
        self.skills.clear();
        self.load_all()
    }
}
