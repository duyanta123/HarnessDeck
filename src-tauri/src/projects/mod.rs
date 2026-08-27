//! Every project this window can host, and which of them it is pointed at.
//!
//! A project is a local folder plus the DSH profile whose credentials, plugins
//! and session context belong to the work done in that folder. The registry is
//! deliberately a small single file: projects stay ground truth on disk and the
//! profile keeps every secret exactly where DSH already puts it.
pub mod commands;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::paths;

const SELECTION_FILE: &str = "projects.json";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub profile: String,
    pub last_opened_at: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    #[serde(default)]
    pub selected: Option<String>,
    #[serde(default)]
    pub projects: Vec<Project>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Roster {
    pub projects: Vec<Project>,
    pub selected: String,
}

#[derive(Clone)]
pub struct Store {
    file: PathBuf,
}

impl Store {
    pub fn managed() -> Self {
        Self {
            file: paths::app_data_dir().join(SELECTION_FILE),
        }
    }

    /// Every project on disk, or the first-run default if none has been written
    /// yet. Reading is infallible because a missing or damaged registry must not
    /// keep the shell from opening; the old global workspace selection is the
    /// migration source and produces exactly one default project.
    pub fn load(&self) -> Registry {
        let parsed = std::fs::read(&self.file)
            .ok()
            .and_then(|body| serde_json::from_slice::<Registry>(&body).ok())
            .filter(|registry| !registry.projects.is_empty());

        let registry = parsed.unwrap_or_else(|| self.migrate());
        if registry.projects.len() == 1
            && registry.projects[0].id == "default"
            && std::fs::read(&self.file).ok().and_then(|body| serde_json::from_slice::<Registry>(&body).ok()).is_none()
        {
            let _ = self.save(&registry);
        }
        registry
    }

    fn migrate(&self) -> Registry {
        let path = crate::workspace::selected();
        let name = path
            .file_name()
            .map(|segment| segment.to_string_lossy().into_owned())
            .filter(|segment| !segment.trim().is_empty())
            .unwrap_or_else(|| "Default project".to_string());
        let profile = crate::profiles::selected();
        let project = Project {
            id: "default".to_string(),
            name,
            path,
            profile,
            last_opened_at: now_millis(),
        };
        Registry {
            selected: Some(project.id.clone()),
            projects: vec![project],
        }
    }

    pub fn save(&self, registry: &Registry) -> Result<()> {
        if let Some(parent) = self.file.parent() {
            std::fs::create_dir_all(parent).map_err(|cause| {
                Error::Project(format!(
                    "{} could not be created: {cause}",
                    parent.display()
                ))
            })?;
        }
        let mut body = serde_json::to_vec_pretty(registry)
            .map_err(|cause| Error::Project(format!("project state could not be encoded: {cause}")))?;
        body.push(b'\n');
        crate::atomic::write(&self.file, body).map_err(|cause| {
            Error::Project(format!(
                "{} could not be committed: {cause}",
                self.file.display()
            ))
        })
    }

    fn choose(&self, id: &str) -> Result<Roster> {
        let mut registry = self.load();
        let profile = {
            let Some(project) = registry.projects.iter_mut().find(|project| project.id == id) else {
                return Err(Error::Project(format!("there is no project with id {id}")));
            };
            if !profile_exists(&project.profile) {
                return Err(Error::Project(format!(
                    "project {} is bound to profile {}, which no longer exists; edit the project and bind another profile",
                    project.name, project.profile
                )));
            }
            project.last_opened_at = now_millis();
            registry.selected = Some(project.id.clone());
            project.profile.clone()
        };
        self.save(&registry)?;
        crate::profiles::select(&profile)?;
        Ok(roster_of(registry))
    }
}

fn roster_of(registry: Registry) -> Roster {
    let selected = active_id(&registry);
    Roster {
        projects: registry.projects,
        selected,
    }
}

fn active_id(registry: &Registry) -> String {
    registry
        .selected
        .clone()
        .filter(|id| registry.projects.iter().any(|project| project.id == *id))
        .or_else(|| registry.projects.first().map(|project| project.id.clone()))
        .unwrap_or_else(|| "default".to_string())
}

/// Every project on the machine.
pub fn roster() -> Roster {
    let registry = Store::managed().load();
    roster_of(registry)
}

/// The project the next Harness start serves, if a registry exists.
pub fn active() -> Option<Project> {
    let registry = Store::managed().load();
    let selected = active_id(&registry);
    registry
        .projects
        .iter()
        .find(|project| project.id == selected)
        .cloned()
        .or_else(|| registry.projects.first().cloned())
}

