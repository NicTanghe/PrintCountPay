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
pub(crate) struct OidLabel {
    pub(crate) oid: Oid,
    pub(crate) label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MachineProfile {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) manufacturer: String,
    #[serde(default)]
    pub(crate) firmware: String,
    pub(crate) recording: RecordingOidProfile,
    pub(crate) counters: CounterOidSet,
    #[serde(default)]
    pub(crate) toner: TonerOidProfile,
    #[serde(default)]
    pub(crate) extra_poll_labels: Vec<OidLabel>,
    #[serde(default)]
    pub(crate) counter_table: Option<String>,
    #[serde(default)]
    pub(crate) legacy_profile_ids: Vec<String>,
    #[serde(default)]
    pub(crate) matchers: Vec<MachineMatcher>,
    #[serde(skip)]
    pub(crate) source_path: Option<PathBuf>,
}

impl MachineProfile {
    pub(crate) fn id(&self) -> String {
        self.id.clone()
    }

    pub(crate) fn legacy_profile_ids(&self) -> Vec<String> {
        let mut ids = self.legacy_profile_ids.clone();
        if !self.manufacturer.trim().is_empty() && !self.firmware.trim().is_empty() {
            ids.push(format!("{}/{}", self.manufacturer, self.firmware));
        }
        ids.sort();
        ids.dedup();
        ids
    }
}

pub(crate) type ManufacturerProfile = MachineProfile;

#[derive(Debug, Clone, Default)]
pub(crate) struct ProfileAliases {
    legacy_to_current: HashMap<String, String>,
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
            let value = sys_object_id?;
            if !value.trim().starts_with(prefix.trim()) {
                return None;
            }
            score = score.saturating_add(1);
        }

        if let Some(needle) = self.sys_descr_contains.as_deref() {
            let value = sys_descr?;
            if !value
                .to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
            {
                return None;
            }
            score = score.saturating_add(2);
        }

        if let Some(needle) = self.model_contains.as_deref() {
            let value = model?;
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
    aliases: ProfileAliases,
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

    pub(crate) fn migrate_profile_id(&self, id: &str) -> Option<String> {
        if self.profiles.contains_key(id) {
            return Some(id.to_string());
        }
        self.aliases.legacy_to_current.get(id).cloned()
    }

    pub(crate) fn upsert_profile(&mut self, profile: MachineProfile) {
        let id = profile.id();
        self.profiles.insert(id.clone(), profile.clone());

        if let Some(existing) = self.machines.iter_mut().find(|machine| machine.id == id) {
            *existing = profile.clone();
        } else {
            self.machines.push(profile.clone());
        }

        for legacy_id in profile.legacy_profile_ids() {
            self.aliases.legacy_to_current.insert(legacy_id, id.clone());
        }
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

            let profile_id = machine.id();
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

        if tied { None } else { best.map(|(id, _)| id) }
    }
}

pub(crate) fn load_profile_index(root: &Path) -> (ProfileIndex, Option<String>) {
    let mut index = ProfileIndex::default();
    let mut errors = Vec::new();
    let machines_dir = root.join("machines");

    for path in collect_ron_files(&machines_dir) {
        match fs::read_to_string(&path) {
            Ok(contents) => match from_str::<MachineProfile>(&contents) {
                Ok(mut profile) => {
                    if profile.id.trim().is_empty() {
                        errors.push(format!(
                            "Failed to load machine {}: missing profile id",
                            path.display()
                        ));
                        continue;
                    }

                    profile.source_path = Some(path.clone());
                    let id = profile.id();

                    if index.profiles.insert(id.clone(), profile.clone()).is_some() {
                        errors.push(format!(
                            "Duplicate machine profile id {id} at {}",
                            path.display()
                        ));
                    }

                    for legacy_id in profile.legacy_profile_ids() {
                        if let Some(previous) = index
                            .aliases
                            .legacy_to_current
                            .insert(legacy_id.clone(), id.clone())
                            && previous != id
                        {
                            errors.push(format!(
                                "Duplicate legacy profile id {legacy_id} for {previous} and {id}"
                            ));
                        }
                    }

                    index.machines.push(profile);
                }
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

pub(crate) fn profile_path(root: &Path, profile: &MachineProfile) -> PathBuf {
    profile
        .source_path
        .clone()
        .unwrap_or_else(|| root.join("machines").join(format!("{}.ron", profile.id)))
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

#[cfg(test)]
mod tests {
    use super::MachineProfile;
    use printcountpay_core::Oid;
    use ron::de::from_str;

    fn oid(value: &str) -> Oid {
        value.parse().expect("valid oid")
    }

    #[test]
    fn c4502_profile_uses_panel_matching_recording_counters() {
        let profile = from_str::<MachineProfile>(include_str!(
            "../../../profiles/machines/ricoh-aficio-mp-c4502.ron"
        ))
        .expect("c4502 profile should parse");

        assert_eq!(profile.id, "ricoh-aficio-mp-c4502");
        assert_eq!(
            profile.recording.copies_bw,
            vec![oid("1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.3")]
        );
        assert_eq!(
            profile.recording.copies_color,
            vec![oid("1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.5")]
        );
        assert_eq!(
            profile.recording.prints_bw,
            vec![oid("1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.9")]
        );
        assert_eq!(
            profile.recording.prints_color,
            vec![oid("1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.11")]
        );

        assert_eq!(
            profile.counters.bw,
            vec![oid("1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.22")]
        );
        assert_eq!(
            profile.counters.color,
            vec![oid("1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.21")]
        );
        assert_eq!(
            profile.counters.total,
            vec![oid("1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.59")]
        );
    }

    #[test]
    fn c7200_profile_uses_verified_panel_matching_recording_counters() {
        let profile = from_str::<MachineProfile>(include_str!(
            "../../../profiles/machines/ricoh-pro-c7200s-light.ron"
        ))
        .expect("c7200 profile should parse");

        assert_eq!(profile.id, "ricoh-pro-c7200s-light");
        assert_eq!(
            profile.recording.copies_bw,
            vec![oid("1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.3")]
        );
        assert_eq!(
            profile.recording.copies_color,
            vec![oid("1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.5")]
        );
        assert_eq!(
            profile.recording.prints_bw,
            vec![oid("1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.9")]
        );
        assert_eq!(
            profile.recording.prints_color,
            vec![oid("1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.11")]
        );
    }
}
