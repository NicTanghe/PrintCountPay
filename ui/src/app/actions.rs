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

    fn load_manual_pricing_from_path(&mut self) {
        let path = self.manual_pricing_path.trim().to_string();
        if path.is_empty() {
            self.manual_pricing_status = Some("Load failed: path is empty.".to_string());
            return;
        }

        match fs::read_to_string(&path) {
            Ok(contents) => match from_str::<ManualPricingSettings>(&contents) {
                Ok(mut settings) => {
                    settings.normalize();
                    self.manual_pricing = settings;
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

    fn save_manual_pricing_to_path(&mut self) {
        let path = self.manual_pricing_path.trim().to_string();
        if path.is_empty() {
            self.manual_pricing_status = Some("Save failed: path is empty.".to_string());
            return;
        }

        self.manual_pricing.normalize();

        let config = PrettyConfig::new();
        match to_string_pretty(&self.manual_pricing, config) {
            Ok(contents) => match fs::write(&path, contents) {
                Ok(()) => {
                    self.manual_pricing_status =
                        Some(format!("Saved manual pricing to {path}."));
                }
                Err(error) => {
                    self.manual_pricing_status = Some(format!("Save failed: {error}"));
                }
            },
            Err(error) => {
                self.manual_pricing_status = Some(format!("Save failed: {error}"));
            }
        }
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
                self.discovery_status = Some(format!(
                    "Last error: {} ({})",
                    error.summary, error.detail
                ));
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
        let host = record
            .snmp_address
            .as_ref()
            .map(|addr| addr.host.as_str());

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

    fn delete_selected_printer(&mut self) {
        if self.active_tab != Tab::Printers || self.manual_pricing_selected {
            return;
        }

        let Some(selected) = self.selected_printer.clone() else {
            return;
        };

        let Some(index) = self.printers.iter().position(|record| record.id == selected) else {
            self.selected_printer = None;
            return;
        };

        self.printers.remove(index);
        self.poll_states.remove(&selected);
        self.poll_in_flight.remove(&selected);
        self.recording_sessions.remove(&selected);

        if self.printers.is_empty() {
            self.selected_printer = None;
            return;
        }

        let new_index = index.min(self.printers.len() - 1);
        self.selected_printer = Some(self.printers[new_index].id.clone());
    }

    fn printer_matches_host(printer: &PrinterRecord, host: &str) -> bool {
        printer
            .snmp_address
            .as_ref()
            .map(|addr| addr.host.as_str())
            == Some(host)
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

        let existing = record
            .model
            .as_deref()
            .map(str::trim)
            .unwrap_or("");
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
            && !sys_descr.is_empty() && existing == sys_descr
        {
            should_replace = true;
        }
        if let Some(host) = record.ip_or_hostname.as_deref().map(str::trim)
            && !host.is_empty() && existing == host
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
                    self.printers_status = Some(format!(
                        "Saved {} printers to {path}.",
                        self.printers.len()
                    ));
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
                sys_descr = varbind_text_value(&response.varbinds, &Oid::from_slice(&SYS_DESCR_OID));
                sys_object_id = varbind_text_value(
                    &response.varbinds,
                    &Oid::from_slice(&SYS_OBJECT_ID_OID),
                );
                allow_override = printer_name.is_some() || sys_name.is_some() || sys_descr.is_some();
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
            self.apply_printer_name_fallback(&printer_id, name, allow_override, sys_descr.as_deref());
        }

        if let Some(record) = self.printers.iter_mut().find(|record| record.id == printer_id) {
            record.sys_object_id = sys_object_id;
            record.sys_descr = sys_descr.clone();
            record.status = status;
            if let Some(last_seen) = last_seen {
                record.last_seen = Some(last_seen);
            }
        }

        let printer_id_clone = printer_id.clone();
        self.poll_states.insert(printer_id, state);
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
            if let Some(record) = self.printers.iter_mut().find(|record| record.id == printer_id) {
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
        let record = self.printers.iter().find(|record| &record.id == printer_id)?;
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
            }
            Err(error) => {
                session.status = Some(format!("Start failed: {error}"));
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
            }
            Err(error) => {
                session.status = Some(format!("Stop failed: {error}"));
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

        let (name, address) = match self
            .printers
            .iter()
            .find(|record| record.id == printer_id)
        {
            Some(record) => {
                let name = record.model.as_deref().unwrap_or("Unknown name").to_string();
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

    fn snapshot_for_printer(
        &self,
        printer_id: &PrinterId,
    ) -> Result<RecordingSnapshot, String> {
        let Some(state) = self.poll_states.get(printer_id) else {
            return Err("No poll data yet.".to_string());
        };

        match state {
            SnmpPollStatus::Ok {
                received_at,
                varbinds,
            } => Ok(self.build_recording_snapshot(*received_at, varbinds)),
            SnmpPollStatus::Error { summary, detail, .. } => {
                Err(format!("{summary} ({detail})"))
            }
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

        if self
            .active_profile
            .as_ref()
            .map(|profile| profile.id())
            == Some(profile_id.clone())
        {
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
                    self.oids_status = Some(format!("Applied mapping (profile not synced: {error})"));
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
                    let mut request =
                        SnmpWalkRequest::new(address.clone(), Oid::from_slice(root))
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
        }
    }

    fn apply_shared_state(&mut self, snapshot: SharedState) {
        let selected = self.selected_printer.clone();
        let SharedState {
            revision,
            printers,
            poll_states,
            recording_sessions,
            pricing,
        } = snapshot;

        self.printers = printers;
        self.pricing = pricing;
        self.poll_states = poll_states
            .into_iter()
            .map(|entry| (entry.printer_id, entry.state))
            .collect();

        for record in &self.printers {
            self.poll_states
                .entry(record.id.clone())
                .or_insert(SnmpPollStatus::Idle);
        }

        let known_ids: HashSet<PrinterId> = self.printers.iter().map(|record| record.id.clone()).collect();
        self.recording_sessions = recording_sessions
            .into_iter()
            .filter(|entry| known_ids.contains(&entry.printer_id))
            .map(|entry| {
                let local_unlock_state = self
                    .recording_sessions
                    .get(&entry.printer_id)
                    .map(|session| session.end_fields_unlocked)
                    .unwrap_or(entry.session.end_fields_unlocked);
                let mut session = entry.session;
                session.end_fields_unlocked = local_unlock_state;
                (entry.printer_id, session)
            })
            .collect();
        self.poll_in_flight.retain(|printer_id| known_ids.contains(printer_id));

        self.selected_printer = selected.filter(|printer_id| known_ids.contains(printer_id));
        if let Some(selected) = self.selected_printer.clone() {
            self.apply_profile_for_printer(&selected, None);
        } else {
            self.clear_active_profile();
        }

        self.last_shared_state = self.build_shared_state(revision);
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
        });

        assert_eq!(
            app.recording_sessions
                .get(&printer_id)
                .map(|session| session.end_fields_unlocked),
            Some(true)
        );
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
        assert_eq!(session.start.as_ref().and_then(|snapshot| snapshot.bw_printer), Some(456));
        assert_eq!(session.edits.prints_bw.start_input, "456");
    }
}