/// The workspace selected by the active project, if one exists.
pub fn active_workspace() -> Option<PathBuf> {
    active().map(|project| project.path)
}

/// The profile selected by the active project, if one exists.
pub fn active_profile() -> Option<String> {
    active().map(|project| project.profile)
}

/// Validate and remember a project directory. Network and removable drives are
/// rejected with the same rules as the legacy global workspace.
pub fn inspect_path(path: &Path) -> Result<PathBuf> {
    let canonical = path.canonicalize().map_err(|cause| {
        Error::Project(format!("{} could not be opened: {cause}", path.display()))
    })?;
    let canonical = node_runtime::plain_path(canonical);
    let admission = crate::workspace::inspect(&canonical);
    if admission.blocked() {
        return Err(Error::Project(admission.reason.unwrap_or_else(|| {
            "the selected directory is not a safe workspace".into()
        })));
    }
    Ok(canonical)
}

pub fn add(name: Option<String>, path: PathBuf, profile: Option<String>) -> Result<Roster> {
    let path = inspect_path(&path)?;
    let mut registry = Store::managed().load();

    // A folder already admitted as a project is switched to, not duplicated.
    if let Some(existing) = registry.projects.iter().find(|project| project.path == path) {
        let profile = existing.profile.clone();
        registry.selected = Some(existing.id.clone());
        Store::managed().save(&registry)?;
        crate::profiles::select(&profile)?;
        return Ok(roster_of(registry));
    }

    let name = clean_name(name, &path);
    let profile = resolve_profile(&name, profile)?;

    let id = unique_id(&registry, &name);
    registry.projects.push(Project {
        id: id.clone(),
        name,
        path,
        profile,
        last_opened_at: now_millis(),
    });
    registry.selected = Some(id);
    let selected_profile = registry
        .projects
        .last()
        .map(|project| project.profile.clone())
        .unwrap_or_else(|| crate::profiles::selected());
    Store::managed().save(&registry)?;
    crate::profiles::select(&selected_profile)?;
    Ok(roster_of(registry))
}

pub fn remove(id: &str) -> Result<Roster> {
    let mut registry = Store::managed().load();
    if registry.projects.len() <= 1 {
        return Err(Error::Project(
            "the last project cannot be removed; add another one first".into(),
        ));
    }
    let before = registry.projects.len();
    registry.projects.retain(|project| project.id != id);
    if registry.projects.len() == before {
        return Err(Error::Project(format!("there is no project with id {id}")));
    }
    if registry.selected.as_deref() == Some(id) {
        registry.selected = registry.projects.first().map(|project| project.id.clone());
    }
    Store::managed().save(&registry)?;
    Ok(roster_of(registry))
}

pub fn rename(id: &str, name: String) -> Result<Roster> {
    let name = name.trim().to_string();
    if name.is_empty() || name.len() > 80 {
        return Err(Error::Project(
            "project names must be between 1 and 80 characters".into(),
        ));
    }
    let mut registry = Store::managed().load();
    let Some(project) = registry.projects.iter_mut().find(|project| project.id == id) else {
        return Err(Error::Project(format!("there is no project with id {id}")));
    };
    project.name = name;
    Store::managed().save(&registry)?;
    Ok(roster_of(registry))
}

pub fn bind_profile(id: &str, profile: String) -> Result<Roster> {
    let profile = profile.trim().to_string();
    if !profile_exists(&profile) {
        return Err(Error::Project(format!(
            "there is no profile called {profile}"
        )));
    }
    let mut registry = Store::managed().load();
    let (selected, profile_name) = {
        let Some(project) = registry.projects.iter_mut().find(|project| project.id == id) else {
            return Err(Error::Project(format!("there is no project with id {id}")));
        };
        project.profile = profile.clone();
        (registry.selected.as_deref() == Some(id), project.profile.clone())
    };
    Store::managed().save(&registry)?;
    if selected {
        crate::profiles::select(&profile_name)?;
    }
    Ok(roster_of(registry))
}
/// Point the active project at a profile chosen from the standalone profile
/// switcher. Keeps the title-bar chips and `launch_plan` from disagreeing.
pub fn bind_active_profile(name: &str) -> Result<Roster> {
    let profile = name.trim().to_string();
    if !profile_exists(&profile) {
        return Err(Error::Project(format!(
            "there is no profile called {profile}"
        )));
    }
    let mut registry = Store::managed().load();
    let selected = active_id(&registry);
    let Some(project) = registry.projects.iter_mut().find(|project| project.id == selected) else {
        return Ok(roster_of(registry));
    };
    project.profile = profile;
    Store::managed().save(&registry)?;
    Ok(roster_of(registry))
}

