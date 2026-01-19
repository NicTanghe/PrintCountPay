impl PrintCountApp {
    fn tab_bar(&self) -> Element<'_, Message> {
        row![
            self.tab_button(Tab::Printers, "Printers"),
            self.tab_button(Tab::Debug, "Debug")
        ]
        .spacing(8)
        .align_items(Alignment::Center)
        .into()
    }

    fn tab_button(&self, tab: Tab, label: &str) -> Element<'_, Message> {
        let style = if self.active_tab == tab {
            theme::Button::Primary
        } else {
            theme::Button::Secondary
        };

        button(text(label))
            .style(style)
            .on_press(Message::SelectTab(tab))
            .into()
    }

    fn printer_tab_bar(&self) -> Element<'_, Message> {
        row![
            self.printer_tab_button(PrinterTab::Polling, "Polling"),
            self.printer_tab_button(PrinterTab::Recording, "Recording"),
            self.printer_tab_button(PrinterTab::Pricing, "Pricing"),
            self.printer_tab_button(PrinterTab::Oids, "SNMP OIDs"),
            self.printer_tab_button(PrinterTab::AddPrinters, "Discovery + Manual")
        ]
        .spacing(4)
        .align_items(Alignment::Center)
        .into()
    }

    fn printer_tab_button(&self, tab: PrinterTab, label: &str) -> Element<'_, Message> {
        let style = theme::Button::custom(FirefoxTabStyle {
            active: self.printer_tab == tab,
        });

        button(text(label))
            .padding([6, 12])
            .style(style)
            .on_press(Message::SelectPrinterTab(tab))
            .into()
    }

    fn discovery_controls_view(&self) -> Element<'_, Message> {
        let cidr_input = text_input("192.168.129.1/24", &self.discovery_cidr)
            .on_input(Message::DiscoveryCidrChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);
        let community_input = text_input("public", &self.discovery_community)
            .on_input(Message::DiscoveryCommunityChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        let action_button = if self.discovery_active {
            button("Stop").on_press(Message::StopDiscovery)
        } else {
            button("Start").on_press(Message::StartDiscovery)
        };

        let status = self
            .discovery_status
            .as_deref()
            .unwrap_or("Idle - ready to scan.");
        let progress = if self.discovery_total > 0 {
            format!(
                "Scanned {}/{} | Found {} | Errors {}",
                self.discovery_scanned,
                self.discovery_total,
                self.discovery_found,
                self.discovery_errors
            )
        } else {
            "Scanned 0/0 | Found 0 | Errors 0".to_string()
        };

        let content = column![
            text("Discovery")
                .size(16)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            column![
                text("CIDR range")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                cidr_input,
            ]
            .spacing(4),
            column![
                text("Community")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                community_input,
            ]
            .spacing(4),
            row![action_button]
                .spacing(8)
                .align_items(Alignment::Center),
            text(status)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            text(progress)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(6);

        container(content)
            .padding(8)
            .style(theme::Container::Box)
            .into()
    }

    fn manual_printer_controls_view(&self) -> Element<'_, Message> {
        let name_input = text_input("Front Office", &self.manual_name)
            .on_input(Message::ManualNameChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);
        let host_input = text_input("192.168.1.50", &self.manual_host)
            .on_input(Message::ManualHostChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);
        let port_input = text_input("161", &self.manual_port)
            .on_input(Message::ManualPortChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);
        let community_input = text_input("public", &self.manual_community)
            .on_input(Message::ManualCommunityChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        let status = self.manual_status.as_deref().unwrap_or("Ready.");

        let content = column![
            text("Manual add")
                .size(16)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            column![
                text("Name")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                name_input,
            ]
            .spacing(4),
            column![
                text("Host or IP")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                host_input,
            ]
            .spacing(4),
            column![
                text("Port")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                port_input,
            ]
            .spacing(4),
            column![
                text("Community")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                community_input,
            ]
            .spacing(4),
            row![button("Add printer").on_press(Message::AddManualPrinter)]
                .spacing(8)
                .align_items(Alignment::Center),
            text(format!("Status: {status}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(6);

        container(content)
            .padding(8)
            .style(theme::Container::Box)
            .into()
    }

    fn printer_storage_controls_view(&self) -> Element<'_, Message> {
        let status = self.printers_status.as_deref().unwrap_or("Ready.");
        let path_input = text_input("printers.ron", &self.printers_path)
            .on_input(Message::PrintersPathChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        let path_controls = row![
            path_input,
            button("Load").on_press(Message::LoadPrinters),
            button("Export").on_press(Message::SavePrinters),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let content = column![
            text("Printer list storage")
                .size(16)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            column![
                text("RON path")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                path_controls,
            ]
            .spacing(4),
            text(format!("Status: {status}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(6);

        container(content)
            .padding(8)
            .style(theme::Container::Box)
            .into()
    }

    fn printers_tab_view(&self) -> Element<'_, Message> {
        let list = self.printer_list_view();
        let details = self.printer_details_view();

        row![list, details]
            .spacing(16)
            .align_items(Alignment::Start)
            .into()
    }

    fn recording_tab_view(&self) -> Element<'_, Message> {
        let selected_id = self.selected_printer.as_ref();
        let selected_label = selected_id
            .and_then(|selected| {
                self.printers
                    .iter()
                    .find(|record| &record.id == selected)
                    .map(|record| {
                        record
                            .model
                            .as_deref()
                            .unwrap_or("Unknown name")
                            .to_string()
                    })
            })
            .unwrap_or_else(|| "No printer selected".to_string());

        let selected_id_label = selected_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "None".to_string());

        let session = selected_id
            .and_then(|id| self.recording_sessions.get(id))
            .cloned()
            .unwrap_or_default();

        let status = session.status.as_deref().unwrap_or("Ready.");
        let state_label = if session.active {
            "Recording active"
        } else {
            "Recording idle"
        };

        let controls_enabled = selected_id.is_some();
        let start_button = if !controls_enabled || session.active {
            button("Start recording").style(theme::Button::Secondary)
        } else {
            button("Start recording").on_press(Message::StartRecording)
        };
        let stop_button = if !controls_enabled || !session.active {
            button("Stop recording").style(theme::Button::Secondary)
        } else {
            button("Stop recording").on_press(Message::StopRecording)
        };

        let start_time = session
            .start
            .as_ref()
            .map(|snapshot| snapshot.received_at.to_string())
            .unwrap_or_else(|| "n/a".to_string());
        let end_time = session
            .end
            .as_ref()
            .map(|snapshot| snapshot.received_at.to_string())
            .unwrap_or_else(|| "n/a".to_string());

        let delta_section: Element<'_, Message> = if session.start.is_some() && session.end.is_some()
        {
            let copies_bw_start = category_start_value(&session, RecordingCategory::CopiesBw);
            let copies_bw_end = category_end_value(&session, RecordingCategory::CopiesBw);
            let copies_bw_delta = delta_value(copies_bw_start, copies_bw_end);

            let copies_color_start = category_start_value(&session, RecordingCategory::CopiesColor);
            let copies_color_end = category_end_value(&session, RecordingCategory::CopiesColor);
            let copies_color_delta = delta_value(copies_color_start, copies_color_end);

            let prints_bw_start = category_start_value(&session, RecordingCategory::PrintsBw);
            let prints_bw_end = category_end_value(&session, RecordingCategory::PrintsBw);
            let prints_bw_delta = delta_value(prints_bw_start, prints_bw_end);

            let prints_color_start = category_start_value(&session, RecordingCategory::PrintsColor);
            let prints_color_end = category_end_value(&session, RecordingCategory::PrintsColor);
            let prints_color_delta = delta_value(prints_color_start, prints_color_end);

            let include_copies_bw = session.edits.copies_bw.include_in_price;
            let include_copies_color = session.edits.copies_color.include_in_price;
            let include_prints_bw = session.edits.prints_bw.include_in_price;
            let include_prints_color = session.edits.prints_color.include_in_price;

            let start_bw_total = sum_two(copies_bw_start, prints_bw_start);
            let end_bw_total = sum_two(copies_bw_end, prints_bw_end);
            let total_bw_delta = delta_value(start_bw_total, end_bw_total);

            let start_color_total = sum_two(copies_color_start, prints_color_start);
            let end_color_total = sum_two(copies_color_end, prints_color_end);
            let total_color_delta = delta_value(start_color_total, end_color_total);

            let bw_delta = sum_optional_included([
                (include_copies_bw, copies_bw_delta),
                (include_prints_bw, prints_bw_delta),
            ]);
            let color_delta = sum_optional_included([
                (include_copies_color, copies_color_delta),
                (include_prints_color, prints_color_delta),
            ]);

            let bw_pricing = bw_pricing_from_settings(&self.pricing);
            let color_price = color_price_from_settings(&self.pricing);
            let bw_cost_raw = match bw_delta {
                Some(0) => Some(0),
                Some(count) => bw_pricing.map(|pricing| bw_cost_cents(count, pricing)),
                None => None,
            };
            let bw_cost_value = bw_cost_raw.map(|value| {
                if self.pricing.round_to_half_euro {
                    round_to_nearest_50_cents(value)
                } else {
                    value
                }
            });
            let color_cost_value = match color_delta {
                Some(0) => Some(0),
                Some(count) => color_price.map(|price| color_cost_cents(count, price)),
                None => None,
            };
            let subtotal_cents = match (bw_cost_value, color_cost_value) {
                (Some(bw), Some(color)) => Some(bw + color),
                _ => None,
            };
            let total_cents = subtotal_cents;
            let rounding_label = if self.pricing.round_to_half_euro {
                "B/W rounded to nearest 0.50 EUR"
            } else {
                "No rounding applied"
            };

            column![
                self.recording_table_header(),
                self.recording_table_row_editable(
                    RecordingCategory::CopiesBw,
                    "Copies B/W",
                    &session.edits.copies_bw.start_input,
                    &session.edits.copies_bw.end_input,
                    copies_bw_delta,
                    include_copies_bw,
                ),
                self.recording_table_row_editable(
                    RecordingCategory::CopiesColor,
                    "Copies color",
                    &session.edits.copies_color.start_input,
                    &session.edits.copies_color.end_input,
                    copies_color_delta,
                    include_copies_color,
                ),
                self.recording_table_row_editable(
                    RecordingCategory::PrintsBw,
                    "Prints B/W",
                    &session.edits.prints_bw.start_input,
                    &session.edits.prints_bw.end_input,
                    prints_bw_delta,
                    include_prints_bw,
                ),
                self.recording_table_row_editable(
                    RecordingCategory::PrintsColor,
                    "Prints color",
                    &session.edits.prints_color.start_input,
                    &session.edits.prints_color.end_input,
                    prints_color_delta,
                    include_prints_color,
                ),
                Rule::horizontal(1),
                self.recording_table_row(
                    "Total B/W",
                    start_bw_total,
                    end_bw_total,
                    total_bw_delta,
                ),
                self.recording_table_row(
                    "Total color",
                    start_color_total,
                    end_color_total,
                    total_color_delta,
                ),
                Rule::horizontal(1),
                self.value_line("Total price", total_cents.map(format_cents)),
                text(rounding_label)
                    .size(11)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            ]
            .spacing(6)
            .into()
        } else {
            text("No completed recording yet.")
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a)))
                .into()
        };

        let content = column![
            text(format!("Selected printer: {selected_label}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            text(format!("Recording printer ID: {selected_id_label}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            text(state_label)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            row![start_button, stop_button]
                .spacing(8)
                .align_items(Alignment::Center),
            text(format!("Start snapshot: {start_time}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            text(format!("End snapshot: {end_time}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            text(format!("Status: {status}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            delta_section
        ]
        .spacing(12);

        container(content)
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }

    fn pricing_tab_view(&self) -> Element<'_, Message> {
        let bw_section = column![
            text("Black/white pricing")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            self.pricing_input(
                "First 5 pages (EUR)",
                "0.25",
                &self.pricing.bw_first_input,
                Message::PricingBwFirstChanged,
            ),
            self.pricing_input(
                "Next 5 pages (EUR)",
                "0.10",
                &self.pricing.bw_next_input,
                Message::PricingBwNextChanged,
            ),
            self.pricing_input(
                "Rest (EUR)",
                "0.06",
                &self.pricing.bw_rest_input,
                Message::PricingBwRestChanged,
            ),
        ]
        .spacing(6);

        let color_section = column![
            text("Color pricing")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            self.pricing_input(
                "Per page (EUR)",
                "0.50",
                &self.pricing.color_input,
                Message::PricingColorChanged,
            ),
        ]
        .spacing(6);

        let rounding_toggle =
            checkbox("Round B/W to nearest 0.50 EUR", self.pricing.round_to_half_euro)
                .on_toggle(Message::PricingRoundChanged)
                .size(12);

        let hint = text("Used for recording totals. Decimals accept . or ,")
            .size(11)
            .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a)));

        let content = column![bw_section, color_section, rounding_toggle, hint].spacing(12);

        container(content)
            .padding(12)
            .width(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }

    fn printer_list_view(&self) -> Element<'_, Message> {
        let mut list_items = column![].spacing(6);

        if self.printers.is_empty() {
            list_items = list_items.push(
                text("No printers discovered or added yet.")
                    .size(14)
                    .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            );
        } else {
            for record in &self.printers {
                list_items = list_items.push(self.printer_row(record));
            }
        }

        let content = column![
            self.printer_storage_controls_view(),
            text("Printers")
                .size(20)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            text("Discovery and manual entries appear here.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            list_items,
        ]
        .spacing(12);

        let scroll = scrollable(content)
            .height(Length::Fill)
            .width(Length::Fill);

        container(scroll)
            .padding(12)
            .width(Length::FillPortion(1))
            .height(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }

    fn printer_row(&self, record: &PrinterRecord) -> Element<'_, Message> {
        let is_selected = self.selected_printer.as_ref() == Some(&record.id);
        let address = record
            .ip_or_hostname
            .as_deref()
            .or_else(|| record.snmp_address.as_ref().map(|addr| addr.host.as_str()))
            .unwrap_or("unknown host");
        let name = record.model.as_deref().unwrap_or("Unknown name");
        let status = status_label(record.status);

        let content = column![
            text(name)
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37))),
            text(address)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            text(status)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(2);

        let style = if is_selected {
            theme::Button::Primary
        } else {
            theme::Button::Secondary
        };

        button(content)
            .style(style)
            .width(Length::Fill)
            .on_press(Message::SelectPrinter(record.id.clone()))
            .into()
    }

    fn printer_details_view(&self) -> Element<'_, Message> {
        let selected_id = self.selected_printer.as_ref();
        let record = selected_id.and_then(|selected| {
            self.printers.iter().find(|record| &record.id == selected)
        });
        let selection_missing = selected_id.is_some() && record.is_none();

        let header = match self.printer_tab {
            PrinterTab::AddPrinters => column![
                text("Add printers")
                    .size(20)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                text("Run discovery or add a printer manually.")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            ]
            .spacing(4),
            _ => {
                let title = match self.printer_tab {
                    PrinterTab::Recording => "Recording",
                    PrinterTab::Pricing => "Pricing",
                    _ => "Printer details",
                };
                let mut content = column![text(title)
                    .size(20)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12)))]
                .spacing(4);

                if let Some(record) = record {
                    let address = record
                        .snmp_address
                        .as_ref()
                        .map(|addr| addr.to_string())
                        .unwrap_or_else(|| "Not set".to_string());
                    let name = record.model.as_deref().unwrap_or("Unknown name");
                    content = content.push(
                        text(format!("ID: {}", record.id))
                            .size(13)
                            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    );
                    content = content.push(
                        text(format!("Name: {}", name))
                            .size(13)
                            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    );
                    content = content.push(
                        text(format!("Address: {}", address))
                            .size(13)
                            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    );
                } else if selection_missing {
                    content = content.push(
                        text("Selected printer not found.")
                            .size(13)
                            .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
                    );
                }

                content
            }
        };

        let body = match self.printer_tab {
            PrinterTab::Polling => {
                if let Some(record) = record {
                    let in_flight = self.poll_in_flight.contains(&record.id);
                    let state = self
                        .poll_states
                        .get(&record.id)
                        .cloned()
                        .unwrap_or(SnmpPollStatus::Idle);
                    self.printer_poll_view(&state, in_flight)
                } else if selection_missing {
                    self.empty_printer_tab_view("Selected printer not found.")
                } else {
                    self.empty_printer_tab_view("Select a printer to start polling.")
                }
            }
            PrinterTab::Oids => {
                if let Some(record) = record {
                    self.printer_oids_view(record)
                } else if selection_missing {
                    self.empty_printer_tab_view("Selected printer not found.")
                } else {
                    self.empty_printer_tab_view("Select a printer to edit OIDs.")
                }
            }
            PrinterTab::Recording => self.recording_tab_view(),
            PrinterTab::Pricing => self.pricing_tab_view(),
            PrinterTab::AddPrinters => self.printer_add_printers_view(),
        };

        let content = column![self.printer_tab_bar(), header, body].spacing(12);

        container(content)
            .padding(12)
            .width(Length::FillPortion(2))
            .height(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }

    fn printer_add_printers_view(&self) -> Element<'_, Message> {
        column![
            self.discovery_controls_view(),
            self.manual_printer_controls_view(),
        ]
        .spacing(12)
        .into()
    }

    fn empty_printer_tab_view(&self, message: &str) -> Element<'_, Message> {
        text(message)
            .size(14)
            .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a)))
            .into()
    }

    fn printer_poll_view(&self, state: &SnmpPollStatus, in_flight: bool) -> Element<'_, Message> {
        let content = column![
            text("Polling every 5 seconds")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            self.poll_state_view(state, in_flight),
            self.counters_view(state, in_flight),
            self.poll_export_controls_view(),
        ]
        .spacing(8);

        content.into()
    }

    fn printer_oids_view(&self, record: &PrinterRecord) -> Element<'_, Message> {
        let status = self.oids_status.as_deref().unwrap_or("No changes yet.");
        let address = record
            .snmp_address
            .as_ref()
            .map(|addr| addr.to_string())
            .unwrap_or_else(|| "Not set".to_string());

        let path_input = text_input("counter_oids.ron", &self.oids_path)
            .on_input(Message::OidsPathChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        let path_controls = row![
            path_input,
            button("Load").on_press(Message::LoadOids),
            button("Save").on_press(Message::SaveOids),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let manual_inputs = column![
            self.oids_input(
                "B/W OIDs",
                "1.3.6.1.2.1.43.10.2.1.4.1.1",
                &self.oids_bw_text,
                Message::OidsBwChanged,
            ),
            self.oids_input(
                "Color OIDs",
                "1.3.6.1.2.1.43.10.2.1.4.1.2",
                &self.oids_color_text,
                Message::OidsColorChanged,
            ),
            self.oids_input(
                "Total OIDs",
                "1.3.6.1.2.1.43.10.2.1.4.1.3",
                &self.oids_total_text,
                Message::OidsTotalChanged,
            ),
        ]
        .spacing(8);

        let crawl_label = if self.oids_crawl_in_flight {
            "Crawling..."
        } else {
            "Crawl from printer"
        };

        let crawl_button = if self.oids_crawl_in_flight {
            button(crawl_label).style(theme::Button::Secondary)
        } else {
            button(crawl_label).on_press(Message::CrawlOids)
        };

        let actions = row![button("Apply mapping").on_press(Message::ApplyOids), crawl_button]
            .spacing(8)
            .align_items(Alignment::Center);

        let content = column![
            text("Counter OID mapping")
                .size(18)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            text("Enter dotted OIDs separated by commas or spaces.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            column![
                text("RON path")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                path_controls,
            ]
            .spacing(4),
            manual_inputs,
            actions,
            text(format!("Status: {status}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            text(format!("Crawl target: {address}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            text(
                "Crawl roots: 1.3.6.1.2.1.43, 1.3.6.1.4.1.367, 1.3.6.1.4.1.367.3.2.1.2.19, 1.3.6.1.4.1.367.3.2.1.2.24",
            )
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(8);

        content.into()
    }

    fn pricing_input(
        &self,
        label: &str,
        placeholder: &str,
        value: &str,
        on_change: fn(String) -> Message,
    ) -> Element<'_, Message> {
        let input = text_input(placeholder, value)
            .on_input(on_change)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        column![
            text(label)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
            input
        ]
        .spacing(4)
        .into()
    }

    fn oids_input(
        &self,
        label: &str,
        placeholder: &str,
        value: &str,
        on_change: fn(String) -> Message,
    ) -> Element<'_, Message> {
        let input = text_input(placeholder, value)
            .on_input(on_change)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        column![
            text(label)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
            input
        ]
        .spacing(4)
        .into()
    }

    fn poll_state_view(&self, state: &SnmpPollStatus, in_flight: bool) -> Element<'_, Message> {
        let indicator = self.polling_indicator("Polling SNMP...", in_flight);
        let (last_poll, body): (String, Element<'_, Message>) = match state {
            SnmpPollStatus::Idle => (
                "Last poll: n/a".to_string(),
                text("Waiting for next poll.")
                    .size(14)
                    .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a)))
                    .into(),
            ),
            SnmpPollStatus::Ok {
                received_at,
                varbinds,
            } => {
                let total_varbinds = varbinds.len();
                let shown_varbinds = total_varbinds.min(MAX_VARBINDS_SHOWN);
                let mut rows = column![].spacing(4);
                if varbinds.is_empty() {
                    rows = rows.push(
                        text("No varbinds returned.")
                            .size(13)
                            .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
                    );
                } else {
                    for varbind in varbinds.iter().take(MAX_VARBINDS_SHOWN) {
                        rows = rows.push(
                            text(format!("{} = {}", varbind.oid, varbind.value))
                                .size(13)
                                .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37))),
                        );
                    }
                    if total_varbinds > shown_varbinds {
                        rows = rows.push(
                            text(format!(
                                "Showing {shown_varbinds} of {total_varbinds} varbinds."
                            ))
                            .size(12)
                            .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                        );
                    }
                }

                let list = scrollable(rows)
                    .height(Length::Fill)
                    .width(Length::Fill);

                let body = column![
                    text(format!("Varbinds: {shown_varbinds}/{total_varbinds}"))
                        .size(12)
                        .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                    list
                ]
                .spacing(6)
                .into();

                (format!("Last poll: {}", received_at), body)
            }
            SnmpPollStatus::Error {
                received_at,
                summary,
                detail,
            } => (
                format!("Last poll: {}", received_at),
                column![
                    text(format!("Error: {}", summary))
                        .size(13)
                        .style(theme::Text::Color(Color::from_rgb8(0xe0, 0x4f, 0x4f))),
                    text(detail)
                        .size(12)
                        .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                ]
                .spacing(4)
                .into(),
            ),
        };

        let header = row![
            text(last_poll)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a)))
                .width(Length::Fill),
            indicator,
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        column![header, body].spacing(6).into()
    }

    fn counters_view(&self, state: &SnmpPollStatus, in_flight: bool) -> Element<'_, Message> {
        let header = row![
            text("Counters")
                .size(18)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12)))
                .width(Length::Fill),
            self.polling_indicator("Polling counters...", in_flight),
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let body: Element<'_, Message> = match state {
            SnmpPollStatus::Ok {
                received_at,
                varbinds,
            } => {
                let resolution = resolve_counters(*received_at, &self.counter_oids, varbinds);
                let mut lines = column![
                    text("Printer counts")
                        .size(13)
                        .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    self.value_line(
                        "B/W printer",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_BW_PRINTER_COUNT_OID),
                        ),
                    ),
                    self.value_line(
                        "Color printer",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_COLOR_PRINTER_COUNT_OID),
                        ),
                    ),
                    text("Copier counts")
                        .size(13)
                        .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    self.value_line(
                        "B/W copier",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_BW_COPIER_COUNT_OID),
                        ),
                    ),
                    self.value_line(
                        "Color copier",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_COLOR_COPIER_COUNT_OID),
                        ),
                    ),
                    text("Click totals")
                        .size(13)
                        .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    self.counter_line("B/W clicks", resolution.snapshot.bw),
                    self.counter_line("Color clicks", resolution.snapshot.color),
                    self.counter_line("Total clicks", resolution.snapshot.total),
                    text("Toner levels")
                        .size(13)
                        .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    self.value_line(
                        "Black",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_TONER_BLACK_OID),
                        ),
                    ),
                    self.value_line(
                        "Cyan",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_TONER_CYAN_OID),
                        ),
                    ),
                    self.value_line(
                        "Magenta",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_TONER_MAGENTA_OID),
                        ),
                    ),
                    self.value_line(
                        "Yellow",
                        extract_value_string(
                            varbinds,
                            &Oid::from_slice(&RICOH_TONER_YELLOW_OID),
                        ),
                    ),
                ]
                .spacing(4);

                if self.counter_oids_empty() {
                    lines = lines.push(
                        text("Counter OIDs not mapped yet.")
                            .size(12)
                            .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                    );
                }

                if !resolution.warnings.is_empty() {
                    let warning_text = resolution
                        .warnings
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<String>>()
                        .join("; ");
                    lines = lines.push(
                        text(format!("Warnings: {warning_text}"))
                            .size(12)
                            .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                    );
                }

                lines.into()
            }
            SnmpPollStatus::Idle => text("No counter data yet.")
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a)))
                .into(),
            SnmpPollStatus::Error { .. } => text("Counters unavailable due to SNMP error.")
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0xe0, 0x4f, 0x4f)))
                .into(),
        };

        let content = column![header, body].spacing(6);

        container(content)
            .padding(8)
            .style(theme::Container::Box)
            .into()
    }

    fn polling_indicator(&self, label: &str, in_flight: bool) -> Element<'_, Message> {
        let color = if in_flight {
            Color::from_rgb8(0x3b, 0x82, 0xf6)
        } else {
            Color::TRANSPARENT
        };

        text(label)
            .size(12)
            .style(theme::Text::Color(color))
            .into()
    }

    fn poll_export_controls_view(&self) -> Element<'_, Message> {
        let status = self.poll_export_status.as_deref().unwrap_or("Ready.");
        let path_input = text_input("polling_export.txt", &self.poll_export_path)
            .on_input(Message::PollExportPathChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        let path_controls = row![
            path_input,
            button("Export").on_press(Message::ExportPollData),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let content = column![
            text("Poll export")
                .size(16)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            column![
                text("File path")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                path_controls,
            ]
            .spacing(4),
            text(format!("Status: {status}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(6);

        container(content)
            .padding(8)
            .style(theme::Container::Box)
            .into()
    }

    fn counter_line(&self, label: &str, value: Option<u64>) -> Element<'_, Message> {
        let value_text = value.map(|value| value.to_string()).unwrap_or_else(|| "N/A".to_string());

        let label = text(label)
            .size(13)
            .width(Length::Fill)
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));
        let value = text(value_text)
            .size(13)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)));

        row![label, value]
            .spacing(12)
            .align_items(Alignment::Center)
            .into()
    }

    fn value_line(&self, label: &str, value: Option<String>) -> Element<'_, Message> {
        let value_text = value.unwrap_or_else(|| "N/A".to_string());

        let label = text(label)
            .size(13)
            .width(Length::Fill)
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));
        let value = text(value_text)
            .size(13)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)));

        row![label, value]
            .spacing(12)
            .align_items(Alignment::Center)
            .into()
    }

    fn recording_table_header(&self) -> Element<'_, Message> {
        let label = text("Category")
            .size(12)
            .width(Length::FillPortion(2))
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));
        let start = text("Start")
            .size(12)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Right)
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));
        let end = text("End")
            .size(12)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Right)
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));
        let delta = text("Delta")
            .size(12)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Right)
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));

        row![label, start, end, delta]
            .spacing(12)
            .align_items(Alignment::Center)
            .into()
    }

    fn recording_table_row(
        &self,
        label: &str,
        start: Option<u64>,
        end: Option<u64>,
        delta: Option<u64>,
    ) -> Element<'_, Message> {
        let label = text(label)
            .size(13)
            .width(Length::FillPortion(2))
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));
        let start = text(format_count(start))
            .size(13)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Right)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)));
        let end = text(format_count(end))
            .size(13)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Right)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)));
        let delta = text(format_count(delta))
            .size(13)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Right)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)));

        row![label, start, end, delta]
            .spacing(12)
            .align_items(Alignment::Center)
            .into()
    }

    fn recording_table_row_editable(
        &self,
        category: RecordingCategory,
        label: &str,
        start_value: &str,
        end_value: &str,
        delta: Option<u64>,
        include_in_price: bool,
    ) -> Element<'_, Message> {
        let indicator_color = if include_in_price {
            Color::from_rgb8(0x6a, 0x6a, 0x6a)
        } else {
            Color::from_rgb8(0xe0, 0x4f, 0x4f)
        };

        let indicator = button(text("o").size(12))
            .on_press(Message::RecordingToggleInclude(category))
            .padding(2)
            .style(theme::Button::custom(IndicatorButtonStyle {
                color: indicator_color,
            }));

        let label = row![
            indicator,
            text(label)
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)))
        ]
        .spacing(6)
        .align_items(Alignment::Center)
        .width(Length::FillPortion(2));

        let start = text_input("n/a", start_value)
            .on_input(move |value| Message::RecordingStartChanged { category, value })
            .padding(4)
            .size(12)
            .width(Length::FillPortion(1));
        let end = text_input("n/a", end_value)
            .on_input(move |value| Message::RecordingEndChanged { category, value })
            .padding(4)
            .size(12)
            .width(Length::FillPortion(1));
        let delta = text(format_count(delta))
            .size(13)
            .width(Length::FillPortion(1))
            .horizontal_alignment(Horizontal::Right)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)));

        row![label, start, end, delta]
            .spacing(12)
            .align_items(Alignment::Center)
            .into()
    }

    fn debug_tab_view(&self) -> Element<'_, Message> {
        let level_picker = pick_list(
            &LogLevel::ALL[..],
            Some(self.log_level),
            Message::LogLevelChanged,
        )
        .placeholder("Log level");

        let console_header = row![
            text("Console")
                .size(20)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            level_picker
        ]
        .spacing(12)
        .align_items(Alignment::Center);

        let log_lines = self.log_lines_view();
        let filters = self.target_filters_view();

        let console = column![console_header, filters, log_lines]
            .spacing(12)
            .width(Length::FillPortion(2));

        let debug_panel = self.debug_panel_view();

        row![console, debug_panel]
            .spacing(16)
            .align_items(Alignment::Start)
            .into()
    }

    fn target_filters_view(&self) -> Element<'_, Message> {
        let mut filter_column = column![
            text("Targets")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)))
        ]
        .spacing(6);

        for target in self.sorted_targets() {
            let enabled = self.enabled_targets.contains(&target);
            filter_column = filter_column.push(
                checkbox(target.clone(), enabled)
                    .on_toggle(move |value| Message::ToggleTarget(target.clone(), value)),
            );
        }

        container(filter_column)
            .padding(8)
            .style(theme::Container::Box)
            .into()
    }

    fn log_lines_view(&self) -> Element<'_, Message> {
        let mut lines = column![].spacing(4);

        for entry in self.visible_entries() {
            let color = level_color(entry.level);
            let line = text(entry.format_line())
                .size(14)
                .horizontal_alignment(Horizontal::Left)
                .style(theme::Text::Color(color));
            lines = lines.push(line);
        }

        scrollable(lines)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    fn debug_panel_view(&self) -> Element<'_, Message> {
        let copy_status = self.copy_status.as_deref().unwrap_or("Ready");
        let panel = column![
            text("Debug panel")
                .size(20)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            text("Per-printer errors: none recorded yet.")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            text("SNMP OIDs used: not captured yet.")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            text("Persistence diagnostics: not captured yet.")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            text(format!("Mock SNMP entries: {}", self.mock_snmp_count))
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            button("Add mock SNMP entry").on_press(Message::AddMockSnmp),
            button("Copy diagnostics").on_press(Message::CopyDiagnostics),
            text(format!("Clipboard: {copy_status}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(10);

        container(panel)
            .padding(12)
            .width(Length::FillPortion(1))
            .style(theme::Container::Box)
            .into()
    }

}
