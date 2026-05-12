use std::path::{Path, PathBuf};

use ai::skills::{ParsedSkill, SkillProvider, SkillReference, SkillScope};
use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

use crate::ai::skills::SkillDescriptor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillManagerEvent {
    InventoryChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillInventoryDuplicate {
    pub path: PathBuf,
    pub name: String,
    pub description: String,
    pub content: String,
    pub provider: SkillProvider,
    pub scope: SkillScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillInventoryItem {
    pub name: String,
    pub default_skill: SkillInventoryDuplicate,
    pub duplicates: Vec<SkillInventoryDuplicate>,
}

impl SkillInventoryItem {
    pub fn has_duplicates(&self) -> bool {
        self.duplicates.len() > 1
    }
}

pub struct SkillManager {}

impl SkillManager {
    pub fn new(_ctx: &mut ModelContext<Self>) -> Self {
        Self {}
    }

    pub fn get_skills_for_working_directory(
        &self,
        _working_directory: Option<&Path>,
        _ctx: &AppContext,
    ) -> Vec<SkillDescriptor> {
        vec![]
    }

    pub fn skill_by_path(&self, _skill_path: &Path) -> Option<&ParsedSkill> {
        None
    }

    pub fn list_skill_inventory(&self, ctx: &AppContext) -> Vec<SkillInventoryItem> {
        let _ = ctx;
        vec![]
    }

    pub fn reference_for_skill_path(&self, skill_path: &Path) -> SkillReference {
        SkillReference::Path(skill_path.to_path_buf())
    }

    pub fn skill_by_reference(&self, _reference: &SkillReference) -> Option<&ParsedSkill> {
        None
    }

    pub fn active_bundled_skill(&self, _id: &str, _ctx: &AppContext) -> Option<&ParsedSkill> {
        None
    }

    pub fn skill_exists_for_any_provider(
        &self,
        _skill: &SkillDescriptor,
        _providers: &[SkillProvider],
    ) -> bool {
        false
    }

    pub fn best_supported_provider(
        &self,
        skill: &SkillDescriptor,
        _supported_providers: &[SkillProvider],
    ) -> SkillProvider {
        skill.provider
    }
}

impl Entity for SkillManager {
    type Event = SkillManagerEvent;
}

impl SingletonEntity for SkillManager {}
