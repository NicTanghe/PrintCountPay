use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use ron::de::from_str;
use serde::{Deserialize, Serialize};

use printcountpay_core::{CounterOidSet, Oid};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RecordingOidProfile {
    pub(crate) copies_bw: Vec<Oid>,
    pub(crate) copies_color: Vec<Oid>,
    pub(crate) prints_bw: Vec<Oid>,
    pub(crate) prints_color: Vec<Oid>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct TonerOidProfile {
    pub(crate) black: Option<Oid>,
    pub(crate) cyan: Option<Oid>,
    pub(crate) magenta: Option<Oid>,
    pub(crate) yellow: Option<Oid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ManufacturerProfile {
    pub(crate) manufacturer: String,
    pub(crate) firmware: String,
    pub(crate) recording: RecordingOidProfile,
    pub(crate) counters: CounterOidSet,
    #[serde(default)]
    pub(crate) toner: TonerOidProfile,
    #[serde(default)]
    pub(crate) extra_poll_oids: Vec<Oid>,
    #[serde(default)]
    pub(crate) counter_table: Option<String>,
}

impl ManufacturerProfile {
    pub(crate) fn id(&self) -> String {
        format!("{}/{}", self.manufacturer, self.firmware)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MachineProfile {
    pub(crate) id: String,
    pub(crate) manufacturer: String,
    pub(crate) firmware: String,
    #[serde(default)]
    pub(crate) matchers: Vec<MachineMatcher>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MachineMatcher {
    pub(crate) sys_object_id_prefix: Option<String>,
    pub(crate) sys_descr_contains: Option<String>,
    pub(crate) model_contains: Option<String>,
}

impl MachineMatcher {
    fn match_score(
        &self,
        sys_object_id: Option<&str>,
        sys_descr: Option<&str>,
        model: Option<&str>,
    ) -> Option<u8> {
        let mut score = 0u8;

        if let Some(prefix) = self.sys_object_id_prefix.as_deref() {
            let Some(value) = sys_object_id else {
                return None;
            };
            if !value.trim().starts_with(prefix.trim()) {
                return None;
            }
            score = score.saturating_add(1);
        }

        if let Some(needle) = self.sys_descr_contains.as_deref() {
            let Some(value) = sys_descr else {
                return None;
            };
            if !value
                .to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
            {
                return None;
            }
            score = score.saturating_add(2);
        }

        if let Some(needle) = self.model_contains.as_deref() {
            let Some(value) = model else {
                return None;
            };
            if !value
                .to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
            {
                return None;
            }
            score = score.saturating_add(3);
        }

        Some(score)
    }
}

#[derive(Debug, Default)]
pub(crate) struct ProfileIndex {
    pub(crate) profiles: HashMap<String, ManufacturerProfile>,
    pub(crate) machines: Vec<MachineProfile>,
}

impl ProfileIndex {
    pub(crate) fn profile_ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.profiles.keys().cloned().collect();
        ids.sort();
        ids
    }

    pub(crate) fn profile(&self, id: &str) -> Option<&ManufacturerProfile> {
        self.profiles.get(id)
    }

    pub(crate) fn match_profile_id(
        &self,
        sys_object_id: Option<&str>,
        sys_descr: Option<&str>,
        model: Option<&str>,
    ) -> Option<String> {
        let mut best: Option<(String, u8)> = None;
        let mut tied = false;

        for machine in &self.machines {
            let mut best_score: Option<u8> = None;
            for matcher in &machine.matchers {
                if let Some(score) = matcher.match_score(sys_object_id, sys_descr, model) {
                    best_score = Some(best_score.map_or(score, |current| current.max(score)));
                }
            }

            let Some(score) = best_score else {
                continue;
            };

            let profile_id = format!("{}/{}", machine.manufacturer, machine.firmware);
            match best {
                None => {
                    best = Some((profile_id, score));
                    tied = false;
                }
                Some((_, best_score)) => {
                    if score > best_score {
                        best = Some((profile_id, score));
                        tied = false;
                    } else if score == best_score {
                        tied = true;
                    }
                }
            }
        }

        if tied {
            None
        } else {
            best.map(|(id, _)| id)
        }
    }
}

pub(crate) fn load_profile_index(root: &Path) -> (ProfileIndex, Option<String>) {
    let mut index = ProfileIndex::default();
    let mut errors = Vec::new();

    let manufacturers_dir = root.join("manufacturers");
    let machines_dir = root.join("machines");

    for path in collect_ron_files(&manufacturers_dir) {
        match fs::read_to_string(&path) {
            Ok(contents) => match from_str::<ManufacturerProfile>(&contents) {
                Ok(profile) => {
                    index.profiles.insert(profile.id(), profile);
                }
                Err(error) => errors.push(format!(
                    "Failed to parse profile {}: {error}",
                    path.display()
                )),
            },
            Err(error) => errors.push(format!(
                "Failed to read profile {}: {error}",
                path.display()
            )),
        }
    }

    for path in collect_ron_files(&machines_dir) {
        match fs::read_to_string(&path) {
            Ok(contents) => match from_str::<MachineProfile>(&contents) {
                Ok(profile) => index.machines.push(profile),
                Err(error) => errors.push(format!(
                    "Failed to parse machine {}: {error}",
                    path.display()
                )),
            },
            Err(error) => errors.push(format!(
                "Failed to read machine {}: {error}",
                path.display()
            )),
        }
    }

    let status = if errors.is_empty() {
        None
    } else {
        Some(errors.join(" | "))
    };

    (index, status)
}

pub(crate) fn profile_path(root: &Path, manufacturer: &str, firmware: &str) -> PathBuf {
    root.join("manufacturers")
        .join(manufacturer)
        .join(format!("{firmware}.ron"))
}

fn collect_ron_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(collect_ron_files(&path));
            } else if path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("ron"))
                .unwrap_or(false)
            {
                files.push(path);
            }
        }
    }
    files
}
