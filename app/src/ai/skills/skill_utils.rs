//! Utility functions for working with skills.

use super::{SkillDescriptor, SkillManager};
use crate::ai::blocklist::view_util::render_provider_icon_button;
use ai::skills::{
    home_skills_path, provider_rank, ParsedSkill, SkillProvider, SKILL_PROVIDER_DEFINITIONS,
};
use lazy_static::lazy_static;
use siphasher::sip::SipHasher;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::path::PathBuf;
use warp_core::ui::appearance::Appearance;
use warp_core::ui::theme::color::internal_colors;
use warp_core::ui::Icon;
use warpui::prelude::MouseStateHandle;
use warpui::EventContext;
use warpui::{AppContext, Element, SingletonEntity};

use crate::warp_managed_paths_watcher::warp_managed_skill_dirs;

lazy_static! {
    static ref CONTENT_HASHER: SipHasher = SipHasher::new_with_keys(0, 0);
}

/// Tries to insert or update a skill descriptor in the deduplication map.
/// If a skill with the same (directory, content) key already exists, keeps the one
/// from the higher-priority provider based on [`SKILL_PROVIDER_DEFINITIONS`].
fn try_insert_skill(
    dedup_map: &mut HashMap<u64, SkillDescriptor>,
    descriptor: SkillDescriptor,
    dir_path: &Path,
    content: &str,
) {
    let mut hasher = *CONTENT_HASHER;
    // Hash the directory path and content to create a unique key for deduplication.
    dir_path.hash(&mut hasher);
    content.hash(&mut hasher);
    let key = hasher.finish();
    match dedup_map.entry(key) {
        Entry::Vacant(e) => {
            e.insert(descriptor);
        }
        Entry::Occupied(mut e) => {
            // Prefer the skill from the higher-priority provider.
            if provider_rank(descriptor.provider) < provider_rank(e.get().provider) {
                e.insert(descriptor);
            }
        }
    }
}

/// Deduplicates skills with identical content installed under the same directory across
/// multiple providers, keeping the single best representative per
/// [`SKILL_PROVIDER_DEFINITIONS`] (index 0 = highest priority).
///
/// Two skills are considered duplicates only when they share the same owning directory
/// **and** identical content — which is the common case when a tool like `npx skills`
/// symlinks the same skill under `~/.agents/skills/`, `~/.warp/skills/`, `~/.claude/skills/`, etc.
///
/// After content dedup, additionally deduplicates by name: for skills with the same name,
/// keeps only the one with the highest provider priority (per [`provider_rank`]); same provider
/// rank breaks ties by shortest reference path. Final list is sorted by name for stable ordering.
///
/// Each element of `skill_paths` is a `(dir_path, skill_file_path)` tuple where
/// `dir_path` is the directory that owns the skill.
#[cfg_attr(not(feature = "local_fs"), allow(dead_code))]
pub(crate) fn unique_skills(
    skill_paths: &[(PathBuf, PathBuf)],
    skills_by_path: &HashMap<PathBuf, ParsedSkill>,
) -> Vec<SkillDescriptor> {
    // First pass: hash(dir_path + content) → best descriptor (symlink-dedup)
    let mut dedup_map: HashMap<u64, SkillDescriptor> = HashMap::new();

    for (dir_path, path) in skill_paths {
        if let Some(skill) = skills_by_path.get(path) {
            try_insert_skill(
                &mut dedup_map,
                SkillDescriptor::from(skill.clone()),
                dir_path,
                &skill.content,
            );
        }
    }

    // Second pass: name → best descriptor (name-dedup), preferring higher provider
    // priority, then shortest reference path for same-provider ties.
    let mut name_map: HashMap<String, SkillDescriptor> = HashMap::new();
    for skill in dedup_map.into_values() {
        match name_map.entry(skill.name.clone()) {
            Entry::Vacant(e) => {
                e.insert(skill);
            }
            Entry::Occupied(mut e) => {
                let existing_rank = provider_rank(e.get().provider);
                let new_rank = provider_rank(skill.provider);
                if new_rank < existing_rank {
                    // Higher priority provider wins
                    e.insert(skill);
                } else if new_rank == existing_rank
                    && reference_path_len(&skill.reference)
                        < reference_path_len(&e.get().reference)
                {
                    // Same provider rank → shorter path wins
                    e.insert(skill);
                }
            }
        }
    }

    let mut skills: Vec<SkillDescriptor> = name_map.into_values().collect();
    skills.sort_by(|a, b| a.name.cmp(&b.name));
    skills
}

