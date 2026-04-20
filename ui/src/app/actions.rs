const MANUAL_BILL_ADJECTIVES: &[&str] = &[
    "amber", "ancient", "autumn", "bright", "brisk", "calm", "cedar", "clear", "cloudy", "copper",
    "coral", "cosmic", "crisp", "dusty", "ember", "fern", "gentle", "golden", "granite", "harbor",
    "hazel", "hidden", "ivory", "jade", "lilac", "linen", "lively", "lunar", "mellow", "misty",
    "noble", "ochre", "olive", "opal", "paper", "pearl", "quiet", "radiant", "river", "rustic",
    "saffron", "satin", "silver", "soft", "solar", "steady", "stone", "summer", "tender", "velvet",
    "vivid", "warm", "wild", "willow", "winter", "woodland", "zephyr",
];

const MANUAL_BILL_SUBJECTS: &[&str] = &[
    "atlas", "aurora", "beacon", "birch", "bloom", "breeze", "brook", "canvas", "cinder",
    "circuit", "cloud", "comet", "cove", "crest", "dawn", "ember", "field", "flame", "forest",
    "garden", "glow", "grove", "harbor", "horizon", "island", "journal", "lantern", "leaf",
    "meadow", "mesa", "mirror", "mosaic", "notebook", "orbit", "paper", "pebble", "pine", "plume",
    "prairie", "quartz", "rain", "reef", "river", "shadow", "signal", "sketch", "song", "sparrow",
    "stone", "summit", "terrace", "thicket", "trail", "valley", "vista", "willow", "wind",
    "wonder",
];

const MANUAL_PRICING_MAX_BACKUPS: usize = 3;

fn title_case_word(value: &str) -> String {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };

    first.to_ascii_uppercase().to_string() + chars.as_str()
}

fn parse_manual_pricing_contents(contents: &str) -> Result<ManualPricingWorkspace, String> {
    match from_str::<ManualPricingWorkspace>(contents) {
        Ok(mut workspace) => {
            workspace.normalize();
            Ok(workspace)
        }
        Err(workspace_error) => match from_str::<ManualPricingSettings>(contents) {
            Ok(mut settings) => {
                settings.normalize();
                Ok(ManualPricingWorkspace {
                    settings,
                    bills: Vec::new(),
                    bill_tombstones: Vec::new(),
                })
            }
            Err(settings_error) => Err(format!(
                "{workspace_error} | legacy fallback: {settings_error}"
            )),
        },
    }
}

fn parse_manual_bill_store_contents(contents: &str) -> Result<ManualBillStore, String> {
    match from_str::<ManualBillStore>(contents) {
        Ok(mut store) => {
            store.normalize();
            Ok(store)
        }
        Err(store_error) => match from_str::<Vec<ManualPricingBill>>(contents) {
            Ok(mut bills) => {
                for bill in &mut bills {
                    bill.normalize();
                }
                Ok(ManualBillStore {
                    bills,
                    bill_tombstones: Vec::new(),
                })
            }
            Err(legacy_error) => Err(format!("{store_error} | legacy fallback: {legacy_error}")),
        },
    }
}

fn pricing_sync_id_value(id: &str) -> Option<u128> {
    id.parse::<u128>().ok()
}

fn current_pricing_sync_id() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn manual_pricing_version_id(path: &Path) -> Option<String> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_nanos().to_string())
}

fn pricing_sync_is_stale(candidate_id: &str, current_id: Option<&str>) -> bool {
    let Some(current_id) = current_id else {
        return false;
    };

    match (
        pricing_sync_id_value(candidate_id),
        pricing_sync_id_value(current_id),
    ) {
        (Some(candidate), Some(current)) => candidate <= current,
        _ => candidate_id == current_id,
    }
}

fn should_preserve_local_stopped_session(
    local: &RecordingSession,
    incoming: &RecordingSession,
) -> bool {
    if local.active || !incoming.active {
        return false;
    }

    let Some(local_end_at) = local.end.as_ref().map(|snapshot| snapshot.received_at) else {
        return false;
    };

    let incoming_start_at = incoming
        .start
        .as_ref()
        .map(|snapshot| snapshot.received_at)
        .unwrap_or(0);
    incoming_start_at <= local_end_at
}

fn should_preserve_local_active_session(
    local: &RecordingSession,
    incoming: &RecordingSession,
) -> bool {
    if !local.active || incoming.active {
        return false;
    }

    let Some(local_start_at) = local.start.as_ref().map(|snapshot| snapshot.received_at) else {
        return false;
    };

    let incoming_latest_at = incoming
        .end
        .as_ref()
        .map(|snapshot| snapshot.received_at)
        .or_else(|| incoming.start.as_ref().map(|snapshot| snapshot.received_at))
        .unwrap_or(0);
    incoming_latest_at < local_start_at
}

fn prefer_local_recording_session(
    local: &RecordingSession,
    incoming: Option<&RecordingSession>,
) -> bool {
    let Some(incoming) = incoming else {
        return local.has_state();
    };

    let local_manual_state_changed_at = local.manual_state_changed_at_millis;
    let incoming_manual_state_changed_at = incoming.manual_state_changed_at_millis;
    if (local_manual_state_changed_at != 0 || incoming_manual_state_changed_at != 0)
        && local_manual_state_changed_at != incoming_manual_state_changed_at
    {
        return local_manual_state_changed_at > incoming_manual_state_changed_at;
    }

    if should_preserve_local_stopped_session(local, incoming)
        || should_preserve_local_active_session(local, incoming)
    {
        return true;
    }

    let local_version = local.version_millis();
    let incoming_version = incoming.version_millis();
    if local_version != incoming_version {
        return local_version > incoming_version;
    }

    local.end.is_some() && incoming.end.is_none()
}

fn prefer_local_poll_state(local: &SnmpPollStatus, incoming: &SnmpPollStatus) -> bool {
    match (poll_received_at(local), poll_received_at(incoming)) {
        (Some(local_received_at), Some(incoming_received_at)) => {
            local_received_at > incoming_received_at
        }
        (Some(_), None) => true,
        _ => false,
    }
}

fn manual_pricing_backup_path(path: &Path, index: usize) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("{}.bak{index}", path.to_string_lossy()))
}

fn manual_pricing_temp_path(path: &Path) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("{}.tmp", path.to_string_lossy()))
}

fn manual_bill_store_temp_path(path: &Path) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("{}.tmp", path.to_string_lossy()))
}

fn rotate_manual_pricing_backups(path: &Path) -> Result<(), String> {
    for index in (1..=MANUAL_PRICING_MAX_BACKUPS).rev() {
        if index == MANUAL_PRICING_MAX_BACKUPS {
            let target = manual_pricing_backup_path(path, index);
            if target.is_file() {
                fs::remove_file(&target)
                    .map_err(|error| format!("Failed to remove {}: {error}", target.display()))?;
            }
            continue;
        }

        let source = manual_pricing_backup_path(path, index);
        let target = manual_pricing_backup_path(path, index + 1);
        if source.is_file() {
            fs::rename(&source, &target).map_err(|error| {
                format!(
                    "Failed to rotate {} to {}: {error}",
                    source.display(),
                    target.display()
                )
            })?;
        }
    }

    if path.is_file() {
        let backup = manual_pricing_backup_path(path, 1);
        fs::rename(path, &backup).map_err(|error| {
            format!(
                "Failed to move {} to {}: {error}",
                path.display(),
                backup.display()
            )
        })?;
    }

    Ok(())
}

fn write_manual_pricing_workspace(
    path: &Path,
    workspace: &ManualPricingWorkspace,
) -> Result<(), String> {
    let contents =
        to_string_pretty(workspace, PrettyConfig::new()).map_err(|error| error.to_string())?;
    let temp_path = manual_pricing_temp_path(path);

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to prepare {}: {error}", parent.display()))?;
    }

    if path.exists() && !path.is_file() {
        return Err(format!("{} is not a file.", path.display()));
    }

    fs::write(&temp_path, contents)
        .map_err(|error| format!("Failed to write {}: {error}", temp_path.display()))?;

    if let Err(error) = rotate_manual_pricing_backups(path) {
        let backup = manual_pricing_backup_path(path, 1);
        if !path.exists() && backup.is_file() {
            let _ = fs::rename(&backup, path);
        }
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    if let Err(error) = fs::rename(&temp_path, path) {
        let backup = manual_pricing_backup_path(path, 1);
        if !path.exists() && backup.is_file() {
            let _ = fs::rename(&backup, path);
        }
        let _ = fs::remove_file(&temp_path);
        return Err(format!(
            "Failed to finalize save to {}: {error}",
            path.display()
        ));
    }

    Ok(())
}

fn write_manual_bill_store(path: &Path, store: &ManualBillStore) -> Result<(), String> {
    let contents =
        to_string_pretty(store, PrettyConfig::new()).map_err(|error| error.to_string())?;
    let temp_path = manual_bill_store_temp_path(path);

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to prepare {}: {error}", parent.display()))?;
    }

    if path.exists() && !path.is_file() {
        return Err(format!("{} is not a file.", path.display()));
    }

    fs::write(&temp_path, contents)
        .map_err(|error| format!("Failed to write {}: {error}", temp_path.display()))?;
    if path.is_file() {
        fs::remove_file(path)
            .map_err(|error| format!("Failed to replace {}: {error}", path.display()))?;
    }
    fs::rename(&temp_path, path)
        .map_err(|error| format!("Failed to finalize {}: {error}", path.display()))?;

    Ok(())
}

#[derive(Clone)]
enum ManualBillRevision {
    Bill(ManualPricingBill),
    Tombstone(ManualPricingBillTombstone),
}

impl ManualBillRevision {
    fn timestamp(&self) -> u64 {
        match self {
            Self::Bill(bill) => bill.updated_at_millis,
            Self::Tombstone(tombstone) => tombstone.deleted_at_millis,
        }
    }
}

fn prefer_manual_bill_revision(
    candidate: &ManualBillRevision,
    current: &ManualBillRevision,
) -> bool {
    candidate.timestamp() > current.timestamp()
        || (candidate.timestamp() == current.timestamp()
            && matches!(candidate, ManualBillRevision::Tombstone(_))
            && matches!(current, ManualBillRevision::Bill(_)))
}

fn canonical_manual_bill_store(
    bills: Vec<ManualPricingBill>,
    tombstones: Vec<ManualPricingBillTombstone>,
) -> ManualBillStore {
    let mut latest: HashMap<String, ManualBillRevision> = HashMap::new();

    for mut bill in bills {
        bill.normalize();
        if bill.id.trim().is_empty() {
            continue;
        }

        let key = bill.id.trim().to_string();
        bill.id = key.clone();
        let revision = ManualBillRevision::Bill(bill);
        let replace = latest
            .get(&key)
            .is_none_or(|current| prefer_manual_bill_revision(&revision, current));
        if replace {
            latest.insert(key, revision);
        }
    }

    for mut tombstone in tombstones {
        tombstone.normalize();
        if tombstone.id.is_empty() {
            continue;
        }

        let key = tombstone.id.clone();
        let revision = ManualBillRevision::Tombstone(tombstone);
        let replace = latest
            .get(&key)
            .is_none_or(|current| prefer_manual_bill_revision(&revision, current));
        if replace {
            latest.insert(key, revision);
        }
    }

    let mut bills = Vec::new();
    let mut bill_tombstones = Vec::new();
    for revision in latest.into_values() {
        match revision {
            ManualBillRevision::Bill(bill) => bills.push(bill),
            ManualBillRevision::Tombstone(tombstone) => bill_tombstones.push(tombstone),
        }
    }

    bills.sort_by(|left, right| {
        right
            .updated_at_millis
            .cmp(&left.updated_at_millis)
            .then_with(|| left.id.cmp(&right.id))
    });
    bill_tombstones.sort_by(|left, right| {
        right
            .deleted_at_millis
            .cmp(&left.deleted_at_millis)
            .then_with(|| left.id.cmp(&right.id))
    });

    ManualBillStore {
        bills,
        bill_tombstones,
    }
}

impl PrintCountApp {
    fn default_manual_pricing_path(&self) -> String {
        Path::new(&self.data_root)
            .join("manual_pricing.ron")
            .to_string_lossy()
            .to_string()
    }

    fn default_printers_path(&self) -> String {
        Path::new(&self.data_root)
            .join("printers.ron")
            .to_string_lossy()
            .to_string()
    }

    fn default_counter_oids_path(&self) -> String {
        Path::new(&self.data_root)
            .join("counter_oids.ron")
            .to_string_lossy()
            .to_string()
    }

    fn default_statistics_path(&self) -> String {
        Path::new(&self.data_root)
            .join("statistics.ron")
            .to_string_lossy()
            .to_string()
    }

    fn load_printers_if_present(&mut self) {
        let path = self.printers_path.trim();
        if path.is_empty() || !Path::new(path).is_file() {
            self.printers_status = None;
            return;
        }

        self.load_printers_from_path();
    }

    fn load_manual_pricing_if_present(&mut self) {
        let path = self.manual_pricing_path.trim();
        if path.is_empty() || !Path::new(path).is_file() {
            self.manual_pricing_status = None;
            return;
        }

        self.load_manual_pricing_from_path();
    }

    fn load_manual_bill_store_if_present(&mut self) {
        let path = self.manual_bill_store_path.trim();
        if path.is_empty() || !Path::new(path).is_file() {
            return;
        }

        match fs::read_to_string(path) {
            Ok(contents) => match parse_manual_bill_store_contents(&contents) {
                Ok(store) => {
                    self.manual_bills.extend(store.bills);
                    self.manual_bill_tombstones.extend(store.bill_tombstones);
                    self.normalize_manual_bills();
                    self.sync_selected_manual_bill();
                    self.manual_bills_dirty = true;
                }
                Err(error) => tracing::warn!(
                    target: targets::STORAGE,
                    "Failed to load manual bill store from {}: {}",
                    path,
                    error
                ),
            },
            Err(error) => tracing::warn!(
                target: targets::STORAGE,
                "Failed to read manual bill store from {}: {}",
                path,
                error
            ),
        }
    }

    fn load_statistics_if_present(&mut self) {
        let path = self.statistics_path.trim();
        if path.is_empty() || !Path::new(path).is_file() {
            self.statistics_path = self.default_statistics_path();
            return;
        }

        match load_statistics_store(Path::new(path)) {
            Ok(store) => {
                self.statistics_store = store;
                self.statistics_revision = self.statistics_revision.saturating_add(1);
            }
            Err(error) => {
                tracing::warn!(
                    target: targets::STORAGE,
                    "Failed to load statistics store from {}: {}",
                    path,
                    error
                );
            }
        }
    }

    fn load_manual_pricing_from_path(&mut self) {
        let path = self.manual_pricing_path.trim().to_string();
        if path.is_empty() {
            self.manual_pricing_status = Some("Load failed: path is empty.".to_string());
            return;
        }

        match fs::read_to_string(&path) {
            Ok(contents) => match parse_manual_pricing_contents(&contents) {
                Ok(workspace) => {
                    let mut settings = workspace.settings;
                    settings.reset_calculator_state();
                    self.manual_pricing = settings;
                    self.manual_bills = workspace.bills;
                    self.manual_bill_tombstones = workspace.bill_tombstones;
                    self.last_manual_pricing_sync_id = manual_pricing_version_id(Path::new(&path))
                        .or_else(|| Some(current_pricing_sync_id()));
                    self.normalize_manual_bills();
                    self.sync_selected_manual_bill();
                    self.manual_bills_dirty = true;
                    self.manual_pricing_status =
                        Some(format!("Loaded manual pricing from {path}."));
                }
                Err(error) => {
                    self.manual_pricing_status = Some(format!("Load failed: {error}"));
                }
            },
            Err(error) => {
                self.manual_pricing_status = Some(format!("Load failed: {error}"));
            }
        }
    }

    fn synced_pricing_settings(&self) -> PricingSettings {
        let mut pricing = self.pricing.clone();
        pricing.manual_pricing = self.manual_pricing.clone();
        pricing
    }

    fn current_manual_pricing_workspace(&mut self) -> ManualPricingWorkspace {
        self.manual_pricing.normalize();
        self.normalize_manual_bills();
        ManualPricingWorkspace {
            settings: self.manual_pricing.clone(),
            bills: self.manual_bills.clone(),
            bill_tombstones: self.manual_bill_tombstones.clone(),
        }
    }

    fn current_manual_bill_store(&mut self) -> ManualBillStore {
        self.normalize_manual_bills();
        ManualBillStore {
            bills: self.manual_bills.clone(),
            bill_tombstones: self.manual_bill_tombstones.clone(),
        }
    }

    fn persist_manual_pricing_workspace(
        &self,
        workspace: &ManualPricingWorkspace,
    ) -> Result<String, String> {
        let path = self.manual_pricing_path.trim().to_string();
        if path.is_empty() {
            return Err("path is empty.".to_string());
        }

        write_manual_pricing_workspace(Path::new(&path), workspace)?;
        Ok(path)
    }

    fn persist_manual_bill_store(&mut self) -> Result<String, String> {
        let path = self.manual_bill_store_path.trim().to_string();
        if path.is_empty() {
            return Err("bill store path is empty.".to_string());
        }

        let store = self.current_manual_bill_store();
        write_manual_bill_store(Path::new(&path), &store)?;
        Ok(path)
    }

    fn persist_manual_bill_store_if_dirty(&mut self) {
        if !self.manual_bills_dirty {
            return;
        }

        match self.persist_manual_bill_store() {
            Ok(_) => {
                self.manual_bills_dirty = false;
            }
            Err(error) => tracing::warn!(
                target: targets::STORAGE,
                "Failed to persist manual bill store: {}",
                error
            ),
        }
    }