pub fn select(id: &str) -> Result<Roster> {
    Store::managed().choose(id)
}

fn clean_name(name: Option<String>, path: &Path) -> String {
    name.map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty() && name.len() <= 80)
        .or_else(|| {
            path.file_name()
                .map(|segment| segment.to_string_lossy().into_owned())
                .filter(|segment| !segment.trim().is_empty())
        })
        .unwrap_or_else(|| "Untitled project".to_string())
}

fn profile_exists(name: &str) -> bool {
    crate::profiles::roster()
        .profiles
        .iter()
        .any(|profile| profile.name == name)
}

fn resolve_profile(project_name: &str, requested: Option<String>) -> Result<String> {
    if let Some(requested) = requested {
        let requested = requested.trim().to_string();
        if !requested.is_empty() {
            if !profile_exists(&requested) {
                return Err(Error::Project(format!(
                    "there is no profile called {requested}"
                )));
            }
            return Ok(requested);
        }
    }

    let candidate = auto_profile_name(project_name);
    if profile_exists(&candidate) {
        return Ok(candidate);
    }
    if crate::profiles::is_new_name(&candidate) {
        if crate::profiles::create(&candidate).is_ok() && profile_exists(&candidate) {
            return Ok(candidate);
        }
    }
    Ok(crate::profiles::selected())
}

fn auto_profile_name(project_name: &str) -> String {
    let mut slug = String::new();
    for character in project_name.chars() {
        let lowered = character.to_ascii_lowercase();
        if lowered.is_ascii_lowercase() || lowered.is_ascii_digit() {
            slug.push(lowered);
        } else if character.is_whitespace() || matches!(character, '-' | '_') {
            if !slug.ends_with('-') {
                slug.push('-');
            }
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "proj-default".to_string()
    } else {
        format!("proj-{slug}")
    }
}

fn unique_id(registry: &Registry, name: &str) -> String {
    let mut base = auto_profile_name(name);
    if let Some(stripped) = base.strip_prefix("proj-") {
        base = stripped.to_string();
    }
    if base.is_empty() {
        base = "project".to_string();
    }
    for index in 1..16_384 {
        let candidate = format!("{base}-{index}");
        if !registry.projects.iter().any(|project| project.id == candidate) {
            return candidate;
        }
    }
    format!("{base}-{}", now_millis())
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_name_is_cleaned() {
        assert_eq!(clean_name(Some("  ".into()), Path::new("C:\\work")), "work");
        assert_eq!(
            clean_name(Some("Office".into()), Path::new("C:\\work")),
            "Office"
        );
        assert_eq!(auto_profile_name("My Project 2"), "proj-my-project-2");
    }

    #[test]
    fn an_unsafe_directory_is_rejected() {
        let missing = std::env::temp_dir().join("dsh-studio-project-that-must-not-exist");
        assert!(inspect_path(&missing).is_err());
    }
#[test]
    fn a_registry_survives_a_restart() {
        let root = std::env::temp_dir().join(format!(
            "dsh-studio-projects-persistence-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let store = Store {
            file: root.join("projects.json"),
        };
        let registry = Registry {
            selected: Some("b".into()),
            projects: vec![
                Project {
                    id: "a".into(),
                    name: "A".into(),
                    path: PathBuf::from("C:\\work-a"),
                    profile: "web".into(),
                    last_opened_at: 1,
                },
                Project {
                    id: "b".into(),
                    name: "B".into(),
                    path: PathBuf::from("C:\\work-b"),
                    profile: "work".into(),
                    last_opened_at: 2,
                },
            ],
        };

        store.save(&registry).expect("save");
        let loaded = store.load();
        assert_eq!(loaded.projects.len(), 2);
        assert_eq!(active_id(&loaded), "b");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_registry_keeps_the_selected_project() {
        let registry = Registry {
            selected: None,
            projects: vec![Project {
                id: "a".into(),
                name: "A".into(),
                path: PathBuf::from("C:\\work"),
                profile: "web".into(),
                last_opened_at: 1,
            }],
        };
        assert_eq!(active_id(&registry), "a");
    }
}