/// Length of the reference path string, used as tiebreaker between same-provider skills.
fn reference_path_len(reference: &ai::skills::SkillReference) -> usize {
    match reference {
        ai::skills::SkillReference::Path(p) => p.to_string_lossy().len(),
        ai::skills::SkillReference::BundledSkillId(id) => id.len(),
    }
}

/// 列出当前 working directory 适用的全部 skills。
///
/// **设计说明**:旧版 `list_skills_if_changed` 在云端协议下做差量发送(对比上轮已发的
/// `conversation.latest_skills()`,未变化时返回 `None`)以节省上行 token —— warp 后端
/// 维护会话状态,首轮收到后保留即可。项目去云端后,BYOP 走 OpenAI/Anthropic 等无状态
/// `/chat/completions`,system prompt 每轮在客户端完整重渲染,数据必须每轮都送达,
/// 否则第二轮起 system prompt 里 skills section 会消失。
/// 因此简化为每轮全量返回。
pub fn list_skills(working_directory: Option<&Path>, app: &AppContext) -> Vec<SkillDescriptor> {
    SkillManager::as_ref(app).get_skills_for_working_directory(working_directory, app)
}

/// Renders an 'open skill' button for blocklist AI actions and the code diff view.
pub fn render_skill_button<F>(
    button_label: &str,
    button_handle: MouseStateHandle,
    appearance: &Appearance,
    skill_provider: SkillProvider,
    icon_override: Option<Icon>,
    on_click: F,
) -> Box<dyn Element>
where
    F: FnMut(&mut EventContext) + 'static,
{
    let theme = appearance.theme();
    let logo_fill = internal_colors::fg_overlay_6(theme);

    let icon = icon_override.unwrap_or_else(|| skill_provider.icon());

    let color = if icon_override.is_some() {
        logo_fill
    } else {
        skill_provider.icon_fill(logo_fill)
    };

    render_provider_icon_button(
        button_label,
        button_handle,
        appearance,
        icon,
        color,
        on_click,
    )
}

/// Returns a branded icon override for well-known skill names.
pub fn icon_override_for_skill_name(name: &str) -> Option<Icon> {
    match name {
        "stripe-projects-cli" => Some(Icon::StripeLogo),
        _ => None,
    }
}

pub fn skill_path_from_file_path(file_path: &Path) -> Option<PathBuf> {
    for definition in SKILL_PROVIDER_DEFINITIONS.iter() {
        let home_skill_dirs = if definition.provider == SkillProvider::Warp {
            warp_managed_skill_dirs()
        } else {
            home_skills_path(definition.provider).into_iter().collect()
        };
        for home_skills_path in home_skill_dirs {
            if let Ok(relative_path) = file_path.strip_prefix(&home_skills_path) {
                let skill_name = relative_path.components().next()?;
                return Some(home_skills_path.join(skill_name).join("SKILL.md"));
            }
        }
    }
    let path_components: Vec<_> = file_path.components().collect();

    for def in SKILL_PROVIDER_DEFINITIONS.iter() {
        let skill_components: Vec<_> = def.skills_path.components().collect();

        for (idx, window) in path_components.windows(skill_components.len()).enumerate() {
            if window == skill_components.as_slice() {
                let skill_dir = PathBuf::from_iter(
                    file_path
                        .components()
                        .take(idx + skill_components.len() + 1),
                );
                return Some(skill_dir.join("SKILL.md"));
            }
        }
    }
    None
}

#[cfg(test)]
#[path = "skill_utils_tests.rs"]
mod tests;
