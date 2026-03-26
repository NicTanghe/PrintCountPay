impl PrintCountApp {
    fn tab_bar(&self) -> Element<'_, Message> {
        let mut left_tabs = row![self.tab_button(Tab::Printers, "Printers")]
            .spacing(8)
            .align_items(Alignment::Center);

        if self.advanced_mode {
            left_tabs = left_tabs.push(self.tab_button(Tab::Debug, "Debug"));
        }

        left_tabs.into()
    }

    fn window_controls_bar(&self) -> Element<'_, Message> {
        let right_controls = row![
            self.advanced_toggle_button(),
            self.window_button("-", Message::MinimizeWindow),
            self.window_button("x", Message::CloseWindow),
        ]
        .spacing(6)
        .align_items(Alignment::Center);

        row![horizontal_space(), right_controls]
            .spacing(8)
            .align_items(Alignment::Center)
            .into()
    }

    fn tab_button(&self, tab: Tab, label: &str) -> Element<'_, Message> {
        if self.active_tab == tab {
            self.top_bar_button(
                label,
                solid_brand_button_style(SIDEBAR_BRAND_SAMPLE),
                Message::SelectTab(tab),
            )
        } else {
            self.top_bar_button(label, theme::Button::Secondary, Message::SelectTab(tab))
        }
    }

    fn advanced_toggle_button(&self) -> Element<'_, Message> {
        let label = if self.advanced_mode {
            "Advanced: On"
        } else {
            "Advanced: Off"
        };
        if self.advanced_mode {
            self.top_bar_button(
                label,
                solid_brand_button_style(CONTROLS_BRAND_SAMPLE),
                Message::ToggleAdvancedMode,
            )
        } else {
            self.top_bar_button(label, theme::Button::Secondary, Message::ToggleAdvancedMode)
        }
    }

    fn window_button(&self, label: &str, message: Message) -> Element<'_, Message> {
        self.top_bar_button(label, theme::Button::Secondary, message)
    }

    fn top_bar_button(
        &self,
        label: &str,
        style: impl Fn(&Theme, iced::widget::button::Status) -> iced::widget::button::Style + 'static,
        message: Message,
    ) -> Element<'_, Message> {
        let label = container(text(label.to_string()).size(12))
            .height(Length::Fixed(16.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center);

        button(label)
            .style(style)
            .padding([4, 8])
            .on_press(message)
            .into()
    }

    fn printer_tab_bar(&self) -> Element<'_, Message> {
        let mut tabs = row![
            self.printer_tab_button(PrinterTab::Recording, "Recording"),
            self.printer_tab_button(PrinterTab::Pricing, "Pricing")
        ]
        .spacing(4)
        .align_items(Alignment::Center);

        if self.advanced_mode {
            tabs = row![
                self.printer_tab_button(PrinterTab::Polling, "Polling"),
                self.printer_tab_button(PrinterTab::Recording, "Recording"),
                self.printer_tab_button(PrinterTab::Pricing, "Pricing"),
                self.printer_tab_button(PrinterTab::Oids, "SNMP OIDs"),
                self.printer_tab_button(PrinterTab::AddPrinters, "Discovery + Manual")
            ]
            .spacing(4)
            .align_items(Alignment::Center);
        }

        tabs.into()
    }

    fn printer_tab_button(&self, tab: PrinterTab, label: &str) -> Element<'_, Message> {
        let style = theme::Button::custom(firefox_tab_style(self.printer_tab == tab));

        button(text(label.to_string()))
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
            button("Stop")
                .style(theme::Button::custom(solid_brand_button_style(
                    CONTENT_BRAND_SAMPLE,
                )))
                .on_press(Message::StopDiscovery)
        } else {
            button("Start")
                .style(theme::Button::custom(solid_brand_button_style(
                    CONTENT_BRAND_SAMPLE,
                )))
                .on_press(Message::StartDiscovery)
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
            row![
                button("Add printer")
                    .style(theme::Button::custom(solid_brand_button_style(
                        CONTENT_BRAND_SAMPLE,
                    )))
                    .on_press(Message::AddManualPrinter)
            ]
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
            button("Load")
                .style(theme::Button::custom(solid_brand_button_style(
                    SIDEBAR_BRAND_SAMPLE,
                )))
                .on_press(Message::LoadPrinters),
            button("Export")
                .style(theme::Button::custom(solid_brand_button_style(
                    SIDEBAR_BRAND_SAMPLE,
                )))
                .on_press(Message::SavePrinters),
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
        let last_poll_received_at = selected_id
            .and_then(|id| self.poll_states.get(id))
            .and_then(poll_received_at);
        let live_snapshot = if session.active {
            selected_id.and_then(|id| self.snapshot_for_printer(id).ok())
        } else {
            None
        };

        let status = session.status.clone().unwrap_or_else(|| {
            if session.active {
                let elapsed = last_poll_received_at
                    .map(|received_at| {
                        format_elapsed_hms(now_epoch_seconds().saturating_sub(received_at))
                    })
                    .unwrap_or_else(|| "n/a".to_string());
                format!("Time since last poll: {elapsed}")
            } else {
                last_poll_received_at
                    .or_else(|| session.end.as_ref().map(|snapshot| snapshot.received_at))
                    .or_else(|| session.start.as_ref().map(|snapshot| snapshot.received_at))
                    .map(|received_at| format!("Last poll at: {}", format_clock_hms(received_at)))
                    .unwrap_or_else(|| "Ready.".to_string())
            }
        });
        let state_label = if session.active {
            "Recording active"
        } else {
            "Recording idle"
        };

        let controls_enabled = selected_id.is_some();
        let start_button = if !controls_enabled || session.active {
            button("Start recording").style(theme::Button::Secondary)
        } else {
            button("Start recording")
                .style(theme::Button::custom(solid_brand_button_style(
                    CONTENT_BRAND_SAMPLE,
                )))
                .on_press(Message::StartRecording)
        };
        let stop_button = if !controls_enabled || !session.active {
            button("Stop recording").style(theme::Button::Secondary)
        } else {
            button("Stop recording")
                .style(theme::Button::custom(solid_brand_button_style(
                    CONTENT_BRAND_SAMPLE,
                )))
                .on_press(Message::StopRecording)
        };

        let elapsed_time = session
            .start
            .as_ref()
            .map(|start_snapshot| {
                let end_received_at = session
                    .end
                    .as_ref()
                    .map(|snapshot| snapshot.received_at)
                    .or_else(|| live_snapshot.as_ref().map(|snapshot| snapshot.received_at))
                    .unwrap_or(start_snapshot.received_at);

                format_elapsed_hms(end_received_at.saturating_sub(start_snapshot.received_at))
            })
            .unwrap_or_else(|| "n/a".to_string());

        let delta_section: Element<'_, Message> = if session.start.is_some() {
            let live_snapshot_ref = live_snapshot.as_ref();
            let end_display = |input: &str, category| {
                if !input.trim().is_empty() {
                    return input.to_string();
                }
                live_snapshot_ref
                    .and_then(|snapshot| snapshot_category_value(snapshot, category))
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| input.to_string())
            };

            let copies_bw_start = category_start_value(&session, RecordingCategory::CopiesBw);
            let copies_bw_end =
                category_end_value(&session, RecordingCategory::CopiesBw, live_snapshot_ref);
            let copies_bw_delta = delta_value(copies_bw_start, copies_bw_end);

            let copies_color_start = category_start_value(&session, RecordingCategory::CopiesColor);
            let copies_color_end =
                category_end_value(&session, RecordingCategory::CopiesColor, live_snapshot_ref);
            let copies_color_delta = delta_value(copies_color_start, copies_color_end);

            let prints_bw_start = category_start_value(&session, RecordingCategory::PrintsBw);
            let prints_bw_end =
                category_end_value(&session, RecordingCategory::PrintsBw, live_snapshot_ref);
            let prints_bw_delta = delta_value(prints_bw_start, prints_bw_end);

            let prints_color_start = category_start_value(&session, RecordingCategory::PrintsColor);
            let prints_color_end =
                category_end_value(&session, RecordingCategory::PrintsColor, live_snapshot_ref);
            let prints_color_delta = delta_value(prints_color_start, prints_color_end);

            let include_copies_bw = session.edits.copies_bw.include_in_price;
            let include_copies_color = session.edits.copies_color.include_in_price;
            let include_prints_bw = session.edits.prints_bw.include_in_price;
            let include_prints_color = session.edits.prints_color.include_in_price;

            let copies_bw_end_input =
                end_display(&session.edits.copies_bw.end_input, RecordingCategory::CopiesBw);
            let copies_color_end_input =
                end_display(&session.edits.copies_color.end_input, RecordingCategory::CopiesColor);
            let prints_bw_end_input =
                end_display(&session.edits.prints_bw.end_input, RecordingCategory::PrintsBw);
            let prints_color_end_input =
                end_display(&session.edits.prints_color.end_input, RecordingCategory::PrintsColor);

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
                    &copies_bw_end_input,
                    copies_bw_delta,
                    include_copies_bw,
                ),
                self.recording_table_row_editable(
                    RecordingCategory::CopiesColor,
                    "Copies color",
                    &session.edits.copies_color.start_input,
                    &copies_color_end_input,
                    copies_color_delta,
                    include_copies_color,
                ),
                self.recording_table_row_editable(
                    RecordingCategory::PrintsBw,
                    "Prints B/W",
                    &session.edits.prints_bw.start_input,
                    &prints_bw_end_input,
                    prints_bw_delta,
                    include_prints_bw,
                ),
                self.recording_table_row_editable(
                    RecordingCategory::PrintsColor,
                    "Prints color",
                    &session.edits.prints_color.start_input,
                    &prints_color_end_input,
                    prints_color_delta,
                    include_prints_color,
                ),
                rule::horizontal(1),
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
                rule::horizontal(1),
                self.value_line("Total price", total_cents.map(format_cents)),
                text(rounding_label)
                    .size(11)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            ]
            .spacing(6)
            .into()
        } else {
            text("No recording started yet.")
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a)))
                .into()
        };

        let mut content = column![].spacing(12);
        if self.advanced_mode {
            content = content.push(
                text(format!("Selected printer: {selected_label}"))
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            );
            content = content.push(
                text(format!("Recording printer ID: {selected_id_label}"))
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            );
            content = content.push(
                text(state_label)
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            );
        }
        content = content.push(
            row![start_button, stop_button]
                .spacing(8)
                .align_items(Alignment::Center),
        );
        content = content.push(
            text(format!("Elapsed: {elapsed_time}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        );
        content = content.push(
            text(format!("Status: {status}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        );
        content = content.push(delta_section);

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

        let rounding_toggle = checkbox(self.pricing.round_to_half_euro)
            .label("Round B/W to nearest 0.50 EUR")
            .on_toggle(Message::PricingRoundChanged)
            .size(12)
            .style(theme::Checkbox::custom(brand_checkbox_style(
                CONTENT_BRAND_SAMPLE,
            )));

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
        let mut list_items = column![].spacing(10);

        if self.printers.is_empty() {
            list_items = list_items.push(
                text("No printers discovered or added yet.")
                    .size(14)
                    .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            );
        } else {
            let total = self.printers.len();
            for (index, record) in self.printers.iter().enumerate() {
                list_items = list_items.push(self.printer_row(record, index, total));
            }
        }

        let scroll = scrollable(list_items)
            .height(Length::Fill)
            .width(Length::Fill);
        let mut content = column![
            self.tab_bar(),
            text("Printers")
                .size(28)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
        ]
        .spacing(18);

        content = content.push(scroll);

        if self.advanced_mode {
            content = content.push(self.printer_storage_controls_view());
        }

        container(content)
            .padding(iced::Padding {
                top: 20.0,
                right: 18.0,
                bottom: 16.0,
                left: 18.0,
            })
            .width(Length::FillPortion(1))
            .height(Length::Fill)
            .style(theme::Container::Custom(sidebar_panel_style()))
            .into()
    }

    fn printer_row(&self, record: &PrinterRecord, index: usize, total: usize) -> Element<'_, Message> {
        let is_selected = self.selected_printer.as_ref() == Some(&record.id);
        let is_recording = self
            .recording_sessions
            .get(&record.id)
            .map(|session| session.active)
            .unwrap_or(false);
        let base_color = printer_card_tint(index, total);
        let address = record
            .ip_or_hostname
            .as_deref()
            .or_else(|| record.snmp_address.as_ref().map(|addr| addr.host.as_str()))
            .unwrap_or("unknown host")
            .to_string();
        let name = record.model.as_deref().unwrap_or("Unknown name").to_string();
        let status = status_label(record.status).to_string();
        let name_color = if is_selected {
            Color::from_rgb8(0xff, 0xff, 0xff)
        } else {
            Color::from_rgb8(0x1f, 0x2a, 0x37)
        };
        let secondary_color = if is_selected {
            Color::from_rgba8(0xff, 0xff, 0xff, 0.82)
        } else {
            Color::from_rgb8(0x5a, 0x66, 0x78)
        };
        let status_color = if is_selected {
            Color::from_rgba8(0xff, 0xff, 0xff, 0.86)
        } else {
            match record.status {
                printcountpay_core::PrinterStatus::Online => Color::from_rgb8(0x4d, 0x8f, 0x6a),
                printcountpay_core::PrinterStatus::Offline => Color::from_rgb8(0x8a, 0x93, 0xa3),
                printcountpay_core::PrinterStatus::Error => Color::from_rgb8(0xd2, 0x57, 0x57),
                printcountpay_core::PrinterStatus::Unknown => Color::from_rgb8(0xb1, 0x87, 0x38),
            }
        };

        let details = row![
            text(address)
                .size(13)
                .style(theme::Text::Color(secondary_color)),
            text("|")
                .size(13)
                .style(theme::Text::Color(secondary_color)),
            text(status)
                .size(13)
                .style(theme::Text::Color(status_color)),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let content = column![
            text(name)
                .size(16)
                .style(theme::Text::Color(name_color)),
            details,
        ]
        .spacing(6);

        let base = button(content)
            .style(theme::Button::custom(printer_card_style(
                is_selected,
                base_color,
            )))
            .width(Length::Fill)
            .padding([14, 16])
            .clip(true)
            .on_press(Message::SelectPrinter(record.id.clone()));

        BadgeOverlay::new(base, self.recording_badge(is_recording), is_recording)
            .margin(6.0)
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
                    if self.advanced_mode {
                        let address = record
                            .snmp_address
                            .as_ref()
                            .map(|addr| addr.to_string())
                            .unwrap_or_else(|| "Not set".to_string());
                        let name = record.model.as_deref().unwrap_or("Unknown name");
                        let profile_choices = self.profile_choices();
                        let selected_profile = self.profile_choice_for_record(record);
                        let profile_picker = pick_list(
                            profile_choices,
                            Some(selected_profile),
                            Message::ProfileChoiceChanged,
                        )
                        .placeholder("Auto match");
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
                        content = content.push(
                            row![
                                text("Profile")
                                    .size(12)
                                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                                profile_picker
                            ]
                            .spacing(8)
                            .align_items(Alignment::Center),
                        );
                        if let Some(status) = self.profile_status.as_deref() {
                            content = content.push(
                                text(format!("Profile status: {status}"))
                                    .size(12)
                                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                            );
                        }
                    }
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
        text(message.to_string())
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
        .spacing(8)
        .width(Length::Fill);

        scrollable(container(content).padding(iced::Padding {
            top: 0.0,
            right: 24.0,
            bottom: 0.0,
            left: 0.0,
        }))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
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
            button("Load")
                .style(theme::Button::custom(solid_brand_button_style(
                    CONTENT_BRAND_SAMPLE,
                )))
                .on_press(Message::LoadOids),
            button("Save")
                .style(theme::Button::custom(solid_brand_button_style(
                    CONTENT_BRAND_SAMPLE,
                )))
                .on_press(Message::SaveOids),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let counter_inputs = column![
            self.oids_input(
                "Copies B/W OIDs",
                "1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.201",
                &self.recording_oids.copies_bw_input,
                Message::RecordingOidCopiesBwChanged,
            ),
            self.oids_input(
                "Copies color OIDs",
                "1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.203",
                &self.recording_oids.copies_color_input,
                Message::RecordingOidCopiesColorChanged,
            ),
            self.oids_input(
                "Prints B/W OIDs",
                "1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.401",
                &self.recording_oids.prints_bw_input,
                Message::RecordingOidPrintsBwChanged,
            ),
            self.oids_input(
                "Prints color OIDs",
                "1.3.6.1.4.1.367.3.2.1.2.19.5.1.9.403",
                &self.recording_oids.prints_color_input,
                Message::RecordingOidPrintsColorChanged,
            ),
            self.oids_input(
                "Total clicks OIDs",
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
            button(crawl_label)
                .style(theme::Button::custom(solid_brand_button_style(
                    CONTENT_BRAND_SAMPLE,
                )))
                .on_press(Message::CrawlOids)
        };

        let actions = row![
            button("Apply mapping")
                .style(theme::Button::custom(solid_brand_button_style(
                    CONTENT_BRAND_SAMPLE,
                )))
                .on_press(Message::ApplyOids),
            crawl_button
        ]
            .spacing(8)
            .align_items(Alignment::Center);

        let content = column![
            text("Profile OID mapping")
                .size(18)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            text("Enter dotted OIDs separated by commas or spaces.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            column![
                text("Profile file path")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                path_controls,
            ]
            .spacing(4),
            counter_inputs,
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
            text(label.to_string())
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
            text(label.to_string())
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
                let label_map = self.poll_label_map();
                let mut rows = column![].spacing(4);
                if varbinds.is_empty() {
                    rows = rows.push(
                        text("No varbinds returned.")
                            .size(13)
                            .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
                    );
                } else {
                    for varbind in varbinds.iter().take(MAX_VARBINDS_SHOWN) {
                        let label = label_map
                            .get(&varbind.oid)
                            .cloned()
                            .unwrap_or_else(|| varbind.oid.to_string());
                        rows = rows.push(
                            self.poll_varbind_row(&label, &varbind.value.to_string()),
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

                let body = column![
                    text(format!("Varbinds: {shown_varbinds}/{total_varbinds}"))
                        .size(12)
                        .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                    rows
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
                    text(detail.clone())
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

    fn poll_varbind_row(&self, label: &str, value: &str) -> Element<'_, Message> {
        let label = text(label.to_string())
            .size(13)
            .width(Length::Fill)
            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a)));
        let value = text(value.to_string())
            .size(13)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)));

        row![label, value]
            .spacing(12)
            .align_items(Alignment::Center)
            .into()
    }

    fn poll_label_map(&self) -> std::collections::HashMap<Oid, String> {
        let mut map = std::collections::HashMap::new();
        let recording_oids = recording_profile_from_settings_lossy(&self.recording_oids);
        let mut insert_label = |oid: Oid, label: &str| {
            map.entry(oid).or_insert_with(|| label.to_string());
        };

        insert_label(Oid::from_slice(&SYS_DESCR_OID), "System: Description");
        insert_label(Oid::from_slice(&SYS_OBJECT_ID_OID), "System: Object ID");
        insert_label(Oid::from_slice(&SYS_NAME_OID), "System: Name");
        insert_label(Oid::from_slice(&SYS_UPTIME_OID), "System: Uptime");
        insert_label(
            Oid::from_slice(&PRT_GENERAL_PRINTER_NAME_OID),
            "Printer: Name",
        );

        if self
            .active_profile
            .as_ref()
            .and_then(|profile| profile.counter_table.as_deref())
            == Some("ricoh-m184")
        {
            for entry in &RICOH_COUNTER_TABLE {
                insert_label(ricoh_counter_oid(entry.type_id), entry.label);
            }
        }

        for oid in &recording_oids.copies_bw {
            insert_label(oid.clone(), "Recording: Copies B/W");
        }
        for oid in &recording_oids.copies_color {
            insert_label(oid.clone(), "Recording: Copies Color");
        }
        for oid in &recording_oids.prints_bw {
            insert_label(oid.clone(), "Recording: Prints B/W");
        }
        for oid in &recording_oids.prints_color {
            insert_label(oid.clone(), "Recording: Prints Color");
        }

        for oid in &self.counter_oids.bw {
            insert_label(oid.clone(), "Clicks: B/W");
        }
        for oid in &self.counter_oids.color {
            insert_label(oid.clone(), "Clicks: Color");
        }
        for oid in &self.counter_oids.total {
            insert_label(oid.clone(), "Clicks: Total");
        }

        let default_toner = default_toner_oids();
        let toner = self
            .active_profile
            .as_ref()
            .map(|profile| &profile.toner)
            .unwrap_or(&default_toner);
        if let Some(oid) = toner.black.as_ref() {
            insert_label(oid.clone(), "Toner: Black");
        }
        if let Some(oid) = toner.cyan.as_ref() {
            insert_label(oid.clone(), "Toner: Cyan");
        }
        if let Some(oid) = toner.magenta.as_ref() {
            insert_label(oid.clone(), "Toner: Magenta");
        }
        if let Some(oid) = toner.yellow.as_ref() {
            insert_label(oid.clone(), "Toner: Yellow");
        }

        if let Some(profile) = self.active_profile.as_ref() {
            for entry in &profile.extra_poll_labels {
                map.insert(entry.oid.clone(), entry.label.clone());
            }
        }

        map
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
                let recording_oids = recording_profile_from_settings_lossy(&self.recording_oids);
                let default_toner = default_toner_oids();
                let toner = self
                    .active_profile
                    .as_ref()
                    .map(|profile| &profile.toner)
                    .unwrap_or(&default_toner);
                let ricoh_table_label = self.active_profile.as_ref().and_then(|profile| {
                    if profile.counter_table.as_deref() == Some("ricoh-m184") {
                        Some(format!("Ricoh counter table ({})", profile.firmware))
                    } else {
                        None
                    }
                });
                let mut lines = column![
                    text("Printer counts")
                        .size(13)
                        .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    self.value_line(
                        "B/W printer",
                        self.value_from_oids(varbinds, &recording_oids.prints_bw),
                    ),
                    self.value_line(
                        "Color printer",
                        self.value_from_oids(varbinds, &recording_oids.prints_color),
                    ),
                    text("Copier counts")
                        .size(13)
                        .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    self.value_line(
                        "B/W copier",
                        self.value_from_oids(varbinds, &recording_oids.copies_bw),
                    ),
                    self.value_line(
                        "Color copier",
                        self.value_from_oids(varbinds, &recording_oids.copies_color),
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
                        self.toner_value(varbinds, toner.black.as_ref()),
                    ),
                    self.value_line(
                        "Cyan",
                        self.toner_value(varbinds, toner.cyan.as_ref()),
                    ),
                    self.value_line(
                        "Magenta",
                        self.toner_value(varbinds, toner.magenta.as_ref()),
                    ),
                    self.value_line(
                        "Yellow",
                        self.toner_value(varbinds, toner.yellow.as_ref()),
                    ),
                ]
                .spacing(4);

                if let Some(table_label) = ricoh_table_label {
                    lines = lines.push(
                        text(table_label)
                            .size(13)
                            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    );

                    let mut table_rows = column![].spacing(4);
                    for entry in &RICOH_COUNTER_TABLE {
                        let label = format!(
                            "{} (type {}, {})",
                            entry.label, entry.type_id, entry.unit
                        );
                        table_rows = table_rows.push(self.value_line_owned(
                            label,
                            varbind_display_value(varbinds, &ricoh_counter_oid(entry.type_id)),
                        ));
                    }
                    lines = lines.push(table_rows);
                }

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

        text(label.to_string())
            .size(12)
            .style(theme::Text::Color(color))
            .into()
    }

    fn recording_badge(&self, active: bool) -> Element<'_, Message> {
        container(text("REC").size(9))
            .width(Length::Fixed(24.0))
            .height(Length::Fixed(24.0))
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .style(theme::Container::Custom(rec_badge_style(active)))
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

        let label = text(label.to_string())
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

        let label = text(label.to_string())
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

    fn value_line_owned(&self, label: String, value: Option<String>) -> Element<'_, Message> {
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

    fn profile_choices(&self) -> Vec<ProfileChoice> {
        let mut choices = vec![ProfileChoice::Auto];
        choices.extend(
            self.profile_index
                .profile_ids()
                .into_iter()
                .map(ProfileChoice::Profile),
        );
        choices
    }

    fn profile_choice_for_record(&self, record: &PrinterRecord) -> ProfileChoice {
        match record.profile_id.as_deref() {
            Some(id) => ProfileChoice::Profile(id.to_string()),
            None => ProfileChoice::Auto,
        }
    }

    fn value_from_oids(
        &self,
        varbinds: &[SnmpVarBind],
        oids: &[Oid],
    ) -> Option<String> {
        oids.iter()
            .find_map(|oid| varbind_display_value(varbinds, oid))
    }

    fn toner_value(
        &self,
        varbinds: &[SnmpVarBind],
        oid: Option<&Oid>,
    ) -> Option<String> {
        oid.and_then(|oid| varbind_display_value(varbinds, oid))
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
        let label = text(label.to_string())
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
            .style(theme::Button::custom(indicator_button_style(indicator_color)));

        let label = row![
            indicator,
            text(label.to_string())
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
                checkbox(enabled)
                    .label(target.clone())
                    .on_toggle(move |value| Message::ToggleTarget(target.clone(), value))
                    .style(theme::Checkbox::custom(brand_checkbox_style(
                        CONTENT_BRAND_SAMPLE,
                    ))),
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
            button("Copy diagnostics")
                .style(theme::Button::custom(solid_brand_button_style(
                    CONTENT_BRAND_SAMPLE,
                )))
                .on_press(Message::CopyDiagnostics),
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
