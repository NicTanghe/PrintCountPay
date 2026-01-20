impl PrintCountApp {
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
        if let Some(selected) = &self.selected_printer {
            output.push_str(&format!("Selected printer: {}\n", selected));
        }
        output.push_str(&format!("Mock SNMP entries: {}\n", self.mock_snmp_count));
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
            queue.push_back(SnmpAddress::with_default_port(ip.to_string()));
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
                        Err(error) => DiscoveryOutcome::Error(SnmpErrorInfo {
                            summary: error.user_summary(),
                            detail: error.technical_detail(),
                        }),
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

        if let Some(existing) = self.printers.iter_mut().find(|printer| {
            printer
                .snmp_address
                .as_ref()
                .map(|addr| addr.host.as_str())
                == host
        }) {
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
        if self.active_tab != Tab::Printers {
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

    fn find_printer_by_host_mut(&mut self, host: &str) -> Option<&mut PrinterRecord> {
        self.printers.iter_mut().find(|printer| {
            printer
                .snmp_address
                .as_ref()
                .map(|addr| addr.host.as_str())
                == Some(host)
                || printer.ip_or_hostname.as_deref() == Some(host)
        })
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
        if let Some(sys_descr) = sys_descr.map(str::trim) {
            if !sys_descr.is_empty() && existing == sys_descr {
                should_replace = true;
            }
        }
        if let Some(host) = record.ip_or_hostname.as_deref().map(str::trim) {
            if !host.is_empty() && existing == host {
                should_replace = true;
            }
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
    }

    fn poll_selected_printer(&mut self) -> Command<Message> {
        let Some(printer_id) = self.selected_printer.clone() else {
            return Command::none();
        };

        if self.poll_in_flight.contains(&printer_id) {
            return Command::none();
        }

        let Some(record) = self.printers.iter().find(|record| record.id == printer_id) else {
            return Command::none();
        };

        let now = now_epoch_seconds();
        let Some(address) = record.snmp_address.clone() else {
            self.poll_states.insert(
                printer_id,
                SnmpPollStatus::Error {
                    received_at: now,
                    summary: "Missing SNMP address".to_string(),
                    detail: "Printer has no SNMP address configured.".to_string(),
                },
            );
            return Command::none();
        };

        let mut request = SnmpRequest::new(
            address,
            snmp_oids(&self.counter_oids, &self.recording_oids),
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
                    Err(error) => Err(SnmpErrorInfo {
                        summary: error.user_summary(),
                        detail: error.technical_detail(),
                    }),
                }
            },
            move |result| Message::SnmpPolled { printer_id, result },
        )
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

        let snapshot_result = self.snapshot_for_printer(&printer_id);
        let session = self
            .recording_sessions
            .entry(printer_id.clone())
            .or_default();

        match snapshot_result {
            Ok(snapshot) => {
                session.active = true;
                session.start = Some(snapshot.clone());
                session.end = None;
                session.edits.apply_start_snapshot(&snapshot);
                session.status = Some(format!(
                    "Recording started at {}.",
                    snapshot.received_at
                ));
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
                session.status = Some(format!(
                    "Recording stopped at {}.",
                    snapshot.received_at
                ));
            }
            Err(error) => {
                session.status = Some(format!("Stop failed: {error}"));
            }
        }
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
        let copies_bw_oids =
            parse_oid_list(&self.recording_oids.copies_bw_input).unwrap_or_default();
        let copies_color_oids =
            parse_oid_list(&self.recording_oids.copies_color_input).unwrap_or_default();
        let prints_bw_oids =
            parse_oid_list(&self.recording_oids.prints_bw_input).unwrap_or_default();
        let prints_color_oids =
            parse_oid_list(&self.recording_oids.prints_color_input).unwrap_or_default();

        let copies_bw_value = copies_bw_oids
            .iter()
            .find_map(|oid| extract_counter_u64(varbinds, oid));
        let copies_color_value = copies_color_oids
            .iter()
            .find_map(|oid| extract_counter_u64(varbinds, oid));
        let prints_bw_value = prints_bw_oids
            .iter()
            .find_map(|oid| extract_counter_u64(varbinds, oid));
        let prints_color_value = prints_color_oids
            .iter()
            .find_map(|oid| extract_counter_u64(varbinds, oid));

        RecordingSnapshot {
            received_at,
            bw_printer: prints_bw_value,
            bw_copier: copies_bw_value,
            color_printer: prints_color_value,
            color_copier: copies_color_value,
        }
    }

    fn sync_oid_inputs(&mut self) {
        self.recording_oids = recording_oids_from_counter_set(&self.counter_oids);
        self.oids_total_text = format_oid_list(&self.counter_oids.total);
    }

    fn apply_oid_inputs(&mut self) {
        match self.parse_oid_inputs() {
            Ok(set) => {
                self.counter_oids = set;
                self.oids_status = Some("Applied OID mapping.".to_string());
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
            Ok(contents) => match from_str::<CounterOidSet>(&contents) {
                Ok(set) => {
                    self.counter_oids = set;
                    self.sync_oid_inputs();
                    self.oids_status = Some(format!("Loaded OIDs from {path}."));
                }
                Err(error) => {
                    self.oids_status = Some(format!("Load failed: {error}"));
                }
            },
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
        match to_string_pretty(&self.counter_oids, config) {
            Ok(contents) => match fs::write(&path, contents) {
                Ok(()) => {
                    self.oids_status = Some(format!("Saved OIDs to {path}."));
                }
                Err(error) => {
                    self.oids_status = Some(format!("Save failed: {error}"));
                }
            },
            Err(error) => {
                self.oids_status = Some(format!("Save failed: {error}"));
            }
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
                            last_error = Some(SnmpErrorInfo {
                                summary: error.user_summary(),
                                detail: error.technical_detail(),
                            });
                        }
                    }
                }

                if varbinds.is_empty() {
                    Err(last_error.unwrap_or(SnmpErrorInfo {
                        summary: "Crawl failed.".to_string(),
                        detail: "No OIDs returned from crawl.".to_string(),
                    }))
                } else {
                    Ok(counter_oids_from_walk(&varbinds))
                }
            },
            Message::OidsCrawled,
        )
    }

    fn counter_oids_empty(&self) -> bool {
        self.counter_oids.bw.is_empty()
            && self.counter_oids.color.is_empty()
            && self.counter_oids.total.is_empty()
    }
}