    fn persist_statistics_store(&self) -> Result<(), String> {
        let path = self.statistics_path.trim().to_string();
        if path.is_empty() {
            return Err("statistics path is empty.".to_string());
        }

        write_statistics_store(Path::new(&path), &self.statistics_store)
    }

    fn persist_statistics_store_with_logging(&self) {
        if let Err(error) = self.persist_statistics_store() {
            tracing::warn!(
                target: targets::STORAGE,
                "Failed to persist statistics store to {}: {}",
                self.statistics_path,
                error
            );
        }
    }

    fn mark_statistics_changed(&mut self) {
        self.statistics_revision = self.statistics_revision.saturating_add(1);
        self.sync_statistics_visible_series();
        self.persist_statistics_store_with_logging();
        self.send_statistics_state();
        self.queue_statistics_cleanup();
    }

    fn remove_non_initial_zero_statistics_entries(&mut self) {
        let removed = remove_non_initial_zero_poll_metrics(&mut self.statistics_store);
        if removed == 0 {
            tracing::info!(
                target: targets::STORAGE,
                "Statistics zero cleanup found no removable entries."
            );
            return;
        }

        tracing::info!(
            target: targets::STORAGE,
            "Removed {} non-initial zero statistics entries.",
            removed
        );
        self.mark_statistics_changed();
    }

    fn repair_statistics_duplicate_series(&mut self) {
        let before = self.statistics_store.clone();
        normalize_statistics_store(&mut self.statistics_store);
        if self.statistics_store == before {
            tracing::info!(
                target: targets::STORAGE,
                "Statistics duplicate-series repair found no changes."
            );
            return;
        }

        tracing::info!(
            target: targets::STORAGE,
            "Repaired statistics duplicate series labels and keys."
        );
        self.mark_statistics_changed();
    }

    fn queue_statistics_cleanup(&mut self) {
        if self.statistics_store.is_empty() {
            return;
        }

        if self.statistics_cleanup_in_flight {
            self.statistics_cleanup_pending_revision = Some(self.statistics_revision);
            return;
        }

        self.statistics_cleanup_receiver = Some(spawn_cleanup_worker(
            self.statistics_store.clone(),
            self.statistics_revision,
            now_epoch_seconds(),
        ));
        self.statistics_cleanup_in_flight = true;
        self.statistics_cleanup_pending_revision = None;
    }

    fn poll_statistics_cleanup(&mut self) {
        let Some(receiver) = self.statistics_cleanup_receiver.as_ref() else {
            return;
        };

        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.statistics_cleanup_receiver = None;
                self.statistics_cleanup_in_flight = false;
                if self.statistics_cleanup_pending_revision.take().is_some() {
                    self.queue_statistics_cleanup();
                }
                return;
            }
        };

        self.statistics_cleanup_receiver = None;
        self.statistics_cleanup_in_flight = false;

        if result.revision == self.statistics_revision && result.store != self.statistics_store {
            self.statistics_store = result.store;
            self.persist_statistics_store_with_logging();
        }

        if self.statistics_cleanup_pending_revision.take().is_some() {
            self.queue_statistics_cleanup();
        }
    }

    fn save_manual_pricing_to_path(&mut self) {
        let workspace = self.current_manual_pricing_workspace();

        match self.persist_manual_pricing_workspace(&workspace) {
            Ok(path) => {
                self.last_manual_pricing_sync_id = manual_pricing_version_id(Path::new(&path))
                    .or_else(|| Some(current_pricing_sync_id()));
                self.manual_pricing_status = Some(format!("Saved manual pricing to {path}."));
            }
            Err(error) => {
                self.manual_pricing_status = Some(format!("Save failed: {error}"));
            }
        }
    }

    fn sync_prices_to_network(&mut self) {
        let workspace = self.current_manual_pricing_workspace();
        let path = match self.persist_manual_pricing_workspace(&workspace) {
            Ok(path) => path,
            Err(error) => {
                self.manual_pricing_status = Some(format!("Sync failed: {error}"));
                return;
            }
        };

        let sync_id = current_pricing_sync_id();
        let payload = sync::PricingSyncPayload {
            id: sync_id.clone(),
            pricing: self.synced_pricing_settings(),
            workspace,
        };
        self.last_manual_pricing_sync_id = Some(sync_id);

        let synced = self
            .sync_sender
            .as_ref()
            .is_some_and(|sender| sender.send(SyncCommand::SyncPrices(payload)).is_ok());

        self.manual_pricing_status = Some(if synced {
            format!("Saved manual pricing to {path} and synced prices across the network.")
        } else {
            format!("Saved manual pricing to {path}. Sync unavailable.")
        });
    }

    fn apply_statistics_sync(&mut self, payload: sync::StatisticsSyncPayload) {
        if payload.store.is_empty() {
            return;
        }

        let prefer_incoming =
            payload.latest_data_at > statistics_store_latest_timestamp(&self.statistics_store);
        let merge =
            merge_statistics_store(&mut self.statistics_store, &payload.store, prefer_incoming);
        if !merge.changed {
            return;
        }

        self.statistics_revision = self.statistics_revision.saturating_add(1);
        self.sync_statistics_visible_series();
        self.persist_statistics_store_with_logging();
        self.queue_statistics_cleanup();

        if merge.differs_from_incoming {
            self.send_statistics_state();
        }
    }

    fn apply_pricing_sync(&mut self, payload: sync::PricingSyncPayload) {
        if pricing_sync_is_stale(&payload.id, self.last_manual_pricing_sync_id.as_deref()) {
            tracing::info!(
                target: "sync",
                "Ignoring stale pricing sync {} because local manual pricing is newer.",
                payload.id
            );
            return;
        }

        let sync::PricingSyncPayload {
            id,
            pricing,
            mut workspace,
        } = payload;
        let incoming_bills = workspace.bills.clone();
        let incoming_tombstones = workspace.bill_tombstones.clone();
        let merged_store = canonical_manual_bill_store(
            self.manual_bills
                .iter()
                .cloned()
                .chain(workspace.bills.into_iter())
                .collect(),
            self.manual_bill_tombstones
                .iter()
                .cloned()
                .chain(workspace.bill_tombstones.into_iter())
                .collect(),
        );
        workspace.bills = merged_store.bills.clone();
        workspace.bill_tombstones = merged_store.bill_tombstones.clone();
        self.last_manual_pricing_sync_id = Some(id);
        self.pricing = pricing;
        workspace.settings.reset_calculator_state();
        self.manual_pricing = workspace.settings.clone();
        self.manual_bills = merged_store.bills;
        self.manual_bill_tombstones = merged_store.bill_tombstones;
        self.normalize_manual_bills();
        self.sync_selected_manual_bill();
        self.manual_bills_dirty = true;

        self.manual_pricing_status =
            Some(match self.persist_manual_pricing_workspace(&workspace) {
                Ok(path) => format!("Applied synced prices and saved manual pricing to {path}."),
                Err(error) => format!("Applied synced prices, but save failed: {error}"),
            });

        let mut applied_snapshot = self.build_shared_state(self.last_shared_state.revision);
        applied_snapshot.manual_bills = incoming_bills;
        applied_snapshot.manual_bill_tombstones = incoming_tombstones;
        self.last_shared_state = applied_snapshot;
    }

    fn active_manual_pricing(&self) -> &ManualPricingSettings {
        self.selected_manual_bill()
            .map(|bill| &bill.pricing)
            .unwrap_or(&self.manual_pricing)
    }

    fn active_manual_pricing_mut(&mut self) -> &mut ManualPricingSettings {
        if let Some(bill_id) = self.selected_manual_bill_id.clone()
            && let Some(index) = self.manual_bills.iter().position(|bill| bill.id == bill_id)
        {
            self.manual_bills[index].touch();
            self.manual_bills_dirty = true;
            return &mut self.manual_bills[index].pricing;
        }

        &mut self.manual_pricing
    }

    fn selected_manual_bill(&self) -> Option<&ManualPricingBill> {
        let selected_id = self.selected_manual_bill_id.as_deref()?;
        self.manual_bills.iter().find(|bill| bill.id == selected_id)
    }

    fn sync_selected_manual_bill(&mut self) {
        if self
            .selected_manual_bill_id
            .as_deref()
            .is_some_and(|selected_id| !self.manual_bills.iter().any(|bill| bill.id == selected_id))
        {
            self.selected_manual_bill_id = None;
        }
    }

    fn normalize_manual_bills(&mut self) {
        let mut seen_ids = HashSet::new();

        for index in 0..self.manual_bills.len() {
            self.manual_bills[index].normalize();
            if self.manual_bills[index].id.trim().is_empty() {
                let (generated_id, generated_subject) = self.next_manual_bill_name(&seen_ids);
                self.manual_bills[index].id = generated_id;
                if self.manual_bills[index].subject.trim().is_empty() {
                    self.manual_bills[index].subject = generated_subject;
                }
            }

            self.manual_bills[index].id = self.manual_bills[index].id.trim().to_string();
            seen_ids.insert(self.manual_bills[index].id.clone());
        }

        let store = canonical_manual_bill_store(
            std::mem::take(&mut self.manual_bills),
            std::mem::take(&mut self.manual_bill_tombstones),
        );
        self.manual_bills = store.bills;
        self.manual_bill_tombstones = store.bill_tombstones;
    }

    fn next_manual_bill_name(&self, reserved_ids: &HashSet<String>) -> (String, String) {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let adjective_count = MANUAL_BILL_ADJECTIVES.len() as u128;
        let subject_count = MANUAL_BILL_SUBJECTS.len() as u128;
        let combinations = adjective_count.saturating_mul(subject_count);

        for attempt in 0..combinations.max(1) {
            let offset = attempt.saturating_mul(131);
            let composite = seed.wrapping_add(offset) % combinations.max(1);
            let adjective = MANUAL_BILL_ADJECTIVES[(composite % adjective_count.max(1)) as usize];
            let subject = MANUAL_BILL_SUBJECTS
                [((composite / adjective_count.max(1)) % subject_count.max(1)) as usize];
            let candidate_id = format!("{adjective}-{subject}");

            let already_used = reserved_ids.contains(&candidate_id)
                || self.manual_bills.iter().any(|bill| bill.id == candidate_id);
            if !already_used {
                return (
                    candidate_id,
                    format!(
                        "{} {}",
                        title_case_word(adjective),
                        title_case_word(subject)
                    ),
                );
            }
        }

        let fallback = format!(
            "{}-{}-{seed:x}",
            MANUAL_BILL_ADJECTIVES[0], MANUAL_BILL_SUBJECTS[0]
        );
        (
            fallback,
            format!(
                "{} {} {seed:x}",
                title_case_word(MANUAL_BILL_ADJECTIVES[0]),
                title_case_word(MANUAL_BILL_SUBJECTS[0]),
            ),
        )
    }

    fn save_manual_pricing_as_bill(&mut self) {
        self.manual_pricing.normalize();
        let saved_pricing = self.manual_pricing.clone();

        let (id, subject) = self.next_manual_bill_name(&HashSet::new());
        self.manual_bills.insert(
            0,
            ManualPricingBill {
                id: id.clone(),
                subject,
                pricing: saved_pricing,
                updated_at_millis: 0,
            },
        );
        if let Some(saved_bill) = self.manual_bills.first_mut() {
            saved_bill.touch();
        }
        self.manual_bill_tombstones
            .retain(|tombstone| tombstone.id != id);
        self.manual_bills_dirty = true;
        self.manual_pricing.reset_calculator_state();
        self.manual_pricing_selected = true;
        self.selected_manual_bill_id = None;
        self.manual_pricing_tab = ManualPricingTab::Calculator;
        self.manual_pricing_status = Some(format!("Saved bill {id} and cleared calculator."));
    }

    fn delete_selected_manual_pricing_bill(&mut self) {
        let Some(selected_id) = self.selected_manual_bill_id.clone() else {
            return;
        };

        let Some(index) = self
            .manual_bills
            .iter()
            .position(|bill| bill.id == selected_id)
        else {
            self.selected_manual_bill_id = None;
            return;
        };

        let deleted_id = self.manual_bills[index].id.clone();
        self.manual_bills.remove(index);
        self.manual_bill_tombstones
            .push(ManualPricingBillTombstone::new(deleted_id.clone()));
        self.normalize_manual_bills();
        self.manual_bills_dirty = true;
        self.selected_manual_bill_id = None;
        self.manual_pricing_selected = true;
        self.manual_pricing_status = Some(format!("Deleted bill {deleted_id}."));
    }

    fn ensure_statistics_selection(&mut self) {
        self.statistics_selected_printers
            .retain(|printer_id| self.printers.iter().any(|record| &record.id == printer_id));

        if !self.statistics_selected_printers.is_empty() {
            return;
        }

        let fallback = self
            .selected_printer
            .clone()
            .filter(|printer_id| self.printers.iter().any(|record| &record.id == printer_id))
            .or_else(|| self.printers.first().map(|record| record.id.clone()));

        if let Some(printer_id) = fallback {
            self.statistics_selected_printers.insert(printer_id);
        }
    }

    fn toggle_statistics_printer(&mut self, printer_id: PrinterId) {
        if !self.statistics_selected_printers.insert(printer_id.clone()) {
            self.statistics_selected_printers.remove(&printer_id);
        }
        self.statistics_selected_printers
            .retain(|candidate| self.printers.iter().any(|record| &record.id == candidate));
        self.sync_statistics_visible_series();
    }

    fn sync_statistics_visible_series(&mut self) {
        let available = available_series(
            &self.statistics_store,
            &self.statistics_selected_printers,
            &self.pricing,
            Some(self.statistics_time_window()),
        );

        if self.statistics_series_selection_initialized {
            return;
        }

        if !self.statistics_visible_series.is_empty() {
            self.statistics_series_selection_initialized = true;
            return;
        }

        if available.is_empty() {
            return;
        }

        let preferred_labels = [
            "Total B/W",
            "Total Color",
            "Copies B/W",
            "Prints B/W",
            "Copies Color",
            "Prints Color",
        ];
        let mut inserted = 0usize;

        for preferred_label in preferred_labels {
            if let Some(series) = available
                .iter()
                .find(|series| series.label == preferred_label)
            {
                self.statistics_visible_series.insert(series.key.clone());
                inserted += 1;
            }
            if inserted >= 4 {
                break;
            }
        }

        if self.statistics_visible_series.is_empty() {
            self.statistics_visible_series
                .extend(available.iter().take(3).map(|series| series.key.clone()));
        }

        self.statistics_series_selection_initialized = true;
    }

    fn toggle_statistics_series(&mut self, series_key: String) {
        self.statistics_series_selection_initialized = true;
        if !self.statistics_visible_series.insert(series_key.clone()) {
            self.statistics_visible_series.remove(&series_key);
        }
    }

    fn select_statistics_range_preset(&mut self, preset: StatisticsRangePreset) {
        self.statistics_range_preset = preset;
        if preset == StatisticsRangePreset::Custom {
            self.normalize_statistics_custom_range();
        }
        self.sync_statistics_visible_series();
    }

    fn set_statistics_date_year(&mut self, target: StatisticsDateTarget, year: i32) {
        let current = self.statistics_custom_date(target);
        self.set_statistics_custom_date(
            target,
            statistics_date_from_components(year, current.month(), current.day()),
        );
    }

    fn set_statistics_date_month(&mut self, target: StatisticsDateTarget, month: Month) {
        let current = self.statistics_custom_date(target);
        self.set_statistics_custom_date(
            target,
            statistics_date_from_components(current.year(), month, current.day()),
        );
    }

    fn set_statistics_date_day(&mut self, target: StatisticsDateTarget, day: u8) {
        let current = self.statistics_custom_date(target);
        self.set_statistics_custom_date(
            target,
            statistics_date_from_components(current.year(), current.month(), day),
        );
    }

    fn set_statistics_date_today(&mut self, target: StatisticsDateTarget) {
        self.set_statistics_custom_date(target, self.statistics_today());
    }

    fn set_statistics_custom_date(&mut self, target: StatisticsDateTarget, date: Date) {
        let today = self.statistics_today();
        let date = statistics_clamp_date(date, today);
        self.statistics_range_preset = StatisticsRangePreset::Custom;

        match target {
            StatisticsDateTarget::Start => {
                self.statistics_custom_start = date;
                if self.statistics_custom_end < date {
                    self.statistics_custom_end = date;
                }
            }
            StatisticsDateTarget::End => {
                self.statistics_custom_end = date;
                if self.statistics_custom_start > date {
                    self.statistics_custom_start = date;
                }
            }
        }

        self.normalize_statistics_custom_range();
        self.sync_statistics_visible_series();
    }

    fn normalize_statistics_custom_range(&mut self) {
        let today = self.statistics_today();
        self.statistics_custom_start = statistics_clamp_date(self.statistics_custom_start, today);
        self.statistics_custom_end = statistics_clamp_date(self.statistics_custom_end, today);

        if self.statistics_custom_start > self.statistics_custom_end {
            std::mem::swap(
                &mut self.statistics_custom_start,
                &mut self.statistics_custom_end,
            );
        }
    }

    fn statistics_today(&self) -> Date {
        statistics_today_date(now_epoch_seconds())
    }

    fn statistics_custom_date(&self, target: StatisticsDateTarget) -> Date {
        match target {
            StatisticsDateTarget::Start => self.statistics_custom_start,
            StatisticsDateTarget::End => self.statistics_custom_end,
        }
    }

    fn statistics_selected_date_range(&self) -> (Date, Date) {
        let today = self.statistics_today();
        match self.statistics_range_preset {
            StatisticsRangePreset::Custom => (
                statistics_clamp_date(self.statistics_custom_start, today)
                    .min(statistics_clamp_date(self.statistics_custom_end, today)),
                statistics_clamp_date(self.statistics_custom_start, today)
                    .max(statistics_clamp_date(self.statistics_custom_end, today)),
            ),
            preset => statistics_date_for_preset(preset, today),
        }
    }

    fn statistics_time_window(&self) -> StatisticsTimeWindow {
        let (start_date, end_date) = self.statistics_selected_date_range();
        statistics_time_window_for_dates(start_date, end_date, now_epoch_seconds())
    }

    fn refresh_logs(&mut self) {
        let entries = self.log_store.snapshot();
        for entry in &entries {
            if self.known_targets.insert(entry.target.clone()) {
                self.enabled_targets.insert(entry.target.clone());
            }
        }
        self.log_entries = entries;
    }

    fn sorted_targets(&self) -> Vec<String> {
        let mut targets: Vec<String> = self.known_targets.iter().cloned().collect();
        targets.sort();
        targets
    }

    fn visible_entries(&self) -> Vec<&LogEntry> {
        self.log_entries
            .iter()
            .filter(|entry| self.enabled_targets.contains(&entry.target))
            .collect()
    }

    fn copy_diagnostics(&self) -> String {
        let text = self.diagnostics_text();
        match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text)) {
            Ok(()) => {
                tracing::info!(target: targets::UI, "Diagnostics copied to clipboard");
                "Copied".to_string()
            }
            Err(error) => {
                tracing::warn!(target: targets::UI, "Clipboard copy failed: {}", error);
                format!("Failed: {error}")
            }
        }
    }

    fn diagnostics_text(&self) -> String {
        let mut output = String::new();
        output.push_str("PrintCount diagnostics\n");
        output.push_str(&format!("Log level: {}\n", self.log_level));
        output.push_str(&format!("Sync role: {:?}\n", self.sync_role));
        output.push_str(&format!("Sync status: {}\n", self.sync_status_detail));
        if let Some(selected) = &self.selected_printer {
            output.push_str(&format!("Selected printer: {}\n", selected));
        }
        output.push_str(&format!(
            "Targets enabled: {}\n",
            self.sorted_targets()
                .into_iter()
                .filter(|target| self.enabled_targets.contains(target))
                .collect::<Vec<String>>()
                .join(", ")
        ));
        output.push_str("Per-printer errors: none recorded yet\n");
        output.push_str("SNMP OIDs used: not captured yet\n");
        output.push_str("Persistence diagnostics: not captured yet\n");
        output.push_str("Recent logs:\n");

        let entries = self.visible_entries();
        let start = entries.len().saturating_sub(50);
        for entry in entries.into_iter().skip(start) {
            output.push_str(&entry.format_line());
            output.push('\n');
        }

        output
    }

    fn start_discovery(&mut self) -> Command<Message> {
        let cidr = self.discovery_cidr.trim();
        if cidr.is_empty() {
            self.discovery_status = Some("CIDR is empty.".to_string());
            return Command::none();
        }

        let range = match CidrRange::parse(cidr) {
            Ok(range) => range,
            Err(error) => {
                self.discovery_status = Some(format!("Invalid CIDR: {error}"));
                return Command::none();
            }
        };

        let mut queue = VecDeque::new();
        for ip in range.iter() {
            let host = ip.to_string();
            queue.push_back(self.discovery_address_for_host(&host));
        }

        if queue.is_empty() {
            self.discovery_status = Some("CIDR contains no usable hosts.".to_string());
            return Command::none();
        }

        self.discovery_run_id = self.discovery_run_id.wrapping_add(1);
        self.discovery_active = true;
        self.discovery_queue = queue;
        self.discovery_total = self.discovery_queue.len();
        self.discovery_scanned = 0;
        self.discovery_found = 0;
        self.discovery_errors = 0;
        self.discovery_in_flight = 0;
        self.discovery_status = Some(format!(
            "Discovery started ({} hosts).",
            self.discovery_total
        ));

        self.spawn_discovery_tasks()
    }

    fn stop_discovery(&mut self) {
        self.discovery_active = false;
        self.discovery_queue.clear();
        self.discovery_in_flight = 0;
        self.discovery_run_id = self.discovery_run_id.wrapping_add(1);
        self.discovery_status = Some("Discovery stopped.".to_string());
    }

    fn handle_discovery_result(&mut self, result: DiscoveryProbeResult) -> Command<Message> {
        if result.run_id != self.discovery_run_id {
            return Command::none();
        }

        self.discovery_in_flight = self.discovery_in_flight.saturating_sub(1);
        self.discovery_scanned = self.discovery_scanned.saturating_add(1);

        match result.outcome {
            DiscoveryOutcome::Printer(record) => {
                self.discovery_found = self.discovery_found.saturating_add(1);
                self.upsert_printer(record);
            }
            DiscoveryOutcome::NotPrinter => {}
            DiscoveryOutcome::Error(error) => {
                self.discovery_errors = self.discovery_errors.saturating_add(1);
                self.discovery_status =
                    Some(format!("Last error: {} ({})", error.summary, error.detail));
            }
        }

        if self.discovery_queue.is_empty() && self.discovery_in_flight == 0 {
            self.discovery_active = false;
            self.discovery_status = Some(format!(
                "Discovery complete: {} printers found.",
                self.discovery_found
            ));
            return Command::none();
        }

        self.spawn_discovery_tasks()
    }

    fn spawn_discovery_tasks(&mut self) -> Command<Message> {
        if !self.discovery_active {
            return Command::none();
        }

        let mut commands = Vec::new();
        while self.discovery_in_flight < DISCOVERY_CONCURRENCY {
            let Some(address) = self.discovery_queue.pop_front() else {
                break;
            };

            let run_id = self.discovery_run_id;
            let community = self.discovery_community.trim().to_string();
            let community = (!community.is_empty()).then_some(community);
            let config = self.snmp_config.clone();

            self.discovery_in_flight += 1;
            commands.push(Command::perform(
                async move {
                    let result = probe_printer(address, community, config).await;
                    let outcome = match result {
                        Ok(Some(record)) => DiscoveryOutcome::Printer(record),
                        Ok(None) => DiscoveryOutcome::NotPrinter,
                        Err(error) => DiscoveryOutcome::Error(SnmpErrorInfo::from_error(error)),
                    };
                    DiscoveryProbeResult { run_id, outcome }
                },
                Message::DiscoveryProbeFinished,
            ));
        }

        Command::batch(commands)
    }

    fn upsert_printer(&mut self, record: PrinterRecord) {
        let host = record.snmp_address.as_ref().map(|addr| addr.host.as_str());

        let existing = host.and_then(|host| {
            self.printers
                .iter_mut()
                .find(|printer| Self::printer_matches_host(printer, host))
        });

        if let Some(existing) = existing {
            existing.ip_or_hostname = record.ip_or_hostname;
            existing.model = record.model;
            existing.sys_object_id = record.sys_object_id;
            existing.snmp_address = record.snmp_address;
            existing.community = record.community;
            existing.status = record.status;
            existing.last_seen = record.last_seen;
        } else {
            self.poll_states
                .insert(record.id.clone(), SnmpPollStatus::Idle);
            self.printers.push(record);
        }
    }

    fn start_printer_reorder_drag(&mut self, printer_id: PrinterId) {
        self.pending_printer_drag = Some(PendingPrinterReorderDrag {
            printer_id,
            pressed_at: Instant::now(),
        });
        self.active_printer_drag = None;
    }

    fn activate_printer_reorder_drag_if_ready(&mut self) {
        if self.active_printer_drag.is_some() {
            return;
        }

        let Some(pending) = self.pending_printer_drag.clone() else {
            return;
        };

        if pending.pressed_at.elapsed() < PRINTER_REORDER_HOLD_DURATION {
            return;
        }

        let Some(source_index) = self
            .printers
            .iter()
            .position(|record| record.id == pending.printer_id)
        else {
            self.pending_printer_drag = None;
            return;
        };

        self.active_printer_drag = Some(PrinterReorderDrag {
            printer_id: pending.printer_id,
            drop_index: source_index,
        });
        self.pending_printer_drag = None;
    }

    fn complete_printer_card_press(&mut self, printer_id: PrinterId) -> Command<Message> {
        if self.active_printer_drag.is_some() {
            return Command::none();
        }

        let Some(pending) = self.pending_printer_drag.as_ref() else {
            return Command::none();
        };

        if pending.printer_id != printer_id {
            return Command::none();
        }

        self.pending_printer_drag = None;
        self.manual_pricing_selected = false;
        self.selected_manual_bill_id = None;
        self.selected_printer = Some(printer_id.clone());
        self.apply_profile_for_printer(&printer_id, None);
        self.poll_selected_printer()
    }

    fn hover_printer_reorder_drop(&mut self, drop_index: usize) {
        let Some(drag) = self.active_printer_drag.as_mut() else {
            return;
        };

        drag.drop_index = drop_index.min(self.printers.len());
    }

    fn finish_printer_reorder_drag(&mut self) -> bool {
        let Some(drag) = self.active_printer_drag.take() else {
            return false;
        };

        let Some(source_index) = self
            .printers
            .iter()
            .position(|record| record.id == drag.printer_id)
        else {
            return false;
        };

        let mut target_index = drag.drop_index.min(self.printers.len());
        if source_index < target_index {
            target_index = target_index.saturating_sub(1);
        }

        if target_index == source_index {
            return false;
        }

        let record = self.printers.remove(source_index);
        self.printers.insert(target_index, record);

        let synced = self.sync_sender.is_some();
        self.printers_status = Some(if synced {
            "Reordered printers and synced the list across the network. Use Export to save it to disk.".to_string()
        } else {
            "Reordered printers. Sync unavailable; use Export to save it to disk.".to_string()
        });

        true
    }

    fn cancel_printer_reorder_drag(&mut self) {
        self.pending_printer_drag = None;
        self.active_printer_drag = None;
    }

    fn cancel_pending_printer_reorder(&mut self, printer_id: &PrinterId) {
        if self.active_printer_drag.is_some() {
            return;
        }

        if self
            .pending_printer_drag
            .as_ref()
            .is_some_and(|pending| &pending.printer_id == printer_id)
        {
            self.pending_printer_drag = None;
        }
    }

    fn delete_selected_printer(&mut self) {
        if self.active_tab != Tab::Printers || self.manual_pricing_selected {
            return;
        }

        let Some(selected) = self.selected_printer.clone() else {
            return;
        };

        let Some(index) = self
            .printers
            .iter()
            .position(|record| record.id == selected)
        else {
            self.selected_printer = None;
            return;
        };

        self.printers.remove(index);
        self.poll_states.remove(&selected);
        self.poll_in_flight.remove(&selected);
        self.recording_sessions.remove(&selected);
        self.statistics_selected_printers.remove(&selected);

        if self.printers.is_empty() {
            self.selected_printer = None;
            return;
        }

        let new_index = index.min(self.printers.len() - 1);
        self.selected_printer = Some(self.printers[new_index].id.clone());
        self.ensure_statistics_selection();
        self.sync_statistics_visible_series();
    }

    fn printer_matches_host(printer: &PrinterRecord, host: &str) -> bool {
        printer.snmp_address.as_ref().map(|addr| addr.host.as_str()) == Some(host)
            || printer.ip_or_hostname.as_deref() == Some(host)
    }

    fn find_printer_by_host(&self, host: &str) -> Option<&PrinterRecord> {
        self.printers
            .iter()
            .find(|printer| Self::printer_matches_host(printer, host))
    }

    fn discovery_address_for_host(&self, host: &str) -> SnmpAddress {
        let port = self
            .find_printer_by_host(host)
            .and_then(|printer| printer.snmp_address.as_ref())
            .map(|address| address.port)
            .unwrap_or(DEFAULT_SNMP_PORT);

        SnmpAddress::new(host.to_string(), port)
    }

    fn find_printer_by_host_mut(&mut self, host: &str) -> Option<&mut PrinterRecord> {
        self.printers
            .iter_mut()
            .find(|printer| Self::printer_matches_host(printer, host))
    }

    fn add_manual_printer(&mut self) {
        let name = self.manual_name.trim().to_string();
        let host = self.manual_host.trim().to_string();
        let port_text = self.manual_port.trim().to_string();
        let community = self.manual_community.trim().to_string();

        if host.is_empty() {
            self.manual_status = Some("Add failed: host is empty.".to_string());
            return;
        }

        let port = if port_text.is_empty() {
            DEFAULT_SNMP_PORT
        } else {
            match port_text.parse::<u16>() {
                Ok(port) => port,
                Err(_) => {
                    self.manual_status = Some("Add failed: invalid port.".to_string());
                    return;
                }
            }
        };

        let now = now_epoch_seconds();
        if let Some(existing) = self.find_printer_by_host_mut(&host) {
            if !name.is_empty() {
                existing.model = Some(name);
            }
            existing.ip_or_hostname = Some(host.clone());
            existing.snmp_address = Some(SnmpAddress::new(host.clone(), port));
            if !community.is_empty() {
                existing.community = Some(community);
            }
            existing.last_seen = Some(now);
            self.manual_status = Some(format!("Updated printer {host}."));
            return;
        }

        let mut record = PrinterRecord::new(PrinterId::new(format!("manual-{host}")));
        record.ip_or_hostname = Some(host.clone());
        record.model = (!name.is_empty()).then_some(name);
        record.snmp_address = Some(SnmpAddress::new(host.clone(), port));
        record.community = (!community.is_empty()).then_some(community);
        record.last_seen = Some(now);

        self.poll_states
            .insert(record.id.clone(), SnmpPollStatus::Idle);
        self.printers.push(record);
        self.manual_name.clear();
        self.manual_host.clear();
        self.manual_status = Some(format!("Added printer {host}."));
    }

    fn apply_printer_name_fallback(
        &mut self,
        printer_id: &PrinterId,
        name: String,
        allow_override: bool,
        sys_descr: Option<&str>,
    ) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }

        let Some(record) = self
            .printers
            .iter_mut()
            .find(|record| &record.id == printer_id)
        else {
            return;
        };

        let existing = record.model.as_deref().map(str::trim).unwrap_or("");
        let is_manual = record.id.0.starts_with("manual-");

        if existing.is_empty() {
            record.model = Some(name.to_string());
            return;
        }

        if is_manual {
            return;
        }

        if !allow_override {
            return;
        }

        let mut should_replace = false;
        if let Some(sys_descr) = sys_descr.map(str::trim)
            && !sys_descr.is_empty()
            && existing == sys_descr
        {
            should_replace = true;
        }
        if let Some(host) = record.ip_or_hostname.as_deref().map(str::trim)
            && !host.is_empty()
            && existing == host
        {
            should_replace = true;
        }

        if should_replace && existing != name {
            record.model = Some(name.to_string());
        }
    }

    fn load_printers_from_path(&mut self) {
        let path = self.printers_path.trim().to_string();
        if path.is_empty() {
            self.printers_status = Some("Load failed: path is empty.".to_string());
            return;
        }

        match fs::read_to_string(&path) {
            Ok(contents) => match from_str::<Vec<PrinterRecord>>(&contents) {
                Ok(printers) => {
                    let count = printers.len();
                    self.replace_printers(printers);
                    self.printers_status = Some(format!("Loaded {count} printers from {path}."));
                }
                Err(error) => {
                    self.printers_status = Some(format!("Load failed: {error}"));
                }
            },
            Err(error) => {
                self.printers_status = Some(format!("Load failed: {error}"));
            }
        }
    }

    fn save_printers_to_path(&mut self) {
        let path = self.printers_path.trim().to_string();
        if path.is_empty() {
            self.printers_status = Some("Save failed: path is empty.".to_string());
            return;
        }

        let config = PrettyConfig::new();
        match to_string_pretty(&self.printers, config) {
            Ok(contents) => match fs::write(&path, contents) {
                Ok(()) => {
                    self.printers_status =
                        Some(format!("Saved {} printers to {path}.", self.printers.len()));
                }
                Err(error) => {
                    self.printers_status = Some(format!("Save failed: {error}"));
                }
            },
            Err(error) => {
                self.printers_status = Some(format!("Save failed: {error}"));
            }
        }
    }

    fn replace_printers(&mut self, printers: Vec<PrinterRecord>) {
        let selected = self.selected_printer.clone();
        self.pending_printer_drag = None;
        self.active_printer_drag = None;
        self.printers = printers;
        self.poll_states.clear();
        self.poll_in_flight.clear();
        self.recording_sessions
            .retain(|id, _| self.printers.iter().any(|record| &record.id == id));

        for record in &self.printers {
            self.poll_states
                .insert(record.id.clone(), SnmpPollStatus::Idle);
        }

        if let Some(selected) = selected {
            if self.printers.iter().any(|record| record.id == selected) {
                self.selected_printer = Some(selected);
            } else {
                self.selected_printer = None;
            }
        }
        self.statistics_selected_printers
            .retain(|printer_id| self.printers.iter().any(|record| &record.id == printer_id));
        self.ensure_statistics_selection();
        self.sync_statistics_visible_series();

        if let Some(selected) = self.selected_printer.clone() {
            self.apply_profile_for_printer(&selected, None);
        }
    }

    fn handle_snmp_polled(
        &mut self,
        printer_id: PrinterId,
        result: Result<SnmpResponse, SnmpErrorInfo>,
    ) {
        self.poll_in_flight.remove(&printer_id);
        let received_at = now_epoch_seconds();
        let mut poll_name = None;
        let mut allow_override = false;
        let mut sys_descr = None;
        let mut sys_object_id = None;

        let (state, status, last_seen) = match result {
            Ok(response) => {
                let printer_name = varbind_text_value(
                    &response.varbinds,
                    &Oid::from_slice(&PRT_GENERAL_PRINTER_NAME_OID),
                );
                let sys_name =
                    varbind_text_value(&response.varbinds, &Oid::from_slice(&SYS_NAME_OID));
                sys_descr =
                    varbind_text_value(&response.varbinds, &Oid::from_slice(&SYS_DESCR_OID));
                sys_object_id =
                    varbind_text_value(&response.varbinds, &Oid::from_slice(&SYS_OBJECT_ID_OID));
                allow_override =
                    printer_name.is_some() || sys_name.is_some() || sys_descr.is_some();
                poll_name = printer_name.or(sys_name).or_else(|| sys_descr.clone());
                (
                    SnmpPollStatus::Ok {
                        received_at,
                        varbinds: response.varbinds,
                    },
                    PrinterStatus::Online,
                    Some(received_at),
                )
            }
            Err(error) => (
                SnmpPollStatus::Error {
                    received_at,
                    summary: error.summary,
                    detail: error.detail,
                },
                error.status,
                None,
            ),
        };

        if let Some(name) = poll_name {
            self.apply_printer_name_fallback(
                &printer_id,
                name,
                allow_override,
                sys_descr.as_deref(),
            );
        }

        if let Some(record) = self
            .printers
            .iter_mut()
            .find(|record| record.id == printer_id)
        {
            record.sys_object_id = sys_object_id;
            record.sys_descr = sys_descr.clone();
            record.status = status;
            if let Some(last_seen) = last_seen {
                record.last_seen = Some(last_seen);
            }
        }

        let printer_id_clone = printer_id.clone();
        self.poll_states.insert(printer_id, state);
        self.sync_statistics_from_poll_state(&printer_id_clone);
        if self.selected_printer.as_ref() == Some(&printer_id_clone) {
            let needs_profile = self
                .printers
                .iter()
                .find(|record| record.id == printer_id_clone)
                .and_then(|record| record.profile_id.as_ref())
                .is_none();
            if needs_profile {
                self.apply_profile_for_printer(&printer_id_clone, sys_descr.as_deref());
            }
        }
    }

    fn sync_statistics_from_poll_state(&mut self, printer_id: &PrinterId) {
        let Some((received_at, metrics)) = self.statistics_poll_metrics_for_printer(printer_id)
        else {
            return;
        };

        if append_poll_sample(&mut self.statistics_store, printer_id, received_at, metrics) {
            self.mark_statistics_changed();
        }
    }

    fn statistics_poll_metrics_for_printer(
        &self,
        printer_id: &PrinterId,
    ) -> Option<(u64, Vec<StatisticsPollMetric>)> {
        let SnmpPollStatus::Ok {
            received_at,
            varbinds,
        } = self.poll_states.get(printer_id)?
        else {
            return None;
        };

        let (counter_oids, recording_settings, profile) =
            if self.selected_printer.as_ref() == Some(printer_id) {
                (
                    self.counter_oids.clone(),
                    self.recording_oids.clone(),
                    self.active_profile.clone(),
                )
            } else if let Some(profile) = self.profile_for_poll(printer_id) {
                (
                    profile.counters.clone(),
                    recording_settings_from_profile(&profile.recording),
                    Some(profile),
                )
            } else {
                (default_counter_oids(), default_recording_oid_inputs(), None)
            };
        let label_map = build_poll_label_map(&counter_oids, &recording_settings, profile.as_ref());
        let metrics = varbinds
            .iter()
            .filter_map(|varbind| {
                let value = varbind.value.as_u64()?;
                let label = label_map
                    .get(&varbind.oid)
                    .cloned()
                    .unwrap_or_else(|| varbind.oid.to_string());
                Some(StatisticsPollMetric::new(
                    varbind.oid.to_string(),
                    label,
                    value,
                ))
            })
            .collect::<Vec<_>>();

        (!metrics.is_empty()).then_some((*received_at, metrics))
    }

    fn schedule_statistics_polls(&mut self) -> Command<Message> {
        if self.sync_role == SyncRole::Client {
            return Command::none();
        }

        let now = now_epoch_seconds();
        let due_printers: Vec<_> = self
            .printers
            .iter()
            .filter(|record| !self.poll_in_flight.contains(&record.id))
            .filter(|record| {
                let last_sample_at = self
                    .statistics_store
                    .entry(&record.id)
                    .and_then(|entry| entry.poll_samples.last())
                    .map(|sample| sample.captured_at);
                statistics_poll_due(last_sample_at, now)
            })
            .map(|record| record.id.clone())
            .collect();

        if due_printers.is_empty() {
            return Command::none();
        }

        Command::batch(
            due_printers
                .into_iter()
                .map(|printer_id| self.poll_printer(printer_id))
                .collect::<Vec<_>>(),
        )
    }

    fn poll_selected_printer(&mut self) -> Command<Message> {
        if self.manual_pricing_selected {
            return Command::none();
        }

        let Some(printer_id) = self.selected_printer.clone() else {
            return Command::none();
        };

        if self.sync_role == SyncRole::Client {
            if !self.recent_poll_is_fresh(&printer_id) {
                self.request_remote_poll(&printer_id);
            }
            return Command::none();
        }

        self.poll_printer(printer_id)
    }

    fn poll_printer(&mut self, printer_id: PrinterId) -> Command<Message> {
        if self.poll_in_flight.contains(&printer_id) {
            return Command::none();
        }

        let Some(record) = self.printers.iter().find(|record| record.id == printer_id) else {
            return Command::none();
        };

        let now = now_epoch_seconds();
        let Some(address) = record.snmp_address.clone() else {
            self.poll_states.insert(
                printer_id.clone(),
                SnmpPollStatus::Error {
                    received_at: now,
                    summary: "Missing SNMP address".to_string(),
                    detail: "Printer has no SNMP address configured.".to_string(),
                },
            );
            if let Some(record) = self
                .printers
                .iter_mut()
                .find(|record| record.id == printer_id)
            {
                record.status = PrinterStatus::Error;
            }
            return Command::none();
        };

        let using_selected_context = self.selected_printer.as_ref() == Some(&printer_id);
        let poll_profile = if using_selected_context {
            self.active_profile.clone()
        } else {
            self.profile_for_poll(&printer_id)
        };
        let (counter_oids, recording_settings) = if using_selected_context {
            (self.counter_oids.clone(), self.recording_oids.clone())
        } else if let Some(profile) = poll_profile.as_ref() {
            (
                profile.counters.clone(),
                recording_settings_from_profile(&profile.recording),
            )
        } else {
            (default_counter_oids(), default_recording_oid_inputs())
        };
        let default_toner = default_toner_oids();
        let mut extra_poll = Vec::new();
        if let Some(profile) = poll_profile.as_ref() {
            extra_poll.extend(
                profile
                    .extra_poll_labels
                    .iter()
                    .map(|entry| entry.oid.clone()),
            );
            if profile.counter_table.as_deref() == Some("ricoh-m184") {
                extra_poll.extend(
                    RICOH_COUNTER_TABLE
                        .iter()
                        .map(|entry| ricoh_counter_oid(entry.type_id)),
                );
            }
        }
        let toner_oids = poll_profile
            .as_ref()
            .map(|profile| &profile.toner)
            .unwrap_or(&default_toner);
        let recording_oids = recording_profile_from_settings_lossy(&recording_settings);

        let mut request = SnmpRequest::new(
            address,
            snmp_oids(
                &counter_oids,
                &recording_oids,
                extra_poll.as_slice(),
                toner_oids,
            ),
        );
        if let Some(community) = record.community.clone() {
            request = request.with_community(community);
        }

        let config = self.snmp_config.clone();
        let printer_id = printer_id.clone();

        self.poll_in_flight.insert(printer_id.clone());
        self.poll_states
            .entry(printer_id.clone())
            .or_insert(SnmpPollStatus::Idle);

        Command::perform(
            async move {
                let client = SnmpV2cClient::new(config);
                match client.get(request).await {
                    Ok(response) => Ok(response),
                    Err(error) => Err(SnmpErrorInfo::from_error(error)),
                }
            },
            move |result| Message::SnmpPolled { printer_id, result },
        )
    }

    fn profile_for_poll(&self, printer_id: &PrinterId) -> Option<ManufacturerProfile> {
        let record = self
            .printers
            .iter()
            .find(|record| &record.id == printer_id)?;
        let profile_id = record.profile_id.clone().or_else(|| {
            self.profile_index.match_profile_id(
                record.sys_object_id.as_deref(),
                record.sys_descr.as_deref(),
                record.model.as_deref(),
            )
        })?;

        self.profile_index.profile(&profile_id).cloned()
    }

    fn recent_poll_is_fresh(&self, printer_id: &PrinterId) -> bool {
        let Some(received_at) = self.poll_states.get(printer_id).and_then(poll_received_at) else {
            return false;
        };

        now_epoch_seconds().saturating_sub(received_at) <= 3
    }

    fn request_remote_poll(&self, printer_id: &PrinterId) {
        let Some(sender) = self.sync_sender.as_ref() else {
            return;
        };

        let _ = sender.send(SyncCommand::RequestPoll(printer_id.clone()));
    }

    fn ready_recording_snapshot(
        &self,
        printer_id: &PrinterId,
    ) -> Result<RecordingSnapshot, String> {
        let snapshot = self.snapshot_for_printer(printer_id)?;
        let recording_oids = recording_profile_from_settings_lossy(&self.recording_oids);
        let missing = missing_recording_snapshot_categories(&snapshot, &recording_oids);

        if missing.is_empty() {
            Ok(snapshot)
        } else {
            Err(format!(
                "Waiting for poll data for {}.",
                format_recording_category_list(&missing)
            ))
        }
    }

    fn start_recording(&mut self) {
        let Some(printer_id) = self.selected_printer.clone() else {
            return;
        };

        let already_active = self
            .recording_sessions
            .get(&printer_id)
            .map(|session| session.active)
            .unwrap_or(false);
        if already_active {
            let session = self
                .recording_sessions
                .entry(printer_id.clone())
                .or_default();
            session.status = Some("Start ignored: recording already active.".to_string());
            session.touch();
            return;
        }

        let snapshot_result = self.ready_recording_snapshot(&printer_id);
        let session = self
            .recording_sessions
            .entry(printer_id.clone())
            .or_default();

        match snapshot_result {
            Ok(snapshot) => {
                session.active = true;
                session.start = Some(snapshot.clone());
                session.end = None;
                session.end_fields_unlocked = false;
                session.edits.apply_start_snapshot(&snapshot);
                session.status = None;
                session.mark_manual_state_change();
            }
            Err(error) => {
                session.status = Some(format!("Start failed: {error}"));
                session.touch();
            }
        }
    }

    fn stop_recording(&mut self) {
        let Some(printer_id) = self.selected_printer.clone() else {
            return;
        };

        let is_active = self
            .recording_sessions
            .get(&printer_id)
            .map(|session| session.active)
            .unwrap_or(false);
        if !is_active {
            let session = self
                .recording_sessions
                .entry(printer_id.clone())
                .or_default();
            session.status = Some("Stop failed: no active recording.".to_string());
            session.touch();
            return;
        }

        let snapshot_result = self.snapshot_for_printer(&printer_id);
        let session = self
            .recording_sessions
            .entry(printer_id.clone())
            .or_default();

        match snapshot_result {
            Ok(snapshot) => {
                session.active = false;
                session.end = Some(snapshot.clone());
                session.edits.apply_end_snapshot(&snapshot);
                session.status = None;
                session.mark_manual_state_change();
            }
            Err(error) => {
                session.status = Some(format!("Stop failed: {error}"));
                session.touch();
            }
        }
    }

    fn reset_recording_end_to_polled(&mut self, category: RecordingCategory) {
        let Some(printer_id) = self.selected_printer.clone() else {
            return;
        };

        let live_snapshot = self
            .recording_sessions
            .get(&printer_id)
            .filter(|session| session.active)
            .and_then(|_| self.snapshot_for_printer(&printer_id).ok());

        let session = self.recording_sessions.entry(printer_id).or_default();
        let polled_value = session
            .end
            .as_ref()
            .and_then(|snapshot| snapshot_category_value(snapshot, category))
            .or_else(|| {
                live_snapshot
                    .as_ref()
                    .and_then(|snapshot| snapshot_category_value(snapshot, category))
            });

        session.edits.category_mut(category).end_input = polled_value
            .map(|value| value.to_string())
            .unwrap_or_default();
        session.touch();
    }

    fn export_poll_data(&mut self) {
        let path = self.poll_export_path.trim().to_string();
        if path.is_empty() {
            self.poll_export_status = Some("Export failed: path is empty.".to_string());
            return;
        }

        let Some(printer_id) = self.selected_printer.clone() else {
            self.poll_export_status = Some("Export failed: select a printer first.".to_string());
            return;
        };

        let Some(state) = self.poll_states.get(&printer_id) else {
            self.poll_export_status = Some("Export failed: no poll data yet.".to_string());
            return;
        };

        let SnmpPollStatus::Ok {
            received_at,
            varbinds,
        } = state
        else {
            self.poll_export_status = Some("Export failed: no poll data yet.".to_string());
            return;
        };

        let (name, address) = match self.printers.iter().find(|record| record.id == printer_id) {
            Some(record) => {
                let name = record
                    .model
                    .as_deref()
                    .unwrap_or("Unknown name")
                    .to_string();
                let address = record
                    .snmp_address
                    .as_ref()
                    .map(|addr| addr.to_string())
                    .or_else(|| record.ip_or_hostname.clone())
                    .unwrap_or_else(|| "Not set".to_string());
                (name, address)
            }
            None => ("Unknown name".to_string(), "Not set".to_string()),
        };

        let mut contents = String::new();
        let mut push_line = |line: &str| {
            contents.push_str(line);
            contents.push('\n');
        };

        push_line("PrintCountPay poll export");
        push_line(&format!("printer_id={printer_id}"));
        push_line(&format!("name={name}"));
        push_line(&format!("address={address}"));
        push_line(&format!("received_at={received_at}"));
        push_line("");

        if varbinds.is_empty() {
            push_line("No varbinds returned.");
        } else {
            for varbind in varbinds {
                push_line(&format!("{} = {}", varbind.oid, varbind.value));
            }
        }

        match fs::write(&path, contents) {
            Ok(()) => {
                self.poll_export_status = Some(format!("Exported poll data to {path}."));
            }
            Err(error) => {
                self.poll_export_status = Some(format!("Export failed: {error}"));
            }
        }
    }

    fn snapshot_for_printer(&self, printer_id: &PrinterId) -> Result<RecordingSnapshot, String> {
        let Some(state) = self.poll_states.get(printer_id) else {
            return Err("No poll data yet.".to_string());
        };

        match state {
            SnmpPollStatus::Ok {
                received_at,
                varbinds,
            } => Ok(self.build_recording_snapshot(*received_at, varbinds)),
            SnmpPollStatus::Error {
                summary, detail, ..
            } => Err(format!("{summary} ({detail})")),
            SnmpPollStatus::Idle => Err("No poll data yet.".to_string()),
        }
    }

    fn build_recording_snapshot(
        &self,
        received_at: u64,
        varbinds: &[SnmpVarBind],
    ) -> RecordingSnapshot {
        let recording_oids = recording_profile_from_settings_lossy(&self.recording_oids);

        let copies_bw_value = recording_oids
            .copies_bw
            .iter()
            .find_map(|oid| varbind_numeric_value(varbinds, oid));
        let copies_color_value = recording_oids
            .copies_color
            .iter()
            .find_map(|oid| varbind_numeric_value(varbinds, oid));
        let prints_bw_value = recording_oids
            .prints_bw
            .iter()
            .find_map(|oid| varbind_numeric_value(varbinds, oid));
        let prints_color_value = recording_oids
            .prints_color
            .iter()
            .find_map(|oid| varbind_numeric_value(varbinds, oid));

        RecordingSnapshot {
            received_at,
            bw_printer: prints_bw_value,
            bw_copier: copies_bw_value,
            color_printer: prints_color_value,
            color_copier: copies_color_value,
        }
    }

    fn apply_profile_for_printer(
        &mut self,
        printer_id: &PrinterId,
        sys_descr_override: Option<&str>,
    ) {
        let (sys_object_id, sys_descr, model, mut profile_id) = {
            let record_snapshot = self.printers.iter().find(|record| &record.id == printer_id);
            let Some(record_snapshot) = record_snapshot else {
                return;
            };

            (
                record_snapshot.sys_object_id.clone(),
                record_snapshot.sys_descr.clone(),
                record_snapshot.model.clone(),
                record_snapshot.profile_id.clone(),
            )
        };
        let sys_descr = sys_descr.as_deref().or(sys_descr_override);
        if let Some(ref id) = profile_id
            && self.profile_index.profile(id).is_none()
            && let Some(migrated) = self.migrate_profile_id(id)
        {
            profile_id = Some(migrated.clone());
            if let Some(record) = self
                .printers
                .iter_mut()
                .find(|record| &record.id == printer_id)
            {
                record.profile_id = Some(migrated);
            }
        }

        if profile_id.is_none() {
            profile_id = self.profile_index.match_profile_id(
                sys_object_id.as_deref(),
                sys_descr,
                model.as_deref(),
            );
            if let Some(ref id) = profile_id
                && let Some(record) = self
                    .printers
                    .iter_mut()
                    .find(|record| &record.id == printer_id)
            {
                record.profile_id = Some(id.clone());
            }
        }

        let Some(profile_id) = profile_id else {
            if self.selected_printer.as_ref() == Some(printer_id) {
                self.clear_active_profile();
            }
            return;
        };

        if self.active_profile.as_ref().map(|profile| profile.id()) == Some(profile_id.clone()) {
            return;
        }

        let Some(profile) = self.profile_index.profile(&profile_id).cloned() else {
            if self.selected_printer.as_ref() == Some(printer_id) {
                self.clear_active_profile();
                self.oids_status = Some(format!("Profile {profile_id} not found."));
            }
            return;
        };

        if self.selected_printer.as_ref() == Some(printer_id) {
            self.apply_active_profile(profile);
        }
    }

    fn migrate_profile_id(&self, profile_id: &str) -> Option<String> {
        self.profile_index.migrate_profile_id(profile_id)
    }

    fn apply_active_profile(&mut self, profile: ManufacturerProfile) {
        self.recording_oids = recording_settings_from_profile(&profile.recording);
        self.counter_oids = profile.counters.clone();
        self.oids_total_text = format_oid_list(&self.counter_oids.total);
        self.oids_path = profile_path(Path::new(&self.profiles_root), &profile)
            .to_string_lossy()
            .to_string();
        self.active_profile = Some(profile);
    }

    fn clear_active_profile(&mut self) {
        self.active_profile = None;
        self.counter_oids = default_counter_oids();
        self.recording_oids = default_recording_oid_inputs();
        self.oids_total_text = format_oid_list(&self.counter_oids.total);
        self.oids_path = self.default_counter_oids_path();
    }

    fn sync_active_profile_from_inputs(&mut self) -> Result<(), String> {
        let updated = {
            let Some(profile) = self.active_profile.as_mut() else {
                return Err("No active profile selected.".to_string());
            };

            let recording = recording_profile_from_settings(&self.recording_oids)?;
            profile.recording = recording;
            profile.counters = self.counter_oids.clone();
            profile.clone()
        };
        self.profile_index.upsert_profile(updated);
        Ok(())
    }

    fn sync_oid_inputs(&mut self) {
        self.recording_oids = recording_oids_from_counter_set(&self.counter_oids);
        self.oids_total_text = format_oid_list(&self.counter_oids.total);
    }

    fn apply_oid_inputs(&mut self) {
        match self.parse_oid_inputs() {
            Ok(set) => {
                self.counter_oids = set;
                if let Err(error) = self.sync_active_profile_from_inputs() {
                    self.oids_status =
                        Some(format!("Applied mapping (profile not synced: {error})"));
                } else {
                    self.oids_status = Some("Applied OID mapping.".to_string());
                }
            }
            Err(error) => {
                self.oids_status = Some(format!("Apply failed: {error}"));
            }
        }
    }

    fn parse_oid_inputs(&self) -> Result<CounterOidSet, String> {
        let copies_bw = parse_oid_list(&self.recording_oids.copies_bw_input)
            .map_err(|error| format!("Copies B/W OIDs: {error}"))?;
        let prints_bw = parse_oid_list(&self.recording_oids.prints_bw_input)
            .map_err(|error| format!("Prints B/W OIDs: {error}"))?;
        let copies_color = parse_oid_list(&self.recording_oids.copies_color_input)
            .map_err(|error| format!("Copies color OIDs: {error}"))?;
        let prints_color = parse_oid_list(&self.recording_oids.prints_color_input)
            .map_err(|error| format!("Prints color OIDs: {error}"))?;
        let total = parse_oid_list(&self.oids_total_text)
            .map_err(|error| format!("Total OIDs: {error}"))?;

        let mut bw = copies_bw;
        bw.extend(prints_bw);
        let mut color = copies_color;
        color.extend(prints_color);

        Ok(CounterOidSet { bw, color, total })
    }

    fn load_oids_from_path(&mut self) {
        let path = self.oids_path.trim().to_string();
        if path.is_empty() {
            self.oids_status = Some("Load failed: path is empty.".to_string());
            return;
        }

        match fs::read_to_string(&path) {
            Ok(contents) => {
                if let Ok(mut profile) = from_str::<ManufacturerProfile>(&contents) {
                    profile.source_path = Some(Path::new(&path).to_path_buf());
                    let id = profile.id();
                    self.profile_index.upsert_profile(profile.clone());
                    self.apply_active_profile(profile);
                    self.oids_status = Some(format!("Loaded profile {id} from {path}."));
                    return;
                }
                match from_str::<CounterOidSet>(&contents) {
                    Ok(set) => {
                        self.counter_oids = set;
                        self.sync_oid_inputs();
                        self.oids_status = Some(format!("Loaded OIDs from {path}."));
                    }
                    Err(error) => {
                        self.oids_status = Some(format!("Load failed: {error}"));
                    }
                }
            }
            Err(error) => {
                self.oids_status = Some(format!("Load failed: {error}"));
            }
        }
    }

    fn save_oids_to_path(&mut self) {
        let path = self.oids_path.trim().to_string();
        if path.is_empty() {
            self.oids_status = Some("Save failed: path is empty.".to_string());
            return;
        }

        let config = PrettyConfig::new();
        if let Err(error) = self.sync_active_profile_from_inputs() {
            self.oids_status = Some(format!("Save failed: {error}"));
            return;
        }

        if let Some(profile) = self.active_profile.clone() {
            match to_string_pretty(&profile, config) {
                Ok(contents) => match fs::write(&path, contents) {
                    Ok(()) => {
                        self.oids_status = Some(format!("Saved profile to {path}."));
                    }
                    Err(error) => {
                        self.oids_status = Some(format!("Save failed: {error}"));
                    }
                },
                Err(error) => {
                    self.oids_status = Some(format!("Save failed: {error}"));
                }
            }
        } else {
            self.oids_status = Some("Save failed: no active profile.".to_string());
        }
    }

    fn crawl_oids(&mut self) -> Command<Message> {
        if self.oids_crawl_in_flight {
            return Command::none();
        }

        let Some(printer_id) = self.selected_printer.clone() else {
            self.oids_status = Some("Crawl failed: select a printer first.".to_string());
            return Command::none();
        };

        let Some(record) = self.printers.iter().find(|record| record.id == printer_id) else {
            self.oids_status = Some("Crawl failed: selected printer missing.".to_string());
            return Command::none();
        };

        let Some(address) = record.snmp_address.clone() else {
            self.oids_status = Some("Crawl failed: printer has no SNMP address.".to_string());
            return Command::none();
        };

        let community = record.community.clone();
        let config = self.snmp_config.clone();
        self.oids_crawl_in_flight = true;
        self.oids_status = Some("Crawling printer/vendor MIBs...".to_string());

        Command::perform(
            async move {
                let client = SnmpV2cClient::new(config);
                let mut varbinds = Vec::new();
                let mut last_error = None;

                for root in CRAWL_ROOTS {
                    let mut request = SnmpWalkRequest::new(address.clone(), Oid::from_slice(root))
                        .with_max_results(0);
                    if let Some(ref community) = community {
                        request = request.with_community(community.clone());
                    }

                    match client.walk(request).await {
                        Ok(response) => varbinds.extend(response.varbinds),
                        Err(error) => {
                            last_error = Some(SnmpErrorInfo::from_error(error));
                        }
                    }
                }

                if varbinds.is_empty() {
                    Err(last_error.unwrap_or_else(|| {
                        SnmpErrorInfo::new(
                            PrinterStatus::Error,
                            "Crawl failed.",
                            "No OIDs returned from crawl.",
                        )
                    }))
                } else {
                    Ok(counter_oids_from_walk(&varbinds))
                }
            },
            Message::OidsCrawled,
        )
    }

    fn handle_sync_event(&mut self, event: SyncEvent) -> Command<Message> {
        match event {
            SyncEvent::Ready(sender) => {
                self.sync_sender = Some(sender);
                self.last_shared_state = self.build_shared_state(self.last_shared_state.revision);
                self.send_shared_state(self.last_shared_state.clone());
                self.send_statistics_state();
                Command::none()
            }
            SyncEvent::StatusChanged(status) => {
                self.sync_role = status.role;
                self.sync_status_detail = status.detail.clone();
                tracing::info!(target: "sync", "{}", status.detail);
                Command::none()
            }
            SyncEvent::SnapshotReceived(snapshot) => {
                if snapshot.revision < self.last_shared_state.revision {
                    return Command::none();
                }
                self.apply_shared_state(snapshot);
                Command::none()
            }
            SyncEvent::PollRequested(printer_id) => {
                if self.sync_role == SyncRole::Master && !self.recent_poll_is_fresh(&printer_id) {
                    self.poll_printer(printer_id)
                } else {
                    Command::none()
                }
            }
            SyncEvent::PricingSyncReceived(payload) => {
                self.apply_pricing_sync(payload);
                Command::none()
            }
            SyncEvent::StatisticsSyncReceived(payload) => {
                self.apply_statistics_sync(payload);
                Command::none()
            }
        }
    }

    fn flush_shared_state(&mut self) {
        let snapshot = self.build_shared_state(self.last_shared_state.revision);
        if snapshot == self.last_shared_state {
            return;
        }

        let next = self.build_shared_state(self.last_shared_state.revision.saturating_add(1));
        self.last_shared_state = next.clone();
        self.send_shared_state(next);
    }

    fn send_shared_state(&self, snapshot: SharedState) {
        let Some(sender) = self.sync_sender.as_ref() else {
            return;
        };

        let _ = sender.send(SyncCommand::SetSnapshot(snapshot));
    }

    fn send_statistics_state(&self) {
        if self.statistics_store.is_empty() {
            return;
        }

        let Some(sender) = self.sync_sender.as_ref() else {
            return;
        };

        let payload = sync::StatisticsSyncPayload {
            latest_data_at: statistics_store_latest_timestamp(&self.statistics_store),
            store: self.statistics_store.clone(),
        };
        let _ = sender.send(SyncCommand::SyncStatistics(payload));
    }

    fn build_shared_state(&self, revision: u64) -> SharedState {
        let mut poll_states: Vec<_> = self
            .poll_states
            .iter()
            .map(|(printer_id, state)| sync::PollStateEntry {
                printer_id: printer_id.clone(),
                state: state.clone(),
            })
            .collect();
        poll_states.sort_by(|left, right| left.printer_id.0.cmp(&right.printer_id.0));

        let mut recording_sessions: Vec<_> = self
            .recording_sessions
            .iter()
            .map(|(printer_id, session)| sync::RecordingSessionEntry {
                printer_id: printer_id.clone(),
                session: session.clone(),
            })
            .collect();
        recording_sessions.sort_by(|left, right| left.printer_id.0.cmp(&right.printer_id.0));

        SharedState {
            revision,
            printers: self.printers.clone(),
            poll_states,
            recording_sessions,
            pricing: self.pricing.clone(),
            bill_sync_supported: true,
            manual_bills: self.manual_bills.clone(),
            manual_bill_tombstones: self.manual_bill_tombstones.clone(),
        }
    }

    fn apply_shared_state(&mut self, snapshot: SharedState) {
        let incoming_snapshot = snapshot.clone();
        let selected = self.selected_printer.clone();
        let selected_manual_bill_id = self.selected_manual_bill_id.clone();
        let pending_printer_drag = self.pending_printer_drag.clone();
        let active_printer_drag = self.active_printer_drag.clone();
        let local_poll_states = self.poll_states.clone();
        let SharedState {
            revision,
            printers,
            poll_states,
            recording_sessions,
            pricing,
            bill_sync_supported,
            manual_bills,
            manual_bill_tombstones,
        } = snapshot;
        let incoming_manual_bills = bill_sync_supported.then_some(manual_bills.clone());
        let incoming_manual_bill_tombstones =
            bill_sync_supported.then_some(manual_bill_tombstones.clone());

        self.printers = printers;
        self.pricing = pricing;
        if bill_sync_supported {
            self.manual_bills.extend(manual_bills);
            self.manual_bill_tombstones.extend(manual_bill_tombstones);
            self.normalize_manual_bills();
            self.manual_bills_dirty = true;
        }
        let known_ids: HashSet<PrinterId> = self
            .printers
            .iter()
            .map(|record| record.id.clone())
            .collect();
        let incoming_poll_states: HashMap<_, _> = poll_states
            .into_iter()
            .filter(|entry| known_ids.contains(&entry.printer_id))
            .map(|entry| (entry.printer_id, entry.state))
            .collect();
        self.poll_states = known_ids
            .iter()
            .map(|printer_id| {
                let local_state = local_poll_states.get(printer_id);
                let incoming_state = incoming_poll_states.get(printer_id);
                let state = match (local_state, incoming_state) {
                    (Some(local), Some(incoming)) if prefer_local_poll_state(local, incoming) => {
                        local.clone()
                    }
                    (_, Some(incoming)) => incoming.clone(),
                    (Some(local), None) if poll_received_at(local).is_some() => local.clone(),
                    _ => SnmpPollStatus::Idle,
                };
                (printer_id.clone(), state)
            })
            .collect();
        self.pending_printer_drag =
            pending_printer_drag.filter(|pending| known_ids.contains(&pending.printer_id));
        self.active_printer_drag = active_printer_drag
            .filter(|drag| known_ids.contains(&drag.printer_id))
            .map(|mut drag| {
                drag.drop_index = drag.drop_index.min(self.printers.len());
                drag
            });
        let local_recording_sessions = std::mem::take(&mut self.recording_sessions);
        let incoming_recording_sessions: HashMap<_, _> = recording_sessions
            .into_iter()
            .filter(|entry| known_ids.contains(&entry.printer_id))
            .map(|entry| (entry.printer_id, entry.session))
            .collect();
        self.recording_sessions = known_ids
            .iter()
            .filter_map(|printer_id| {
                let local_session = local_recording_sessions.get(printer_id);
                let incoming_session = incoming_recording_sessions.get(printer_id);
                let local_unlock_state = local_session
                    .map(|session| session.end_fields_unlocked)
                    .or_else(|| incoming_session.map(|session| session.end_fields_unlocked))
                    .unwrap_or(false);

                let mut session = match (local_session, incoming_session) {
                    (Some(local), Some(incoming))
                        if prefer_local_recording_session(local, Some(incoming)) =>
                    {
                        local.clone()
                    }
                    (Some(local), None) if prefer_local_recording_session(local, None) => {
                        local.clone()
                    }
                    (_, Some(incoming)) => incoming.clone(),
                    (Some(local), None) => local.clone(),
                    (None, None) => return None,
                };
                session.end_fields_unlocked = local_unlock_state;
                Some((printer_id.clone(), session))
            })
            .collect();
        self.poll_in_flight
            .retain(|printer_id| known_ids.contains(printer_id));
        self.statistics_selected_printers
            .retain(|printer_id| known_ids.contains(printer_id));

        self.selected_printer = selected.filter(|printer_id| known_ids.contains(printer_id));
        if bill_sync_supported {
            self.selected_manual_bill_id = selected_manual_bill_id;
            self.sync_selected_manual_bill();
        }
        for printer_id in &known_ids {
            self.sync_statistics_from_poll_state(printer_id);
        }
        self.ensure_statistics_selection();
        self.sync_statistics_visible_series();
        if let Some(selected) = self.selected_printer.clone() {
            self.apply_profile_for_printer(&selected, None);
        } else {
            self.clear_active_profile();
        }

        let mut applied_snapshot = self.build_shared_state(revision);
        if let (Some(incoming_manual_bills), Some(incoming_manual_bill_tombstones)) = (
            incoming_manual_bills,
            incoming_manual_bill_tombstones,
        ) {
            applied_snapshot.manual_bills = incoming_manual_bills;
            applied_snapshot.manual_bill_tombstones = incoming_manual_bill_tombstones;
        }

        self.last_shared_state = if applied_snapshot == incoming_snapshot {
            applied_snapshot
        } else {
            incoming_snapshot
        };
    }

    fn counter_oids_empty(&self) -> bool {
        self.counter_oids.bw.is_empty()
            && self.counter_oids.color.is_empty()
            && self.counter_oids.total.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::logging::{LogLevel, LogStore, init_logging};

    fn test_app() -> PrintCountApp {
        let store = LogStore::new(16);
        let flags = Flags {
            log_store: store.clone(),
            reload_handle: init_logging(store, LogLevel::Info),
        };
        let (mut app, _) = PrintCountApp::new(flags);
        app.replace_printers(Vec::new());
        app.selected_printer = None;
        app
    }

    fn printer_record(status: PrinterStatus, last_seen: Option<u64>) -> PrinterRecord {
        let mut record = PrinterRecord::new(PrinterId::new("snmp-192.0.2.10"));
        record.model = Some("Test Printer".to_string());
        record.ip_or_hostname = Some("192.0.2.10".to_string());
        record.snmp_address = Some(SnmpAddress::with_default_port("192.0.2.10"));
        record.status = status;
        record.last_seen = last_seen;
        record
    }

    fn printer_record_with_id(id: &str) -> PrinterRecord {
        let mut record = printer_record(PrinterStatus::Unknown, None);
        record.id = PrinterId::new(id);
        record.model = Some(id.to_string());
        record.ip_or_hostname = Some(format!("{id}.local"));
        record
    }

    fn temp_test_dir(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("current time")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "printcountpay-actions-{name}-{}-{unique}",
            std::process::id()
        ))
    }

    fn read_manual_pricing_workspace(path: &Path) -> ManualPricingWorkspace {
        let contents = fs::read_to_string(path).expect("read manual pricing file");
        parse_manual_pricing_contents(&contents).expect("parse manual pricing file")
    }

    fn read_manual_bill_store(path: &Path) -> ManualBillStore {
        let contents = fs::read_to_string(path).expect("read manual bill store file");
        parse_manual_bill_store_contents(&contents).expect("parse manual bill store file")
    }

    fn read_statistics_store(path: &Path) -> StatisticsStore {
        load_statistics_store(path).expect("read statistics store")
    }

    #[test]
    fn failed_poll_marks_printer_offline() {
        let mut app = test_app();
        let record = printer_record(PrinterStatus::Online, Some(123));
        let printer_id = record.id.clone();

        app.replace_printers(vec![record]);
        app.handle_snmp_polled(
            printer_id.clone(),
            Err(SnmpErrorInfo::new(
                PrinterStatus::Offline,
                "SNMP request timed out for 192.0.2.10:161.",
                "SNMP timeout after 3000ms for 192.0.2.10:161.",
            )),
        );

        let record = app
            .printers
            .iter()
            .find(|record| record.id == printer_id)
            .expect("printer record");
        assert_eq!(record.status, PrinterStatus::Offline);
        assert_eq!(record.last_seen, Some(123));
        assert!(matches!(
            app.poll_states.get(&printer_id),
            Some(SnmpPollStatus::Error { .. })
        ));
    }

    #[test]
    fn statistics_tab_requires_advanced_mode() {
        let mut app = test_app();

        let _ = app.update(Message::SelectTab(Tab::Statistics));
        assert_eq!(app.active_tab, Tab::Printers);

        let _ = app.update(Message::ToggleAdvancedMode);
        assert!(app.advanced_mode);

        let _ = app.update(Message::SelectTab(Tab::Statistics));
        assert_eq!(app.active_tab, Tab::Statistics);

        let _ = app.update(Message::ToggleAdvancedMode);
        assert!(!app.advanced_mode);
        assert_eq!(app.active_tab, Tab::Printers);
    }

    #[test]
    fn discovery_uses_saved_snmp_port_for_known_host() {
        let mut app = test_app();
        let mut record = printer_record(PrinterStatus::Online, Some(123));
        record.snmp_address = Some(SnmpAddress::new("192.0.2.10", 1161));

        app.replace_printers(vec![record]);

        assert_eq!(
            app.discovery_address_for_host("192.0.2.10"),
            SnmpAddress::new("192.0.2.10", 1161)
        );
    }

    #[test]
    fn upsert_printer_matches_known_host_aliases() {
        let mut app = test_app();
        let mut existing = printer_record(PrinterStatus::Unknown, None);
        existing.ip_or_hostname = Some("192.0.2.10".to_string());
        existing.snmp_address = Some(SnmpAddress::new("printer-a.local", 1161));
        let existing_id = existing.id.clone();
        app.replace_printers(vec![existing]);

        let mut discovered = PrinterRecord::new(PrinterId::new("snmp-192.0.2.10"));
        discovered.ip_or_hostname = Some("192.0.2.10".to_string());
        discovered.model = Some("Discovered Printer".to_string());
        discovered.snmp_address = Some(SnmpAddress::new("192.0.2.10", 1161));
        discovered.status = PrinterStatus::Online;

        app.upsert_printer(discovered);

        assert_eq!(app.printers.len(), 1);
        let printer = app.printers.first().expect("single printer");
        assert_eq!(printer.id, existing_id);
        assert_eq!(printer.model.as_deref(), Some("Discovered Printer"));
        assert_eq!(
            printer.snmp_address,
            Some(SnmpAddress::new("192.0.2.10", 1161))
        );
    }

    #[test]
    fn finish_printer_reorder_drag_moves_printer_to_target_slot() {
        let mut app = test_app();
        let printer_a = printer_record_with_id("printer-a");
        let printer_b = printer_record_with_id("printer-b");
        let printer_c = printer_record_with_id("printer-c");

        app.replace_printers(vec![
            printer_a.clone(),
            printer_b.clone(),
            printer_c.clone(),
        ]);

        app.start_printer_reorder_drag(printer_a.id.clone());
        if let Some(pending) = app.pending_printer_drag.as_mut() {
            pending.pressed_at = Instant::now() - PRINTER_REORDER_HOLD_DURATION;
        }
        app.activate_printer_reorder_drag_if_ready();
        app.hover_printer_reorder_drop(3);

        assert!(app.finish_printer_reorder_drag());
        assert_eq!(
            app.printers
                .iter()
                .map(|record| record.id.to_string())
                .collect::<Vec<_>>(),
            vec!["printer-b", "printer-c", "printer-a"]
        );
    }

    #[test]
    fn printer_reorder_requires_hold_before_drag_activates() {
        let mut app = test_app();
        let printer_a = printer_record_with_id("printer-a");

        app.replace_printers(vec![printer_a.clone()]);
        app.start_printer_reorder_drag(printer_a.id.clone());
        app.activate_printer_reorder_drag_if_ready();

        assert!(app.pending_printer_drag.is_some());
        assert!(app.active_printer_drag.is_none());

        if let Some(pending) = app.pending_printer_drag.as_mut() {
            pending.pressed_at = Instant::now() - PRINTER_REORDER_HOLD_DURATION;
        }
        app.activate_printer_reorder_drag_if_ready();

        assert!(app.pending_printer_drag.is_none());
        assert_eq!(
            app.active_printer_drag
                .as_ref()
                .map(|drag| drag.printer_id.to_string()),
            Some("printer-a".to_string())
        );
    }

    #[test]
    fn complete_printer_card_press_selects_printer_when_hold_not_reached() {
        let mut app = test_app();
        let printer_a = printer_record_with_id("printer-a");

        app.replace_printers(vec![printer_a.clone()]);
        let _ = app.complete_printer_card_press(printer_a.id.clone());
        assert!(app.selected_printer.is_none());

        app.start_printer_reorder_drag(printer_a.id.clone());
        let _ = app.complete_printer_card_press(printer_a.id.clone());

        assert_eq!(app.selected_printer, Some(printer_a.id));
        assert!(app.pending_printer_drag.is_none());
        assert!(app.active_printer_drag.is_none());
    }

    #[test]
    fn apply_shared_state_preserves_pending_drag_for_known_printer() {
        let mut app = test_app();
        let printer_a = printer_record_with_id("printer-a");
        let printer_b = printer_record_with_id("printer-b");

        app.replace_printers(vec![printer_a.clone(), printer_b.clone()]);
        app.start_printer_reorder_drag(printer_a.id.clone());
        app.apply_shared_state(sync::SharedState {
            revision: 2,
            printers: vec![printer_b.clone(), printer_a.clone()],
            poll_states: vec![
                sync::PollStateEntry {
                    printer_id: printer_b.id.clone(),
                    state: SnmpPollStatus::Idle,
                },
                sync::PollStateEntry {
                    printer_id: printer_a.id.clone(),
                    state: SnmpPollStatus::Idle,
                },
            ],
            recording_sessions: Vec::new(),
            pricing: app.pricing.clone(),
            bill_sync_supported: false,
            manual_bills: Vec::new(),
            manual_bill_tombstones: Vec::new(),
        });

        assert_eq!(
            app.pending_printer_drag
                .as_ref()
                .map(|pending| pending.printer_id.to_string()),
            Some("printer-a".to_string())
        );
        assert!(app.active_printer_drag.is_none());
    }

    #[test]
    fn apply_shared_state_preserves_snapshot_printer_order() {
        let mut app = test_app();
        let printer_a = printer_record_with_id("printer-a");
        let printer_b = printer_record_with_id("printer-b");

        app.replace_printers(vec![printer_a.clone(), printer_b.clone()]);
        app.apply_shared_state(sync::SharedState {
            revision: 2,
            printers: vec![printer_b.clone(), printer_a.clone()],
            poll_states: vec![
                sync::PollStateEntry {
                    printer_id: printer_b.id.clone(),
                    state: SnmpPollStatus::Idle,
                },
                sync::PollStateEntry {
                    printer_id: printer_a.id.clone(),
                    state: SnmpPollStatus::Idle,
                },
            ],
            recording_sessions: Vec::new(),
            pricing: app.pricing.clone(),
            bill_sync_supported: false,
            manual_bills: Vec::new(),
            manual_bill_tombstones: Vec::new(),
        });

        assert_eq!(
            app.printers
                .iter()
                .map(|record| record.id.to_string())
                .collect::<Vec<_>>(),
            vec!["printer-b", "printer-a"]
        );
    }

    #[test]
    fn apply_shared_state_preserves_local_end_unlock_flag() {
        let mut app = test_app();
        let record = printer_record(PrinterStatus::Online, Some(123));
        let printer_id = record.id.clone();
        app.replace_printers(vec![record.clone()]);

        let mut local_session = RecordingSession::default();
        local_session.end_fields_unlocked = true;
        app.recording_sessions
            .insert(printer_id.clone(), local_session.clone());

        let mut remote_session = RecordingSession::default();
        remote_session.end_fields_unlocked = false;
        app.apply_shared_state(sync::SharedState {
            revision: 2,
            printers: vec![record],
            poll_states: vec![sync::PollStateEntry {
                printer_id: printer_id.clone(),
                state: SnmpPollStatus::Idle,
            }],
            recording_sessions: vec![sync::RecordingSessionEntry {
                printer_id: printer_id.clone(),
                session: remote_session,
            }],
            pricing: app.pricing.clone(),
            bill_sync_supported: false,
            manual_bills: Vec::new(),
            manual_bill_tombstones: Vec::new(),
        });

        assert_eq!(
            app.recording_sessions
                .get(&printer_id)
                .map(|session| session.end_fields_unlocked),
            Some(true)
        );
    }

    #[test]
    fn apply_shared_state_keeps_local_stopped_session_when_remote_active_is_older() {
        let mut app = test_app();
        let record = printer_record(PrinterStatus::Online, Some(123));
        let printer_id = record.id.clone();
        app.replace_printers(vec![record.clone()]);

        let mut local_session = RecordingSession::default();
        local_session.active = false;
        local_session.start = Some(RecordingSnapshot {
            received_at: 100,
            bw_printer: Some(100),
            bw_copier: None,
            color_printer: None,
            color_copier: None,
        });
        local_session.end = Some(RecordingSnapshot {
            received_at: 200,
            bw_printer: Some(120),
            bw_copier: None,
            color_printer: None,
            color_copier: None,
        });
        local_session.status = Some("Stopped locally.".to_string());
        local_session.edits.prints_bw.end_input = "120".to_string();
        app.recording_sessions
            .insert(printer_id.clone(), local_session.clone());
        app.last_shared_state = app.build_shared_state(2);

        let mut remote_session = RecordingSession::default();
        remote_session.active = true;
        remote_session.start = Some(RecordingSnapshot {
            received_at: 150,
            bw_printer: Some(110),
            bw_copier: None,
            color_printer: None,
            color_copier: None,
        });

        app.apply_shared_state(sync::SharedState {
            revision: 3,
            printers: vec![record],
            poll_states: vec![sync::PollStateEntry {
                printer_id: printer_id.clone(),
                state: SnmpPollStatus::Idle,
            }],
            recording_sessions: vec![sync::RecordingSessionEntry {
                printer_id: printer_id.clone(),
                session: remote_session,
            }],
            pricing: app.pricing.clone(),
            bill_sync_supported: false,
            manual_bills: Vec::new(),
            manual_bill_tombstones: Vec::new(),
        });

        let applied = app
            .recording_sessions
            .get(&printer_id)
            .expect("recording session should exist");
        assert!(!applied.active);
        assert_eq!(
            applied.end.as_ref().map(|snapshot| snapshot.received_at),
            Some(200)
        );
        assert_eq!(applied.edits.prints_bw.end_input, "120");
    }

    #[test]
    fn apply_shared_state_keeps_local_active_session_when_remote_snapshot_omits_it() {
        let mut app = test_app();
        let record = printer_record(PrinterStatus::Online, Some(123));
        let printer_id = record.id.clone();
        app.replace_printers(vec![record.clone()]);

        let mut local_session = RecordingSession::default();
        local_session.active = true;
        local_session.start = Some(RecordingSnapshot {
            received_at: 300,
            bw_printer: Some(130),
            bw_copier: None,
            color_printer: None,
            color_copier: None,
        });
        app.recording_sessions
            .insert(printer_id.clone(), local_session.clone());
        app.last_shared_state = app.build_shared_state(4);

        app.apply_shared_state(sync::SharedState {
            revision: 5,
            printers: vec![record],
            poll_states: vec![sync::PollStateEntry {
                printer_id: printer_id.clone(),
                state: SnmpPollStatus::Idle,
            }],
            recording_sessions: Vec::new(),
            pricing: app.pricing.clone(),
            bill_sync_supported: false,
            manual_bills: Vec::new(),
            manual_bill_tombstones: Vec::new(),
        });

        let applied = app
            .recording_sessions
            .get(&printer_id)
            .expect("recording session should remain active");
        assert!(applied.active);
        assert_eq!(
            applied.start.as_ref().map(|snapshot| snapshot.received_at),
            Some(300)
        );
    }

    #[test]
    fn apply_shared_state_keeps_local_active_session_when_remote_inactive_is_older() {
        let mut app = test_app();
        let record = printer_record(PrinterStatus::Online, Some(123));
        let printer_id = record.id.clone();
        app.replace_printers(vec![record.clone()]);

        let mut local_session = RecordingSession::default();
        local_session.active = true;
        local_session.start = Some(RecordingSnapshot {
            received_at: 300,
            bw_printer: Some(130),
            bw_copier: None,
            color_printer: None,
            color_copier: None,
        });
        app.recording_sessions
            .insert(printer_id.clone(), local_session.clone());
        app.last_shared_state = app.build_shared_state(6);

        let mut remote_session = RecordingSession::default();
        remote_session.active = false;
        remote_session.start = Some(RecordingSnapshot {
            received_at: 100,
            bw_printer: Some(100),
            bw_copier: None,
            color_printer: None,
            color_copier: None,
        });
        remote_session.end = Some(RecordingSnapshot {
            received_at: 200,
            bw_printer: Some(120),
            bw_copier: None,
            color_printer: None,
            color_copier: None,
        });

        app.apply_shared_state(sync::SharedState {
            revision: 7,
            printers: vec![record],
            poll_states: vec![sync::PollStateEntry {
                printer_id: printer_id.clone(),
                state: SnmpPollStatus::Idle,
            }],
            recording_sessions: vec![sync::RecordingSessionEntry {
                printer_id: printer_id.clone(),
                session: remote_session,
            }],
            pricing: app.pricing.clone(),
            bill_sync_supported: false,
            manual_bills: Vec::new(),
            manual_bill_tombstones: Vec::new(),
        });

        let applied = app
            .recording_sessions
            .get(&printer_id)
            .expect("recording session should remain active");
        assert!(applied.active);
        assert_eq!(
            applied.start.as_ref().map(|snapshot| snapshot.received_at),
            Some(300)
        );

        assert_eq!(app.last_shared_state.revision, 7);
        assert!(
            app.last_shared_state
                .recording_sessions
                .iter()
                .any(|entry| entry.printer_id == printer_id && !entry.session.active)
        );

        app.flush_shared_state();

        assert_eq!(app.last_shared_state.revision, 8);
        assert!(
            app.last_shared_state
                .recording_sessions
                .iter()
                .any(|entry| entry.printer_id == printer_id && entry.session.active)
        );
    }

    #[test]
    fn apply_shared_state_accepts_remote_manual_stop_when_snapshot_time_looks_older() {
        let mut app = test_app();
        let record = printer_record(PrinterStatus::Online, Some(123));
        let printer_id = record.id.clone();
        app.replace_printers(vec![record.clone()]);

        let mut local_session = RecordingSession::default();
        local_session.active = true;
        local_session.start = Some(RecordingSnapshot {
            received_at: 300,
            bw_printer: Some(130),
            bw_copier: None,
            color_printer: None,
            color_copier: None,
        });
        local_session.updated_at_millis = 1_000;
        local_session.manual_state_changed_at_millis = 1_000;
        app.recording_sessions
            .insert(printer_id.clone(), local_session);
        app.last_shared_state = app.build_shared_state(6);

        let mut remote_session = RecordingSession::default();
        remote_session.active = false;
        remote_session.start = Some(RecordingSnapshot {
            received_at: 100,
            bw_printer: Some(100),
            bw_copier: None,
            color_printer: None,
            color_copier: None,
        });
        remote_session.end = Some(RecordingSnapshot {
            received_at: 200,
            bw_printer: Some(120),
            bw_copier: None,
            color_printer: None,
            color_copier: None,
        });
        remote_session.updated_at_millis = 2_000;
        remote_session.manual_state_changed_at_millis = 2_000;

        app.apply_shared_state(sync::SharedState {
            revision: 7,
            printers: vec![record],
            poll_states: vec![sync::PollStateEntry {
                printer_id: printer_id.clone(),
                state: SnmpPollStatus::Idle,
            }],
            recording_sessions: vec![sync::RecordingSessionEntry {
                printer_id: printer_id.clone(),
                session: remote_session,
            }],
            pricing: app.pricing.clone(),
            bill_sync_supported: false,
            manual_bills: Vec::new(),
            manual_bill_tombstones: Vec::new(),
        });

        let applied = app
            .recording_sessions
            .get(&printer_id)
            .expect("recording session should exist");
        assert!(!applied.active);
        assert_eq!(applied.manual_state_changed_at_millis, 2_000);
        assert_eq!(
            applied.end.as_ref().map(|snapshot| snapshot.received_at),
            Some(200)
        );
    }

    #[test]
    fn apply_shared_state_accepts_remote_active_session_when_started_after_local_end() {
        let mut app = test_app();
        let record = printer_record(PrinterStatus::Online, Some(123));
        let printer_id = record.id.clone();
        app.replace_printers(vec![record.clone()]);

        let mut local_session = RecordingSession::default();
        local_session.active = false;
        local_session.start = Some(RecordingSnapshot {
            received_at: 100,
            bw_printer: Some(100),
            bw_copier: None,
            color_printer: None,
            color_copier: None,
        });
        local_session.end = Some(RecordingSnapshot {
            received_at: 200,
            bw_printer: Some(120),
            bw_copier: None,
            color_printer: None,
            color_copier: None,
        });
        app.recording_sessions.insert(printer_id.clone(), local_session);
        app.last_shared_state = app.build_shared_state(2);

        let mut remote_session = RecordingSession::default();
        remote_session.active = true;
        remote_session.start = Some(RecordingSnapshot {
            received_at: 250,
            bw_printer: Some(130),
            bw_copier: None,
            color_printer: None,
            color_copier: None,
        });

        app.apply_shared_state(sync::SharedState {
            revision: 3,
            printers: vec![record],
            poll_states: vec![sync::PollStateEntry {
                printer_id: printer_id.clone(),
                state: SnmpPollStatus::Idle,
            }],
            recording_sessions: vec![sync::RecordingSessionEntry {
                printer_id: printer_id.clone(),
                session: remote_session,
            }],
            pricing: app.pricing.clone(),
            bill_sync_supported: false,
            manual_bills: Vec::new(),
            manual_bill_tombstones: Vec::new(),
        });

        let applied = app
            .recording_sessions
            .get(&printer_id)
            .expect("recording session should exist");
        assert!(applied.active);
        assert_eq!(
            applied.start.as_ref().map(|snapshot| snapshot.received_at),
            Some(250)
        );
    }

    #[test]
    fn apply_shared_state_keeps_fresher_local_poll_state() {
        let mut app = test_app();
        let record = printer_record(PrinterStatus::Online, Some(123));
        let printer_id = record.id.clone();
        app.replace_printers(vec![record.clone()]);
        app.poll_states.insert(
            printer_id.clone(),
            SnmpPollStatus::Ok {
                received_at: 200,
                varbinds: Vec::new(),
            },
        );
        app.last_shared_state = app.build_shared_state(4);

        app.apply_shared_state(sync::SharedState {
            revision: 5,
            printers: vec![record],
            poll_states: vec![sync::PollStateEntry {
                printer_id: printer_id.clone(),
                state: SnmpPollStatus::Ok {
                    received_at: 100,
                    varbinds: Vec::new(),
                },
            }],
            recording_sessions: Vec::new(),
            pricing: app.pricing.clone(),
            bill_sync_supported: false,
            manual_bills: Vec::new(),
            manual_bill_tombstones: Vec::new(),
        });

        assert_eq!(
            app.poll_states.get(&printer_id).and_then(poll_received_at),
            Some(200)
        );
        assert_eq!(app.last_shared_state.revision, 5);
        assert!(
            app.last_shared_state
                .poll_states
                .iter()
                .any(|entry| entry.printer_id == printer_id
                    && poll_received_at(&entry.state) == Some(100))
        );

        app.flush_shared_state();

        assert_eq!(app.last_shared_state.revision, 6);
        assert!(
            app.last_shared_state
                .poll_states
                .iter()
                .any(|entry| entry.printer_id == printer_id
                    && poll_received_at(&entry.state) == Some(200))
        );
    }

    #[test]
    fn apply_shared_state_rebroadcasts_local_newer_bill_on_next_flush() {
        let mut app = test_app();
        app.manual_bills = vec![ManualPricingBill {
            id: "shared-bill".to_string(),
            subject: "Local newer".to_string(),
            pricing: ManualPricingSettings::default(),
            updated_at_millis: 200,
        }];
        app.last_shared_state = app.build_shared_state(5);

        app.apply_shared_state(sync::SharedState {
            revision: 6,
            printers: Vec::new(),
            poll_states: Vec::new(),
            recording_sessions: Vec::new(),
            pricing: app.pricing.clone(),
            bill_sync_supported: true,
            manual_bills: vec![ManualPricingBill {
                id: "shared-bill".to_string(),
                subject: "Remote older".to_string(),
                pricing: ManualPricingSettings::default(),
                updated_at_millis: 100,
            }],
            manual_bill_tombstones: Vec::new(),
        });

        assert_eq!(app.manual_bills.len(), 1);
        assert_eq!(app.manual_bills[0].subject, "Local newer");
        assert_eq!(app.last_shared_state.revision, 6);
        assert_eq!(app.last_shared_state.manual_bills[0].subject, "Remote older");

        app.flush_shared_state();

        assert_eq!(app.last_shared_state.revision, 7);
        assert_eq!(app.last_shared_state.manual_bills[0].subject, "Local newer");
    }

    #[test]
    fn save_manual_pricing_as_bill_copies_current_calculator_state() {
        let mut app = test_app();
        app.manual_pricing.a3_input = "3.25".to_string();
        app.manual_pricing.binding_input = "4.20".to_string();
        app.manual_pricing.line_items[0].sides_input = "12".to_string();
        app.manual_pricing.line_items[0].sync_sheets_from_sides();
        app.manual_pricing
            .finisher_items
            .push(ManualFinisherLineItem {
                finisher_type: ManualFinisherType::Binding,
                laminate_size: ManualLaminateSize::A4,
                amount_input: "2".to_string(),
            });
        app.manual_pricing.discount_input = "5".to_string();
        app.manual_pricing.rounding_mode = ManualRoundingMode::HalfEuro;

        app.save_manual_pricing_as_bill();

        assert_eq!(app.manual_bills.len(), 1);
        assert!(app.manual_pricing_selected);
        assert_eq!(app.selected_manual_bill_id, None);
        assert_eq!(app.manual_pricing_tab, ManualPricingTab::Calculator);
        assert!(!app.manual_bills[0].id.is_empty());
        assert!(app.manual_bills[0].id.contains('-'));
        assert!(!app.manual_bills[0].subject.trim().is_empty());
        assert_eq!(app.manual_bills[0].pricing.line_items[0].sides_input, "12");
        assert_eq!(app.manual_bills[0].pricing.line_items[0].sheets_input, "12");
        assert_eq!(app.manual_bills[0].pricing.finisher_items.len(), 1);
        assert_eq!(app.manual_bills[0].pricing.discount_input, "5");
        assert_eq!(
            app.manual_bills[0].pricing.rounding_mode,
            ManualRoundingMode::HalfEuro
        );
        assert_eq!(app.manual_pricing.a3_input, "3.25");
        assert_eq!(app.manual_pricing.binding_input, "4.20");
        assert_eq!(
            app.manual_pricing.line_items,
            vec![ManualPricingLineItem::default()]
        );
        assert!(app.manual_pricing.finisher_items.is_empty());
        assert!(app.manual_pricing.discount_input.is_empty());
        assert_eq!(
            app.manual_pricing.rounding_mode,
            ManualRoundingMode::FiveCents
        );
    }

    #[test]
    fn legacy_snapshot_preserves_local_manual_pricing_and_bills() {
        let mut app = test_app();
        app.manual_pricing.line_items[0].sides_input = "9".to_string();
        app.manual_pricing.line_items[0].sync_sheets_from_sides();
        app.manual_pricing.discount_input = "8".to_string();
        app.save_manual_pricing_as_bill();
        let local_manual_pricing = app.manual_pricing.clone();
        let local_bill_id = app.manual_bills[0].id.clone();

        app.apply_shared_state(sync::SharedState {
            revision: 2,
            printers: Vec::new(),
            poll_states: Vec::new(),
            recording_sessions: Vec::new(),
            pricing: app.pricing.clone(),
            bill_sync_supported: false,
            manual_bills: Vec::new(),
            manual_bill_tombstones: Vec::new(),
        });

        assert_eq!(app.manual_pricing, local_manual_pricing);
        assert_eq!(app.manual_bills.len(), 1);
        assert_eq!(app.manual_bills[0].id, local_bill_id);
    }

    #[test]
    fn load_manual_pricing_from_path_clears_main_calculator_preserving_prices_and_bills() {
        let mut app = test_app();
        let root = temp_test_dir("load-manual-pricing-reset");
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("manual_pricing.ron");

        let workspace = ManualPricingWorkspace {
            settings: ManualPricingSettings {
                a0_input: "30".to_string(),
                a3_input: "3.25".to_string(),
                binding_input: "6.50".to_string(),
                line_items: vec![ManualPricingLineItem {
                    size: ManualPrintSize::A0,
                    print_mode: ManualPrintMode::Bw,
                    modifier_index: Some(0),
                    double_sided: false,
                    sheets_input: "7".to_string(),
                    sides_input: "7".to_string(),
                }],
                finisher_items: vec![ManualFinisherLineItem {
                    finisher_type: ManualFinisherType::Laminate,
                    laminate_size: ManualLaminateSize::A0,
                    amount_input: "10".to_string(),
                }],
                discount_input: "15".to_string(),
                rounding_mode: ManualRoundingMode::HalfEuro,
                ..ManualPricingSettings::default()
            },
            bills: vec![ManualPricingBill {
                id: "saved-bill".to_string(),
                subject: "Saved Bill".to_string(),
                pricing: ManualPricingSettings {
                    discount_input: "5".to_string(),
                    rounding_mode: ManualRoundingMode::HalfEuro,
                    line_items: vec![ManualPricingLineItem {
                        sides_input: "9".to_string(),
                        sheets_input: "9".to_string(),
                        ..ManualPricingLineItem::default()
                    }],
                    ..ManualPricingSettings::default()
                },
                updated_at_millis: 100,
            }],
            bill_tombstones: Vec::new(),
        };
        write_manual_pricing_workspace(&path, &workspace).expect("write workspace");
        app.manual_pricing_path = path.to_string_lossy().to_string();

        app.load_manual_pricing_from_path();

        assert_eq!(app.manual_pricing.a0_input, "30");
        assert_eq!(app.manual_pricing.a3_input, "3.25");
        assert_eq!(app.manual_pricing.binding_input, "6.50");
        assert_eq!(
            app.manual_pricing.line_items,
            vec![ManualPricingLineItem::default()]
        );
        assert!(app.manual_pricing.finisher_items.is_empty());
        assert!(app.manual_pricing.discount_input.is_empty());
        assert_eq!(
            app.manual_pricing.rounding_mode,
            ManualRoundingMode::FiveCents
        );
        assert_eq!(app.manual_bills.len(), 1);
        assert_eq!(app.manual_bills[0].id, "saved-bill");
        assert_eq!(app.manual_bills[0].pricing.discount_input, "5");
        assert_eq!(
            app.manual_bills[0].pricing.rounding_mode,
            ManualRoundingMode::HalfEuro
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn parse_manual_pricing_contents_loads_legacy_settings_file() {
        let contents = r#"(
            a0_input: "30",
            a1_input: "20",
            a2_input: "10",
            a3_input: "1.00",
            a4_input: "0.50",
            a3_bw_first_input: "0.35",
            a3_bw_next_input: "0.20",
            a3_bw_rest_input: "0.12",
            a3_color_first_input: "1.25",
            a3_color_rest_input: "0.5",
            a4_bw_first_input: "0.25",
            a4_bw_next_input: "0.10",
            a4_bw_rest_input: "0.06",
            a4_color_first_input: "0.75",
            a4_color_rest_input: "0.50",
            laminate_a2_input: "5",
            laminate_a3_input: "2",
            laminate_a4_input: "1",
            laminate_a5_input: "0.7",
            folding_input: "0.2",
            binding_input: "3.50",
            modifiers: [],
            line_items: [],
            finisher_items: [],
            cutting_enabled: false,
            discount_input: "",
            rounding_mode: HalfEuro,
        )"#;

        let workspace = parse_manual_pricing_contents(contents).expect("legacy pricing file");

        assert_eq!(workspace.settings.a0_input, "30");
        assert_eq!(workspace.settings.a4_color_first_input, "0.75");
        assert_eq!(
            workspace.settings.rounding_mode,
            ManualRoundingMode::HalfEuro
        );
        assert!(workspace.bills.is_empty());
    }

    #[test]
    fn delete_selected_manual_pricing_bill_removes_current_bill() {
        let mut app = test_app();
        app.save_manual_pricing_as_bill();
        let deleted_id = app.manual_bills[0].id.clone();
        app.selected_manual_bill_id = Some(deleted_id.clone());

        app.delete_selected_manual_pricing_bill();

        assert!(app.manual_bills.is_empty());
        assert_eq!(app.manual_bill_tombstones.len(), 1);
        assert_eq!(app.manual_bill_tombstones[0].id, deleted_id);
        assert!(app.manual_pricing_selected);
        assert_eq!(app.selected_manual_bill_id, None);
        assert_eq!(
            app.manual_pricing_status,
            Some(format!("Deleted bill {deleted_id}."))
        );
    }

    #[test]
    fn manual_bill_store_persists_saved_bills_and_tombstones() {
        let mut app = test_app();
        let root = temp_test_dir("manual-bill-store");
        fs::create_dir_all(&root).expect("create temp root");
        let store_path = root.join("manual_bills.ron");
        app.manual_bill_store_path = store_path.to_string_lossy().to_string();

        app.save_manual_pricing_as_bill();
        app.persist_manual_bill_store_if_dirty();

        let stored = read_manual_bill_store(&store_path);
        assert_eq!(stored.bills.len(), 1);
        assert!(stored.bill_tombstones.is_empty());

        let deleted_id = stored.bills[0].id.clone();
        app.selected_manual_bill_id = Some(deleted_id.clone());
        app.delete_selected_manual_pricing_bill();
        app.persist_manual_bill_store_if_dirty();

        let stored = read_manual_bill_store(&store_path);
        assert!(stored.bills.is_empty());
        assert_eq!(stored.bill_tombstones.len(), 1);
        assert_eq!(stored.bill_tombstones[0].id, deleted_id);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_manual_bill_store_prefers_newer_local_bill_snapshot() {
        let mut app = test_app();
        let root = temp_test_dir("manual-bill-merge");
        fs::create_dir_all(&root).expect("create temp root");
        let workspace_path = root.join("manual_pricing.ron");
        let store_path = root.join("manual_bills.ron");

        let workspace = ManualPricingWorkspace {
            settings: ManualPricingSettings::default(),
            bills: vec![ManualPricingBill {
                id: "saved-bill".to_string(),
                subject: "Older Subject".to_string(),
                pricing: ManualPricingSettings::default(),
                updated_at_millis: 10,
            }],
            bill_tombstones: Vec::new(),
        };
        write_manual_pricing_workspace(&workspace_path, &workspace).expect("write workspace");

        let store = ManualBillStore {
            bills: vec![
                ManualPricingBill {
                    id: "saved-bill".to_string(),
                    subject: "Newer Subject".to_string(),
                    pricing: ManualPricingSettings {
                        discount_input: "5".to_string(),
                        ..ManualPricingSettings::default()
                    },
                    updated_at_millis: 20,
                },
                ManualPricingBill {
                    id: "local-only".to_string(),
                    subject: "Local Only".to_string(),
                    pricing: ManualPricingSettings::default(),
                    updated_at_millis: 15,
                },
            ],
            bill_tombstones: Vec::new(),
        };
        write_manual_bill_store(&store_path, &store).expect("write bill store");

        app.manual_pricing_path = workspace_path.to_string_lossy().to_string();
        app.manual_bill_store_path = store_path.to_string_lossy().to_string();

        app.load_manual_pricing_from_path();
        app.load_manual_bill_store_if_present();

        assert_eq!(app.manual_bills.len(), 2);
        assert_eq!(app.manual_bills[0].id, "saved-bill");
        assert_eq!(app.manual_bills[0].subject, "Newer Subject");
        assert_eq!(app.manual_bills[0].pricing.discount_input, "5");
        assert!(app.manual_bills.iter().any(|bill| bill.id == "local-only"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn manual_pricing_save_keeps_three_backups() {
        let root = temp_test_dir("manual-pricing-backups");
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("manual_pricing.ron");
        let mut workspace = ManualPricingWorkspace::default();

        for value in 0..5 {
            workspace.settings.a0_input = value.to_string();
            write_manual_pricing_workspace(&path, &workspace).expect("write workspace");
        }

        assert_eq!(read_manual_pricing_workspace(&path).settings.a0_input, "4");
        assert_eq!(
            read_manual_pricing_workspace(&manual_pricing_backup_path(&path, 1))
                .settings
                .a0_input,
            "3"
        );
        assert_eq!(
            read_manual_pricing_workspace(&manual_pricing_backup_path(&path, 2))
                .settings
                .a0_input,
            "2"
        );
        assert_eq!(
            read_manual_pricing_workspace(&manual_pricing_backup_path(&path, 3))
                .settings
                .a0_input,
            "1"
        );
        assert!(!manual_pricing_backup_path(&path, 4).exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn apply_pricing_sync_updates_state_and_persists_workspace() {
        let mut app = test_app();
        let root = temp_test_dir("apply-pricing-sync");
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("manual_pricing.ron");

        let mut existing_workspace = ManualPricingWorkspace::default();
        existing_workspace.settings.a0_input = "10".to_string();
        write_manual_pricing_workspace(&path, &existing_workspace).expect("seed workspace");
        app.manual_pricing_path = path.to_string_lossy().to_string();

        let mut pricing = PricingSettings::default();
        pricing.color_input = "0.75".to_string();
        let workspace = ManualPricingWorkspace {
            settings: ManualPricingSettings {
                a0_input: "99".to_string(),
                discount_input: "12".to_string(),
                rounding_mode: ManualRoundingMode::HalfEuro,
                line_items: vec![ManualPricingLineItem {
                    sides_input: "4".to_string(),
                    sheets_input: "4".to_string(),
                    ..ManualPricingLineItem::default()
                }],
                ..ManualPricingSettings::default()
            },
            bills: vec![ManualPricingBill {
                id: "shared-bill".to_string(),
                subject: "Shared Bill".to_string(),
                pricing: ManualPricingSettings {
                    discount_input: "5".to_string(),
                    ..ManualPricingSettings::default()
                },
                updated_at_millis: 200,
            }],
            bill_tombstones: Vec::new(),
        };

        app.apply_pricing_sync(sync::PricingSyncPayload {
            id: "sync-1".to_string(),
            pricing: pricing.clone(),
            workspace: workspace.clone(),
        });

        assert_eq!(app.pricing.color_input, "0.75");
        assert_eq!(app.manual_pricing.a0_input, "99");
        assert_eq!(
            app.manual_pricing.line_items,
            vec![ManualPricingLineItem::default()]
        );
        assert!(app.manual_pricing.discount_input.is_empty());
        assert_eq!(
            app.manual_pricing.rounding_mode,
            ManualRoundingMode::FiveCents
        );
        assert_eq!(app.manual_bills.len(), 1);
        assert_eq!(app.manual_bills[0].id, "shared-bill");
        let persisted_workspace = read_manual_pricing_workspace(&path);
        assert_eq!(persisted_workspace.settings.a0_input, "99");
        assert_eq!(
            persisted_workspace.settings.line_items,
            vec![ManualPricingLineItem::default()]
        );
        assert!(persisted_workspace.settings.discount_input.is_empty());
        assert_eq!(
            persisted_workspace.settings.rounding_mode,
            ManualRoundingMode::FiveCents
        );
        assert_eq!(persisted_workspace.bills, workspace.bills);
        assert_eq!(
            read_manual_pricing_workspace(&manual_pricing_backup_path(&path, 1))
                .settings
                .a0_input,
            "10"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn apply_pricing_sync_merges_local_newer_bills_and_rebroadcasts_them() {
        let mut app = test_app();
        let root = temp_test_dir("apply-pricing-sync-merge");
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("manual_pricing.ron");
        write_manual_pricing_workspace(&path, &ManualPricingWorkspace::default())
            .expect("seed workspace");
        app.manual_pricing_path = path.to_string_lossy().to_string();
        app.manual_bills = vec![
            ManualPricingBill {
                id: "shared-bill".to_string(),
                subject: "Local Newer".to_string(),
                pricing: ManualPricingSettings::default(),
                updated_at_millis: 300,
            },
            ManualPricingBill {
                id: "local-only".to_string(),
                subject: "Local Only".to_string(),
                pricing: ManualPricingSettings::default(),
                updated_at_millis: 250,
            },
        ];
        app.last_shared_state = app.build_shared_state(9);

        let incoming_workspace = ManualPricingWorkspace {
            settings: ManualPricingSettings::default(),
            bills: vec![
                ManualPricingBill {
                    id: "shared-bill".to_string(),
                    subject: "Remote Older".to_string(),
                    pricing: ManualPricingSettings::default(),
                    updated_at_millis: 100,
                },
                ManualPricingBill {
                    id: "remote-only".to_string(),
                    subject: "Remote Only".to_string(),
                    pricing: ManualPricingSettings::default(),
                    updated_at_millis: 200,
                },
            ],
            bill_tombstones: Vec::new(),
        };

        app.apply_pricing_sync(sync::PricingSyncPayload {
            id: "sync-2".to_string(),
            pricing: PricingSettings::default(),
            workspace: incoming_workspace.clone(),
        });

        assert_eq!(app.manual_bills.len(), 3);
        assert_eq!(
            app.manual_bills
                .iter()
                .find(|bill| bill.id == "shared-bill")
                .map(|bill| bill.subject.as_str()),
            Some("Local Newer")
        );
        assert!(app.manual_bills.iter().any(|bill| bill.id == "local-only"));
        assert!(app.manual_bills.iter().any(|bill| bill.id == "remote-only"));

        let persisted_workspace = read_manual_pricing_workspace(&path);
        assert_eq!(persisted_workspace.bills.len(), 3);
        assert_eq!(
            persisted_workspace
                .bills
                .iter()
                .find(|bill| bill.id == "shared-bill")
                .map(|bill| bill.subject.as_str()),
            Some("Local Newer")
        );

        assert_eq!(app.last_shared_state.revision, 9);
        assert_eq!(app.last_shared_state.manual_bills, incoming_workspace.bills);

        app.flush_shared_state();

        assert_eq!(app.last_shared_state.revision, 10);
        assert_eq!(app.last_shared_state.manual_bills.len(), 3);
        assert_eq!(
            app.last_shared_state
                .manual_bills
                .iter()
                .find(|bill| bill.id == "shared-bill")
                .map(|bill| bill.subject.as_str()),
            Some("Local Newer")
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn apply_pricing_sync_ignores_older_payload_than_loaded_workspace() {
        let mut app = test_app();
        let root = temp_test_dir("stale-pricing-sync");
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("manual_pricing.ron");

        let mut current_workspace = ManualPricingWorkspace::default();
        current_workspace.settings.a3_color_rest_input = "1".to_string();
        write_manual_pricing_workspace(&path, &current_workspace).expect("seed workspace");

        app.manual_pricing_path = path.to_string_lossy().to_string();
        app.load_manual_pricing_from_path();

        let mut stale_workspace = ManualPricingWorkspace::default();
        stale_workspace.settings.a3_color_rest_input = "0.5".to_string();

        app.apply_pricing_sync(sync::PricingSyncPayload {
            id: "1".to_string(),
            pricing: PricingSettings::default(),
            workspace: stale_workspace,
        });

        assert_eq!(app.manual_pricing.a3_color_rest_input, "1");
        assert_eq!(
            read_manual_pricing_workspace(&path)
                .settings
                .a3_color_rest_input,
            "1"
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn reset_end_to_polled_uses_locked_value() {
        let mut app = test_app();
        let record = printer_record(PrinterStatus::Online, Some(123));
        let printer_id = record.id.clone();
        app.replace_printers(vec![record]);
        app.selected_printer = Some(printer_id.clone());

        let mut session = RecordingSession::default();
        session.end_fields_unlocked = true;
        session.end = Some(RecordingSnapshot {
            received_at: 123,
            bw_printer: Some(456),
            bw_copier: Some(321),
            color_printer: None,
            color_copier: None,
        });
        session.edits.copies_bw.end_input = "999".to_string();
        app.recording_sessions.insert(printer_id.clone(), session);

        app.reset_recording_end_to_polled(RecordingCategory::CopiesBw);

        assert_eq!(
            app.recording_sessions
                .get(&printer_id)
                .map(|session| session.edits.copies_bw.end_input.as_str()),
            Some("321")
        );
    }

    #[test]
    fn start_recording_requires_values_for_configured_categories() {
        let mut app = test_app();
        let record = printer_record(PrinterStatus::Online, Some(123));
        let printer_id = record.id.clone();
        let copies_bw_oid = Oid::from_slice(&[1, 3, 6, 1, 4, 1, 999, 1]);
        let prints_bw_oid = Oid::from_slice(&[1, 3, 6, 1, 4, 1, 999, 2]);

        app.replace_printers(vec![record]);
        app.selected_printer = Some(printer_id.clone());
        app.recording_oids = RecordingOidSettings {
            copies_bw_input: format_oid_list(&[copies_bw_oid.clone()]),
            copies_color_input: String::new(),
            prints_bw_input: format_oid_list(&[prints_bw_oid.clone()]),
            prints_color_input: String::new(),
        };
        app.poll_states.insert(
            printer_id.clone(),
            SnmpPollStatus::Ok {
                received_at: 123,
                varbinds: vec![SnmpVarBind {
                    oid: prints_bw_oid,
                    value: printcountpay_core::SnmpValue::Counter32(456),
                }],
            },
        );

        app.start_recording();

        let session = app
            .recording_sessions
            .get(&printer_id)
            .expect("recording session should be created for status reporting");
        assert!(!session.active);
        assert!(session.start.is_none());
        assert!(
            session
                .status
                .as_deref()
                .is_some_and(|status| status.contains("Copies B/W"))
        );
    }

    #[test]
    fn start_recording_allows_unconfigured_categories_to_remain_empty() {
        let mut app = test_app();
        let record = printer_record(PrinterStatus::Online, Some(123));
        let printer_id = record.id.clone();
        let prints_bw_oid = Oid::from_slice(&[1, 3, 6, 1, 4, 1, 999, 2]);

        app.replace_printers(vec![record]);
        app.selected_printer = Some(printer_id.clone());
        app.recording_oids = RecordingOidSettings {
            copies_bw_input: String::new(),
            copies_color_input: String::new(),
            prints_bw_input: format_oid_list(&[prints_bw_oid.clone()]),
            prints_color_input: String::new(),
        };
        app.poll_states.insert(
            printer_id.clone(),
            SnmpPollStatus::Ok {
                received_at: 123,
                varbinds: vec![SnmpVarBind {
                    oid: prints_bw_oid,
                    value: printcountpay_core::SnmpValue::Counter32(456),
                }],
            },
        );

        app.start_recording();

        let session = app
            .recording_sessions
            .get(&printer_id)
            .expect("recording session should exist");
        assert!(session.active);
        assert_eq!(
            session
                .start
                .as_ref()
                .and_then(|snapshot| snapshot.bw_printer),
            Some(456)
        );
        assert_eq!(session.edits.prints_bw.start_input, "456");
    }

    #[test]
    fn sync_statistics_from_poll_state_stores_total_counter_once_per_bucket() {
        let mut app = test_app();
        let root = temp_test_dir("statistics-poll");
        fs::create_dir_all(&root).expect("create temp root");
        app.statistics_path = root.join("statistics.ron").to_string_lossy().to_string();

        let record = printer_record(PrinterStatus::Online, Some(123));
        let printer_id = record.id.clone();
        let total_oid = Oid::from_slice(&PRT_MARKER_LIFECOUNT_3);
        app.replace_printers(vec![record]);

        app.poll_states.insert(
            printer_id.clone(),
            SnmpPollStatus::Ok {
                received_at: 900,
                varbinds: vec![SnmpVarBind {
                    oid: total_oid.clone(),
                    value: printcountpay_core::SnmpValue::Counter32(111),
                }],
            },
        );
        app.sync_statistics_from_poll_state(&printer_id);

        app.poll_states.insert(
            printer_id.clone(),
            SnmpPollStatus::Ok {
                received_at: 905,
                varbinds: vec![SnmpVarBind {
                    oid: total_oid,
                    value: printcountpay_core::SnmpValue::Counter32(222),
                }],
            },
        );
        app.sync_statistics_from_poll_state(&printer_id);

        let entry = app
            .statistics_store
            .entry(&printer_id)
            .expect("statistics entry");
        assert_eq!(entry.poll_samples.len(), 1);
        assert_eq!(entry.poll_samples[0].metrics.len(), 1);
        assert_eq!(entry.poll_samples[0].metrics[0].label, "Clicks: Total");
        assert_eq!(entry.poll_samples[0].metrics[0].value, 111);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn statistics_series_selection_survives_printer_switches() {
        let mut app = test_app();
        let printer_a = printer_record_with_id("printer-a");
        let printer_b = printer_record_with_id("printer-b");
        let captured_at = app.statistics_time_window().start_inclusive;
        let bw_key = StatisticsPollMetric::new("1.2.3", "Clicks: B/W", 0).series_key;
        let color_key = StatisticsPollMetric::new("1.2.4", "Clicks: Color", 0).series_key;

        app.replace_printers(vec![printer_a.clone(), printer_b.clone()]);
        append_poll_sample(
            &mut app.statistics_store,
            &printer_a.id,
            captured_at,
            vec![
                StatisticsPollMetric::new("1.2.3", "Clicks: B/W", 10),
                StatisticsPollMetric::new("1.2.4", "Clicks: Color", 5),
            ],
        );
        append_poll_sample(
            &mut app.statistics_store,
            &printer_b.id,
            captured_at,
            vec![StatisticsPollMetric::new("1.2.4", "Clicks: Color", 7)],
        );

        app.statistics_selected_printers = HashSet::from([printer_a.id.clone()]);
        app.sync_statistics_visible_series();
        assert!(app.statistics_visible_series.contains(&bw_key));

        for series_key in app
            .statistics_visible_series
            .clone()
            .into_iter()
            .filter(|series_key| series_key != &bw_key)
            .collect::<Vec<_>>()
        {
            app.toggle_statistics_series(series_key);
        }
        assert_eq!(
            app.statistics_visible_series,
            HashSet::from([bw_key.clone()])
        );

        app.statistics_selected_printers = HashSet::from([printer_b.id.clone()]);
        app.sync_statistics_visible_series();
        assert_eq!(
            app.statistics_visible_series,
            HashSet::from([bw_key.clone()])
        );

        app.statistics_selected_printers =
            HashSet::from([printer_a.id.clone(), printer_b.id.clone()]);
        app.sync_statistics_visible_series();
        assert!(app.statistics_visible_series.contains(&bw_key));
        assert!(!app.statistics_visible_series.contains(&color_key));
    }

    #[test]
    fn statistics_series_selection_stays_empty_after_hiding_everything() {
        let mut app = test_app();
        let printer_a = printer_record_with_id("printer-a");
        let printer_b = printer_record_with_id("printer-b");
        let captured_at = app.statistics_time_window().start_inclusive;

        app.replace_printers(vec![printer_a.clone(), printer_b.clone()]);
        append_poll_sample(
            &mut app.statistics_store,
            &printer_a.id,
            captured_at,
            vec![
                StatisticsPollMetric::new("1.2.3", "Clicks: B/W", 10),
                StatisticsPollMetric::new("1.2.4", "Clicks: Color", 5),
            ],
        );
        append_poll_sample(
            &mut app.statistics_store,
            &printer_b.id,
            captured_at,
            vec![StatisticsPollMetric::new("1.2.4", "Clicks: Color", 7)],
        );

        app.statistics_selected_printers = HashSet::from([printer_a.id.clone()]);
        app.sync_statistics_visible_series();

        for series_key in app
            .statistics_visible_series
            .clone()
            .into_iter()
            .collect::<Vec<_>>()
        {
            app.toggle_statistics_series(series_key);
        }
        assert!(app.statistics_visible_series.is_empty());

        app.statistics_selected_printers = HashSet::from([printer_b.id]);
        app.sync_statistics_visible_series();
        assert!(app.statistics_visible_series.is_empty());
    }

    #[test]
    fn stop_recording_does_not_store_income_statistics_sample() {
        let mut app = test_app();
        let root = temp_test_dir("statistics-euro");
        fs::create_dir_all(&root).expect("create temp root");
        app.statistics_path = root.join("statistics.ron").to_string_lossy().to_string();

        let record = printer_record(PrinterStatus::Online, Some(123));
        let printer_id = record.id.clone();
        app.replace_printers(vec![record]);
        app.selected_printer = Some(printer_id.clone());
        app.pricing.bw_first_input = "0.25".to_string();
        app.pricing.bw_next_input = "0.10".to_string();
        app.pricing.bw_rest_input = "0.06".to_string();
        app.pricing.color_input = "0.50".to_string();
        app.recording_sessions.insert(
            printer_id.clone(),
            RecordingSession {
                active: true,
                start: Some(RecordingSnapshot {
                    received_at: 100,
                    bw_printer: Some(100),
                    bw_copier: None,
                    color_printer: None,
                    color_copier: None,
                }),
                end: None,
                status: None,
                end_fields_unlocked: false,
                updated_at_millis: 0,
                manual_state_changed_at_millis: 0,
                edits: RecordingEdits::default(),
            },
        );
        app.poll_states.insert(
            printer_id.clone(),
            SnmpPollStatus::Ok {
                received_at: 200,
                varbinds: vec![SnmpVarBind {
                    oid: Oid::from_slice(&RICOH_BW_PRINTER_COUNT_OID),
                    value: printcountpay_core::SnmpValue::Counter32(110),
                }],
            },
        );

        app.stop_recording();

        assert!(app.statistics_store.entry(&printer_id).is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn apply_statistics_sync_merges_remote_store_and_persists_result() {
        let mut app = test_app();
        let root = temp_test_dir("statistics-sync");
        fs::create_dir_all(&root).expect("create temp root");
        let path = root.join("statistics.ron");
        app.statistics_path = path.to_string_lossy().to_string();

        let printer_id = PrinterId::new("printer-a");
        let mut local_store = StatisticsStore::default();
        append_poll_sample(
            &mut local_store,
            &printer_id,
            900,
            vec![StatisticsPollMetric::new("1.2.3", "Clicks: Total", 10)],
        );
        app.statistics_store = local_store;
        app.persist_statistics_store_with_logging();

        let mut remote_store = StatisticsStore::default();
        append_poll_sample(
            &mut remote_store,
            &printer_id,
            901,
            vec![StatisticsPollMetric::new("1.2.3", "Clicks: Total", 11)],
        );
        append_poll_sample(
            &mut remote_store,
            &printer_id,
            3_600,
            vec![StatisticsPollMetric::new("1.2.4", "Clicks: Total", 20)],
        );
        app.apply_statistics_sync(sync::StatisticsSyncPayload {
            latest_data_at: 3_600,
            store: remote_store,
        });

        let entry = app
            .statistics_store
            .entry(&printer_id)
            .expect("statistics entry");
        assert_eq!(entry.poll_samples.len(), 2);
        assert_eq!(entry.poll_samples[0].captured_at, 901);
        assert_eq!(entry.poll_samples[0].metrics[0].value, 11);
        assert_eq!(entry.poll_samples[1].captured_at, 3_600);
        assert!(entry.euro_samples.is_empty());

        let persisted = read_statistics_store(&path);
        assert_eq!(persisted, app.statistics_store);

        let _ = fs::remove_dir_all(root);
    }
}
