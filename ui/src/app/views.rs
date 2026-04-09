const PRINTER_DRAG_HANDLE_WIDTH: f32 = 46.0;
const PRINTER_ROW_SPACING: f32 = 10.0;
const PRINTER_DROP_SPLIT_Y: f32 = 32.0;

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
        let style: Box<
            dyn Fn(&Theme, iced::widget::button::Status) -> iced::widget::button::Style,
        > = if self.active_tab == tab {
            Box::new(solid_brand_button_style(SIDEBAR_BRAND_SAMPLE))
        } else {
            Box::new(theme::Button::custom(top_controls_button_style()))
        };

        self.top_bar_button(label, style, Message::SelectTab(tab))
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
            self.top_bar_button(
                label,
                theme::Button::custom(top_controls_button_style()),
                Message::ToggleAdvancedMode,
            )
        }
    }

    fn window_button(&self, label: &str, message: Message) -> Element<'_, Message> {
        self.top_bar_button(label, theme::Button::custom(top_controls_button_style()), message)
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

        button(text(label.to_string()).size(12))
            .padding([4, 10])
            .style(style)
            .on_press(Message::SelectPrinterTab(tab))
            .into()
    }

    fn printer_tab_scroll_view<'a>(
        &self,
        content: impl Into<Element<'a, Message>>,
        right_padding: f32,
    ) -> Element<'a, Message> {
        scrollable(container(content).padding(iced::Padding {
            top: 0.0,
            right: right_padding,
            bottom: 0.0,
            left: 0.0,
        }))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new()
                .width(8)
                .margin(2)
                .scroller_width(8),
        ))
        .style(manual_pricing_scrollable_style())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn boxed_printer_tab_scroll_view<'a>(
        &self,
        content: impl Into<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        self.printer_tab_scroll_view(content, 16.0)
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
        let start_recording_result: Option<Result<RecordingSnapshot, String>> = if session.active {
            None
        } else {
            selected_id.map(|id| self.ready_recording_snapshot(id))
        };
        let start_recording_enabled = start_recording_result
            .as_ref()
            .map(Result::is_ok)
            .unwrap_or(false);
        let start_recording_blocked_reason = start_recording_result
            .as_ref()
            .and_then(|result| result.as_ref().err())
            .cloned();
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
            } else if let Some(reason) = start_recording_blocked_reason.clone() {
                format!("Start unavailable: {reason}")
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
        let recording_button_label = if session.active {
            "Stop recording"
        } else {
            "Start recording"
        };
        let recording_button = if !controls_enabled {
            button(recording_button_label)
                .style(theme::Button::custom(muted_content_button_style()))
        } else if session.active {
            button(recording_button_label)
                .style(theme::Button::custom(solid_recording_button_style()))
                .on_press(Message::StopRecording)
        } else if !start_recording_enabled {
            button(recording_button_label)
                .style(theme::Button::custom(muted_content_button_style()))
        } else {
            button(recording_button_label)
                .style(theme::Button::custom(solid_brand_button_style(
                    CONTENT_BRAND_SAMPLE,
                )))
                .on_press(Message::StartRecording)
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

            let copies_bw_start_input =
                category_start_display(&session, RecordingCategory::CopiesBw);
            let copies_bw_end_input =
                category_end_display(&session, RecordingCategory::CopiesBw, live_snapshot_ref);
            let copies_color_start_input =
                category_start_display(&session, RecordingCategory::CopiesColor);
            let copies_color_end_input =
                category_end_display(&session, RecordingCategory::CopiesColor, live_snapshot_ref);
            let prints_bw_start_input =
                category_start_display(&session, RecordingCategory::PrintsBw);
            let prints_bw_end_input =
                category_end_display(&session, RecordingCategory::PrintsBw, live_snapshot_ref);
            let prints_color_start_input =
                category_start_display(&session, RecordingCategory::PrintsColor);
            let prints_color_end_input =
                category_end_display(&session, RecordingCategory::PrintsColor, live_snapshot_ref);

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
                if self.pricing.round_to_five_cents {
                    round_to_nearest_5_cents(value)
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
            let rounding_label = if self.pricing.round_to_five_cents {
                "B/W rounded to nearest 0.05 EUR"
            } else {
                "No rounding applied"
            };
            column![
                self.recording_table_header(),
                self.recording_table_row_editable(
                    RecordingCategory::CopiesBw,
                    "Copies B/W",
                    &copies_bw_start_input,
                    &copies_bw_end_input,
                    copies_bw_delta,
                    include_copies_bw,
                    session.end_fields_unlocked,
                ),
                self.recording_table_row_editable(
                    RecordingCategory::CopiesColor,
                    "Copies color",
                    &copies_color_start_input,
                    &copies_color_end_input,
                    copies_color_delta,
                    include_copies_color,
                    session.end_fields_unlocked,
                ),
                self.recording_table_row_editable(
                    RecordingCategory::PrintsBw,
                    "Prints B/W",
                    &prints_bw_start_input,
                    &prints_bw_end_input,
                    prints_bw_delta,
                    include_prints_bw,
                    session.end_fields_unlocked,
                ),
                self.recording_table_row_editable(
                    RecordingCategory::PrintsColor,
                    "Prints color",
                    &prints_color_start_input,
                    &prints_color_end_input,
                    prints_color_delta,
                    include_prints_color,
                    session.end_fields_unlocked,
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
            row![recording_button]
                .spacing(8)
                .align_items(Alignment::Center),
        );
        content = content.push(
            text(format!("Elapsed: {elapsed_time}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        );
        let status_line = row![
            text(format!("Status: {status}"))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            horizontal_space(),
            if session.start.is_some() {
                self.recording_end_toggle_button(session.end_fields_unlocked)
            } else {
                Space::new().width(Length::Shrink).into()
            }
        ]
        .spacing(8)
        .align_items(Alignment::Center);
        content = content.push(status_line);
        content = content.push(delta_section);

        self.boxed_printer_tab_scroll_view(
            container(content)
                .padding(12)
                .width(Length::Fill)
                .style(theme::Container::Box),
        )
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

        let rounding_toggle = checkbox(self.pricing.round_to_five_cents)
            .label("Round B/W to nearest 0.05 EUR")
            .on_toggle(Message::PricingRoundChanged)
            .size(12)
            .style(theme::Checkbox::custom(brand_checkbox_style(
                CONTENT_BRAND_SAMPLE,
            )));

        let hint = text("Used for recording totals. Decimals accept . or ,")
            .size(11)
            .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a)));

        let content = column![bw_section, color_section, rounding_toggle, hint].spacing(12);

        self.boxed_printer_tab_scroll_view(
            container(content)
                .padding(12)
                .width(Length::Fill)
                .style(theme::Container::Box),
        )
    }

    fn manual_pricing_panel_view(&self) -> Element<'_, Message> {
        let title_block: Element<'_, Message> = if let Some(bill) = self.selected_manual_bill() {
            let subject_input = text_input("Bill subject", &bill.subject)
                .on_input(Message::ManualPricingBillSubjectChanged)
                .padding(6)
                .size(12)
                .width(Length::Fixed(220.0));

            row![
                column![
                    text("Shared bill")
                        .size(20)
                        .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                    text(format!("ID: {}", bill.id))
                        .size(12)
                        .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                ]
                .spacing(4),
                horizontal_space(),
                button("Delete bill")
                    .style(theme::Button::custom(muted_content_button_style()))
                    .on_press(Message::DeleteSelectedManualPricingBill),
                column![
                    text("Subject")
                        .size(12)
                        .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                    subject_input,
                ]
                .spacing(4),
            ]
            .spacing(12)
            .align_items(Alignment::Center)
            .into()
        } else {
            row![
                column![
                    text("Manual pricing")
                        .size(20)
                        .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                    text("Use sheets for paper cost and printed sides for print cost, including recto verso.")
                        .size(12)
                        .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                ]
                .spacing(4),
                horizontal_space(),
                button("Save as bill")
                    .style(theme::Button::custom(solid_brand_button_style(
                        CONTENT_BRAND_SAMPLE,
                    )))
                    .on_press(Message::SaveManualPricingAsBill),
            ]
            .spacing(12)
            .align_items(Alignment::Center)
            .into()
        };

        let mut content = column![title_block]
            .spacing(12)
            .height(Length::Fill);

        if self.selected_manual_bill().is_none() {
            content = content.push(self.manual_pricing_tab_bar());
        }

        content = content.push(
            scrollable(container(self.manual_pricing_body_view()).padding(iced::Padding {
                top: 0.0,
                right: 16.0,
                bottom: 0.0,
                left: 0.0,
            }))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new()
                    .width(8)
                    .margin(2)
                    .scroller_width(8),
            ))
            .style(manual_pricing_scrollable_style())
            .height(Length::Fill)
            .width(Length::Fill),
        );

        container(content)
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Custom(right_content_panel_style()))
            .into()
    }

    fn manual_pricing_tab_bar(&self) -> Element<'_, Message> {
        row![
            self.manual_pricing_tab_button(ManualPricingTab::Calculator, "Calculator"),
            self.manual_pricing_tab_button(ManualPricingTab::Prices, "Prices"),
            self.manual_pricing_tab_button(ManualPricingTab::Finishers, "Finishers"),
        ]
        .spacing(4)
        .align_items(Alignment::Center)
        .into()
    }

    fn manual_pricing_tab_button(
        &self,
        tab: ManualPricingTab,
        label: &str,
    ) -> Element<'_, Message> {
        button(text(label.to_string()).size(12))
            .padding([4, 10])
            .style(theme::Button::custom(firefox_tab_style(
                self.manual_pricing_tab == tab,
            )))
            .on_press(Message::SelectManualPricingTab(tab))
            .into()
    }

    fn manual_pricing_body_view(&self) -> Element<'_, Message> {
        let manual = self.active_manual_pricing();
        let totals = manual_pricing_totals(manual);
        let calculator_only = self.selected_manual_bill().is_some();

        let mut size_prices = column![
            text("Flat print price per side")
                .size(15)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            text("Use flat per-side pricing for A0, A1, and A2.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(8);

        for size in [ManualPrintSize::A0, ManualPrintSize::A1, ManualPrintSize::A2] {
            size_prices = size_prices.push(self.manual_input(
                &format!("{size} per side (EUR)"),
                "0.00",
                manual.size_price_input(size),
                move |value| Message::ManualPricingBasePriceChanged(size, value),
            ));
        }

        let size_prices = container(size_prices)
            .padding(12)
            .width(Length::Fill)
            .style(theme::Container::Box);

        let tiered_prices = column![
            self.manual_tiered_price_box(ManualPrintSize::A3),
            self.manual_tiered_price_box(ManualPrintSize::A4),
        ]
        .spacing(12);

        let mut modifier_setup = column![
            row![
                text("Paper modifiers")
                    .size(15)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                horizontal_space(),
                button("Add modifier")
                    .style(theme::Button::custom(solid_brand_button_style(
                        CONTENT_BRAND_SAMPLE,
                    )))
                    .on_press(Message::ManualPricingModifierAdded),
            ]
            .align_items(Alignment::Center),
            text("Paper modifiers are charged per sheet. Configure A0, A1, A2, A3, and A4 separately for each modifier.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(8);

        for (index, modifier) in manual.modifiers.iter().enumerate() {
            modifier_setup =
                modifier_setup.push(self.manual_pricing_modifier_row(index, modifier));
        }

        let modifier_setup = container(modifier_setup)
            .padding(12)
            .width(Length::Fill)
            .style(theme::Container::Box);

        let mut calculator_section = column![
            row![
                text("Order lines")
                    .size(15)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                horizontal_space(),
                button("Add finisher")
                    .style(theme::Button::custom(solid_brand_button_style(
                        CONTENT_BRAND_SAMPLE,
                    )))
                    .on_press(Message::ManualPricingFinisherAdded),
                button("Add line")
                    .style(theme::Button::custom(solid_brand_button_style(
                        CONTENT_BRAND_SAMPLE,
                    )))
                    .on_press(Message::ManualPricingLineAdded),
            ]
            .spacing(10)
            .align_items(Alignment::Center),
            text("Use sheets for paper count and printed sides for actual printed faces.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(8);

        for (index, line_item) in manual.line_items.iter().enumerate() {
            let line_state = totals
                .line_states
                .get(index)
                .cloned()
                .unwrap_or(ManualLineState::Invalid);
            calculator_section = calculator_section.push(self.manual_pricing_line_item_row(
                index,
                line_item,
                line_state,
            ));
        }

        calculator_section = calculator_section.push(
            text("Finishers")
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
        );

        if manual.finisher_items.is_empty() {
            calculator_section = calculator_section.push(
                text("No finishers added. Use Add finisher for laminate, folding, or binding.")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            );
        } else {
            for (index, finisher_item) in manual.finisher_items.iter().enumerate() {
                let finisher_state = totals
                    .finisher_states
                    .get(index)
                    .cloned()
                    .unwrap_or(ManualFinisherState::Invalid);
                calculator_section = calculator_section.push(self.manual_pricing_finisher_row(
                    index,
                    finisher_item,
                    finisher_state,
                ));
            }
        }

        calculator_section = calculator_section
            .push(
            checkbox(manual.cutting_enabled)
                .label("Cutting (+3 EUR)")
                .on_toggle(Message::ManualPricingCuttingChanged)
                .size(12)
                .style(theme::Checkbox::custom(brand_checkbox_style(
                    CONTENT_BRAND_SAMPLE,
                ))),
            )
            .push(
                self.manual_input(
                    "Discount (%)",
                    "0",
                    &manual.discount_input,
                    Message::ManualPricingDiscountChanged,
                ),
            )
            .push(
                text("Rounding")
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
            )
            .push(
                column![
                checkbox(manual.rounding_mode == ManualRoundingMode::FiveCents)
                    .label("Round down to 0.05 EUR")
                    .on_toggle(|value| {
                        Message::ManualPricingRoundingToggled(ManualRoundingMode::FiveCents, value)
                    })
                    .size(12)
                    .style(theme::Checkbox::custom(brand_checkbox_style(
                        CONTENT_BRAND_SAMPLE,
                    ))),
                checkbox(manual.rounding_mode == ManualRoundingMode::HalfEuro)
                    .label("Round down to 0.50 EUR")
                    .on_toggle(|value| {
                        Message::ManualPricingRoundingToggled(ManualRoundingMode::HalfEuro, value)
                    })
                    .size(12)
                    .style(theme::Checkbox::custom(brand_checkbox_style(
                        CONTENT_BRAND_SAMPLE,
                    ))),
                checkbox(manual.rounding_mode == ManualRoundingMode::DownToFiveEuro)
                    .label("Round down to 5 EUR")
                    .on_toggle(|value| {
                        Message::ManualPricingRoundingToggled(
                            ManualRoundingMode::DownToFiveEuro,
                            value,
                        )
                    })
                    .size(12)
                    .style(theme::Checkbox::custom(brand_checkbox_style(
                        CONTENT_BRAND_SAMPLE,
                    ))),
                checkbox(manual.rounding_mode == ManualRoundingMode::DownToTenEuro)
                    .label("Round down to 10 EUR")
                    .on_toggle(|value| {
                        Message::ManualPricingRoundingToggled(
                            ManualRoundingMode::DownToTenEuro,
                            value,
                        )
                    })
                    .size(12)
                    .style(theme::Checkbox::custom(brand_checkbox_style(
                        CONTENT_BRAND_SAMPLE,
                    ))),
            ]
                .spacing(6),
            );

        let calculator_section = container(calculator_section)
            .padding(12)
            .width(Length::Fill)
            .style(theme::Container::Box);

        let lines_total_label = totals
            .lines_total_cents
            .map(format_cents)
            .unwrap_or_else(|| "Invalid line input".to_string());
        let finishers_total_label = totals
            .finishers_total_cents
            .map(format_cents)
            .unwrap_or_else(|| "Invalid finisher input".to_string());
        let subtotal_label = totals
            .subtotal_cents
            .map(format_cents)
            .unwrap_or_else(|| "Invalid line, finisher, or discount input".to_string());
        let discount_label = totals
            .discount_cents
            .map(|value| format!("-{}", format_cents(value)))
            .unwrap_or_else(|| "Invalid discount input".to_string());
        let before_rounding_label = totals
            .total_before_rounding_cents
            .map(format_cents)
            .unwrap_or_else(|| "N/A".to_string());
        let total_label = totals
            .total_cents
            .map(format_cents)
            .unwrap_or_else(|| "N/A".to_string());
        let warning = if totals.total_cents.is_none() {
            Some(
                text("Fix any invalid line, finisher, size price, modifier price, finisher price, or discount input to calculate the total.")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0xe0, 0x4f, 0x4f))),
            )
        } else {
            None
        };

        let mut summary = column![
            text("Summary")
                .size(15)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            self.value_line("Lines total", Some(lines_total_label)),
            self.value_line("Finishers total", Some(finishers_total_label)),
            self.value_line(
                "Cutting fee",
                Some(if totals.cutting_cents == 0 {
                    "0.00 EUR".to_string()
                } else {
                    format_cents(totals.cutting_cents)
                }),
            ),
            self.value_line("Subtotal before discount", Some(subtotal_label)),
            self.value_line("Discount", Some(discount_label)),
            self.value_line("Before rounding", Some(before_rounding_label)),
            self.value_line(
                "Rounding",
                Some(manual.rounding_mode.to_string()),
            ),
            self.value_line("Final total", Some(total_label)),
        ]
        .spacing(6);

        if let Some(warning) = warning {
            summary = summary.push(warning);
        }

        let summary = container(summary)
            .padding(12)
            .width(Length::Fill)
            .style(theme::Container::Box);

        if calculator_only {
            return column![calculator_section, summary]
                .spacing(12)
                .width(Length::Fill)
                .into();
        }

        match self.manual_pricing_tab {
            ManualPricingTab::Calculator => column![calculator_section, summary]
                .spacing(12)
                .width(Length::Fill)
                .into(),
            ManualPricingTab::Prices => column![
                self.manual_pricing_storage_controls_view(),
                size_prices,
                tiered_prices,
                modifier_setup
            ]
                .spacing(12)
                .width(Length::Fill)
                .into(),
            ManualPricingTab::Finishers => column![
                self.manual_pricing_finishers_config_view()
            ]
            .spacing(12)
            .width(Length::Fill)
            .into(),
        }
    }

    fn manual_pricing_storage_controls_view(&self) -> Element<'_, Message> {
        let status = self.manual_pricing_status.as_deref().unwrap_or("Ready.");
        let path_input = text_input("manual_pricing.ron", &self.manual_pricing_path)
            .on_input(Message::ManualPricingPathChanged)
            .padding(6)
            .size(12)
            .width(Length::Fill);

        let path_controls = row![
            path_input,
            button("Load")
                .style(theme::Button::custom(solid_brand_button_style(
                    SIDEBAR_BRAND_SAMPLE,
                )))
                .on_press(Message::LoadManualPricing),
            button("Save")
                .style(theme::Button::custom(solid_brand_button_style(
                    SIDEBAR_BRAND_SAMPLE,
                )))
                .on_press(Message::SaveManualPricing),
            button("Sync prices")
                .style(theme::Button::custom(solid_brand_button_style(
                    SIDEBAR_BRAND_SAMPLE,
                )))
                .on_press(Message::SyncPrices),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let content = column![
            text("Pricing config")
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

    fn manual_pricing_finishers_config_view(&self) -> Element<'_, Message> {
        let manual = self.active_manual_pricing();

        let mut laminate_prices = column![
            text("Laminate pricing")
                .size(15)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            text("Laminate uses a page size plus an amount. Configure A0 through A5 separately.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(8);

        for size in ManualLaminateSize::ALL {
            laminate_prices = laminate_prices.push(self.manual_input(
                &format!("{size} laminate (EUR)"),
                "0.00",
                manual.laminate_price_input(size),
                move |value| Message::ManualPricingLaminatePriceChanged(size, value),
            ));
        }

        let laminate_prices = container(laminate_prices)
            .padding(12)
            .width(Length::Fill)
            .style(theme::Container::Box);

        let finishing_prices = container(
            column![
                text("Other finishers")
                    .size(15)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                text("Folding and binding use one flat unit price. The calculator amount field controls how many times they are applied.")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                self.manual_input(
                    "Folding (EUR)",
                    "0.00",
                    &manual.folding_input,
                    Message::ManualPricingFoldingPriceChanged,
                ),
                self.manual_input(
                    "Binding (EUR)",
                    "0.00",
                    &manual.binding_input,
                    Message::ManualPricingBindingPriceChanged,
                ),
            ]
            .spacing(8),
        )
        .padding(12)
        .width(Length::Fill)
        .style(theme::Container::Box);

        column![
            self.manual_pricing_storage_controls_view(),
            laminate_prices,
            finishing_prices,
        ]
        .spacing(12)
        .width(Length::Fill)
        .into()
    }

    fn manual_print_mode_button(
        &self,
        index: usize,
        current_mode: ManualPrintMode,
    ) -> Element<'_, Message> {
        let icon_bytes = match current_mode {
            ManualPrintMode::Bw => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../assets/manual-bw-icon.svg"
            ))
            .as_slice(),
            ManualPrintMode::Color => include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../assets/manual-color-icon.svg"
            ))
            .as_slice(),
        };
        let icon = iced::widget::svg(iced::widget::svg::Handle::from_memory(icon_bytes))
            .width(18)
            .height(18)
            .style(|_theme, _status| iced::widget::svg::Style { color: None });
        let next_mode = match current_mode {
            ManualPrintMode::Bw => ManualPrintMode::Color,
            ManualPrintMode::Color => ManualPrintMode::Bw,
        };

        button(
            container(icon)
                .width(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .padding(2)
        .width(Length::Fixed(22.0))
        .style(theme::Button::custom(manual_icon_button_style()))
        .on_press(Message::ManualPricingLinePrintModeChanged(index, next_mode))
        .into()
    }

    fn manual_remove_icon_button(&self, message: Message) -> Element<'_, Message> {
        let icon = iced::widget::svg(iced::widget::svg::Handle::from_memory(
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../assets/manual-trash-icon.svg"
            ))
            .as_slice(),
        ))
        .width(20)
        .height(20)
        .style(|_theme, _status| iced::widget::svg::Style { color: None });

        button(
            container(icon)
                .width(Length::Fill)
                .align_x(Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .padding(1)
        .width(Length::Fixed(30.0))
        .style(theme::Button::custom(manual_icon_button_style()))
        .on_press(message)
        .into()
    }

    fn manual_pricing_line_item_row(
        &self,
        index: usize,
        line_item: &ManualPricingLineItem,
        line_state: ManualLineState,
    ) -> Element<'_, Message> {
        let modifier_choices = self.manual_modifier_choices(
            line_item.size,
            line_item.modifier_index,
        );
        let size_picker = pick_list(
            &ManualPrintSize::ALL[..],
            Some(line_item.size),
            move |size| Message::ManualPricingLineSizeChanged(index, size),
        )
        .placeholder("Size")
        .text_size(11)
        .width(Length::Fill)
        .style(profile_pick_list_style())
        .menu_style(profile_pick_list_menu_style());
        let selected_modifier = modifier_choices
            .iter()
            .find(|choice| choice.index == line_item.modifier_index)
            .cloned()
            .unwrap_or_else(|| ManualModifierChoice {
                index: None,
                label: "No modifier".to_string(),
            });
        let modifier_picker = pick_list(
            modifier_choices,
            Some(selected_modifier),
            move |choice| Message::ManualPricingLineModifierChanged(index, choice.index),
        )
        .placeholder("Modifier")
        .text_size(11)
        .width(Length::Fill)
        .style(profile_pick_list_style())
        .menu_style(profile_pick_list_menu_style());
        let sides_input = text_input("0", &line_item.sides_input)
            .on_input(move |value| Message::ManualPricingLineSidesChanged(index, value))
            .padding(6)
            .size(12)
            .width(Length::Fixed(42.0));
        let double_sided_toggle = checkbox(line_item.double_sided)
            .label("RV")
            .on_toggle(move |value| Message::ManualPricingLineDoubleSidedChanged(index, value))
            .size(12)
            .style(theme::Checkbox::custom(brand_checkbox_style(
                CONTENT_BRAND_SAMPLE,
            )));
        let sheets_value =
            self.recording_readonly_value(&line_item.sheets_input, Length::Fixed(54.0));
        let remove_button = self.manual_remove_icon_button(Message::ManualPricingLineRemoved(index));
        let placeholder_label = || {
            text(" ")
                .size(12)
                .style(theme::Text::Color(Color::TRANSPARENT))
        };

        let controls = row![
            column![
                text("Size")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                size_picker,
            ]
            .spacing(4)
            .width(Length::Fixed(50.0)),
            column![
                text("Type")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                self.manual_print_mode_button(index, line_item.print_mode),
            ]
            .spacing(4)
            .width(Length::Fixed(28.0)),
            column![
                text("Modifier")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                modifier_picker,
            ]
            .spacing(4)
            .width(Length::Fill),
            column![
                text("Zijden")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                sides_input,
            ]
            .spacing(4)
            .width(Length::Fixed(42.0)),
            column![
                text("Vellen")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                sheets_value,
            ]
            .spacing(4)
            .width(Length::Fixed(54.0)),
            column![
                placeholder_label(),
                container(double_sided_toggle)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
            ]
            .spacing(4)
            .width(Length::Fixed(42.0)),
            column![
                placeholder_label(),
                container(remove_button)
                    .width(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center),
            ]
            .spacing(4)
            .width(Length::Fixed(30.0)),
        ]
        .spacing(8)
        .align_items(Alignment::Center);

        let summary = match line_state {
            ManualLineState::Empty => {
                text("Set printed sides. Sheets are auto-calculated from the double-sided toggle.")
            }
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            ManualLineState::Invalid => text("Enter valid sheets, sides, size pricing, and modifier pricing.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0xe0, 0x4f, 0x4f))),
            ManualLineState::Ready(line) => text(format!(
                "{} | print {} sides = {} + {} sheets x {} = {}",
                line.print_pricing_label,
                line.sides,
                format_cents(line.print_total_cents),
                line.sheets,
                format_cents(line.paper_price_cents),
                format_cents(line.total_cents),
            ))
            .size(12)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37))),
        };

        container(column![controls, summary].spacing(10))
            .padding(12)
            .width(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }

    fn manual_pricing_finisher_row(
        &self,
        index: usize,
        finisher_item: &ManualFinisherLineItem,
        finisher_state: ManualFinisherState,
    ) -> Element<'_, Message> {
        let finisher_type_picker = pick_list(
            &ManualFinisherType::ALL[..],
            Some(finisher_item.finisher_type),
            move |finisher_type| Message::ManualPricingFinisherTypeChanged(index, finisher_type),
        )
        .placeholder("Finisher")
        .text_size(11)
        .style(profile_pick_list_style())
        .menu_style(profile_pick_list_menu_style());
        let size_control: Element<'_, Message> =
            if finisher_item.finisher_type == ManualFinisherType::Laminate {
                pick_list(
                    &ManualLaminateSize::ALL[..],
                    Some(finisher_item.laminate_size),
                    move |size| Message::ManualPricingFinisherSizeChanged(index, size),
                )
                .placeholder("Size")
                .text_size(11)
                .style(profile_pick_list_style())
                .menu_style(profile_pick_list_menu_style())
                .into()
            } else {
                self.recording_readonly_value("n/a", Length::Fixed(84.0))
            };
        let amount_input = text_input("0", &finisher_item.amount_input)
            .on_input(move |value| Message::ManualPricingFinisherAmountChanged(index, value))
            .padding(6)
            .size(12)
            .width(Length::Fixed(84.0));
        let remove_button = button("Remove")
            .style(theme::Button::custom(muted_content_button_style()))
            .on_press(Message::ManualPricingFinisherRemoved(index));

        let controls = row![
            column![
                text("Finisher")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                finisher_type_picker,
            ]
            .spacing(4)
            .width(Length::FillPortion(2)),
            column![
                text("Size")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                size_control,
            ]
            .spacing(4),
            column![
                text("Amount")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                amount_input,
            ]
            .spacing(4),
            container(remove_button).align_y(iced::alignment::Vertical::Bottom),
        ]
        .spacing(10)
        .align_items(Alignment::Center);

        let summary = match finisher_state {
            ManualFinisherState::Empty => text("Set an amount to price this finisher.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            ManualFinisherState::Invalid => {
                text("Enter a valid amount and finisher price.")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0xe0, 0x4f, 0x4f)))
            }
            ManualFinisherState::Ready(finisher) => text(format!(
                "{} x {} = {}",
                finisher.amount,
                finisher.label,
                format_cents(finisher.total_cents),
            ))
            .size(12)
            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37))),
        };

        container(column![controls, summary].spacing(8))
            .padding(10)
            .width(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }

    fn manual_tiered_price_box(&self, size: ManualPrintSize) -> Element<'_, Message> {
        let manual = self.active_manual_pricing();
        let bw = column![
            text("B/W")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            self.manual_input(
                "1-5 sides (EUR)",
                "0.00",
                manual.bw_tier_input(size, ManualBwTier::FirstFive).unwrap_or(""),
                move |value| Message::ManualPricingBwTierChanged(size, ManualBwTier::FirstFive, value),
            ),
            self.manual_input(
                "6-10 sides (EUR)",
                "0.00",
                manual.bw_tier_input(size, ManualBwTier::NextFive).unwrap_or(""),
                move |value| Message::ManualPricingBwTierChanged(size, ManualBwTier::NextFive, value),
            ),
            self.manual_input(
                "11+ sides (EUR)",
                "0.00",
                manual.bw_tier_input(size, ManualBwTier::Rest).unwrap_or(""),
                move |value| Message::ManualPricingBwTierChanged(size, ManualBwTier::Rest, value),
            ),
        ]
        .spacing(6);
        

        let color = column![
            text("Color")
                .size(14)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            self.manual_input(
                "1-5 sides (EUR)",
                "0.00",
                manual.color_tier_input(size, ManualColorTier::FirstFive).unwrap_or(""),
                move |value| {
                    Message::ManualPricingColorTierChanged(size, ManualColorTier::FirstFive, value)
                },
            ),
            self.manual_input(
                "6+ sides (EUR)",
                "0.00",
                manual.color_tier_input(size, ManualColorTier::Rest).unwrap_or(""),
                move |value| {
                    Message::ManualPricingColorTierChanged(size, ManualColorTier::Rest, value)
                },
            ),
        ]
        .spacing(6);

        container(
            column![
                text(format!("{size} tiered print pricing"))
                    .size(15)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                text("A3 and A4 can price B/W and Color separately.")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                row![bw.width(Length::FillPortion(1)), color.width(Length::FillPortion(1))]
                    .spacing(12),
            ]
            .spacing(8),
        )
        .padding(12)
        .width(Length::Fill)
        .style(theme::Container::Box)
        .into()
    }

    fn manual_pricing_modifier_row(
        &self,
        index: usize,
        modifier: &ManualPaperModifier,
    ) -> Element<'_, Message> {
        let name_input = text_input("300G", &modifier.name_input)
            .on_input(move |value| Message::ManualPricingModifierNameChanged(index, value))
            .padding(6)
            .size(12)
            .width(Length::Fill);
        let remove_button = button("Remove")
            .style(theme::Button::custom(muted_content_button_style()))
            .on_press(Message::ManualPricingModifierRemoved(index));

        let controls = row![
            column![
                text("Name")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                name_input,
            ]
            .spacing(4)
            .width(Length::FillPortion(2)),
            container(remove_button).align_y(iced::alignment::Vertical::Bottom),
        ]
        .spacing(10)
        .align_items(Alignment::Center);

        let size_row = |size: ManualPrintSize| {
            let enabled = modifier.applies_to_size(size);
            let price_value = modifier.price_input(size);
            row![
                text(size.to_string())
                    .size(12)
                    .width(Length::Fixed(28.0))
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                checkbox(enabled)
                    .label("Applies")
                    .on_toggle(move |value| {
                        Message::ManualPricingModifierAppliesChanged(index, size, value)
                    })
                    .size(12)
                    .style(theme::Checkbox::custom(brand_checkbox_style(
                        CONTENT_BRAND_SAMPLE,
                    ))),
                text_input("0.00", price_value)
                    .on_input(move |value| {
                        Message::ManualPricingModifierPriceChanged(index, size, value)
                    })
                    .padding(6)
                    .size(12)
                    .width(Length::Fixed(110.0)),
                text("EUR per sheet")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            ]
            .spacing(10)
            .align_items(Alignment::Center)
        };

        let applies = column![
            text("Per-size setup")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
            size_row(ManualPrintSize::A0),
            size_row(ManualPrintSize::A1),
            size_row(ManualPrintSize::A2),
            size_row(ManualPrintSize::A3),
            size_row(ManualPrintSize::A4),
        ]
        .spacing(8);

        container(column![controls, applies].spacing(8))
            .padding(10)
            .width(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }

    fn manual_modifier_choices(
        &self,
        size: ManualPrintSize,
        selected_index: Option<usize>,
    ) -> Vec<ManualModifierChoice> {
        let manual = self.active_manual_pricing();
        let mut choices = vec![ManualModifierChoice {
            index: None,
            label: "No modifier".to_string(),
        }];

        for (index, modifier) in manual.modifiers.iter().enumerate() {
            if modifier.applies_to_size(size) {
                choices.push(ManualModifierChoice {
                    index: Some(index),
                    label: modifier.display_name(),
                });
            }
        }

        if let Some(selected_index) = selected_index
            && !choices.iter().any(|choice| choice.index == Some(selected_index))
        {
            let label = manual
                .modifiers
                .get(selected_index)
                .map(|modifier| format!("{} (not for {size})", modifier.display_name()))
                .unwrap_or_else(|| "Missing modifier".to_string());
            choices.push(ManualModifierChoice {
                index: Some(selected_index),
                label,
            });
        }

        choices
    }

    fn manual_pricing_row(&self) -> Element<'_, Message> {
        let is_selected = self.manual_pricing_selected && self.selected_manual_bill_id.is_none();
        let name_color = if is_selected {
            Color::WHITE
        } else {
            Color::from_rgb8(0x1f, 0x2a, 0x37)
        };
        let secondary_color = if is_selected {
            Color::from_rgba8(0xff, 0xff, 0xff, 0.82)
        } else {
            Color::from_rgb8(0x5a, 0x66, 0x78)
        };

        let bill_count = self.manual_bills.len();
        let bill_label = if bill_count == 1 {
            "1 bill".to_string()
        } else {
            format!("{bill_count} bills")
        };

        let content = row![
            column![
                text("Manual pricing")
                    .size(15)
                    .style(theme::Text::Color(name_color)),
                text("A0-A4, paper types, discount, rounding")
                    .size(12)
                    .style(theme::Text::Color(secondary_color)),
            ]
            .spacing(4),
            horizontal_space(),
            text(bill_label)
                .size(12)
                .style(theme::Text::Color(secondary_color)),
        ]
        .spacing(10)
        .align_items(Alignment::Center);

        button(content)
            .style(theme::Button::custom(manual_pricing_header_button_style(
                is_selected,
            )))
            .width(Length::Fill)
            .padding([10, 12])
            .clip(true)
            .on_press(Message::SelectManualPricing)
            .into()
    }

    fn manual_pricing_bill_row(&self, bill: &ManualPricingBill) -> Element<'_, Message> {
        let is_selected =
            self.manual_pricing_selected
                && self.selected_manual_bill_id.as_deref() == Some(bill.id.as_str());
        let bill_id = bill.id.clone();
        let bill_subject = bill.display_subject().to_string();
        let base_color = Color::from_rgb8(0xf3, 0xf6, 0xfa);
        let name_color = if is_selected {
            Color::WHITE
        } else {
            Color::from_rgb8(0x1f, 0x2a, 0x37)
        };
        let secondary_color = if is_selected {
            Color::from_rgba8(0xff, 0xff, 0xff, 0.82)
        } else {
            Color::from_rgb8(0x5a, 0x66, 0x78)
        };

        let card = button(
            column![
                text(bill_id.clone())
                    .size(11)
                    .style(theme::Text::Color(secondary_color)),
                text(bill_subject)
                    .size(14)
                    .style(theme::Text::Color(name_color)),
            ]
            .spacing(4),
        )
        .style(theme::Button::custom(printer_card_style(
            is_selected,
            base_color,
        )))
        .width(Length::Fixed(266.0))
        .padding([11, 12])
        .clip(true)
        .on_press(Message::SelectManualPricingBill(bill_id));

        row![horizontal_space(), card]
            .width(Length::Fill)
            .align_items(Alignment::Center)
            .into()
    }

    fn printer_list_view(&self) -> Element<'_, Message> {
        let mut list_items = column![self.manual_pricing_row()].spacing(10);

        for bill in &self.manual_bills {
            list_items = list_items.push(self.manual_pricing_bill_row(bill));
        }

        if self.printers.is_empty() {
            list_items = list_items.push(
                text("No printers discovered or added yet.")
                    .size(14)
                    .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            );
        } else {
            let active_drop_index = self.active_printer_drag.as_ref().map(|drag| drag.drop_index);
            let total = self.printers.len();
            for (index, record) in self.printers.iter().enumerate() {
                if active_drop_index == Some(index) {
                    list_items = list_items.push(self.printer_drop_indicator());
                }
                list_items = list_items.push(self.printer_row(record, index, total));
            }
            if active_drop_index == Some(self.printers.len()) {
                list_items = list_items.push(self.printer_drop_indicator());
            }
        }

        if self.advanced_mode {
            list_items = list_items.push(self.printer_storage_controls_view());
        }

        let scroll = scrollable(
            container(list_items)
                .width(Length::Fill)
                .padding(iced::Padding {
                    top: 0.0,
                    right: 16.0,
                    bottom: 0.0,
                    left: 0.0,
                }),
        )
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new()
                    .width(8)
                    .margin(2)
                    .scroller_width(8),
            ))
            .style(printer_list_scrollable_style())
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

        container(content)
            .padding(iced::Padding {
                top: 20.0,
                right: 2.0,
                bottom: 16.0,
                left: 18.0,
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Custom(sidebar_panel_style()))
            .into()
    }

    fn printer_drop_indicator(&self) -> Element<'_, Message> {
        row![
            Space::new().width(Length::Fixed(PRINTER_DRAG_HANDLE_WIDTH)),
            container(Space::new().height(Length::Fixed(4.0)))
                .width(Length::Fill)
                .height(Length::Fixed(4.0))
                .style(theme::Container::Custom(printer_drop_indicator_style())),
        ]
        .spacing(PRINTER_ROW_SPACING)
        .width(Length::Fill)
        .align_items(Alignment::Center)
        .into()
    }

    fn printer_drag_handle(&self, printer_id: &PrinterId) -> Element<'_, Message> {
        let is_active = self
            .active_printer_drag
            .as_ref()
            .is_some_and(|drag| drag.printer_id == *printer_id);
        let label_color = if is_active {
            sampled_brand_color(CONTENT_BRAND_SAMPLE)
        } else {
            Color::from_rgb8(0x5a, 0x66, 0x78)
        };
        let interaction = if is_active {
            iced::mouse::Interaction::Grabbing
        } else {
            iced::mouse::Interaction::Grab
        };

        let handle = container(
            text("drag")
                .size(11)
                .style(theme::Text::Color(label_color)),
        )
        .width(Length::Fixed(PRINTER_DRAG_HANDLE_WIDTH))
        .padding([12, 8])
        .align_x(iced::alignment::Horizontal::Center)
        .style(theme::Container::Box);

        mouse_area(handle)
            .on_press(Message::StartPrinterReorderDrag(printer_id.clone()))
            .interaction(interaction)
            .into()
    }

    fn printer_row(&self, record: &PrinterRecord, index: usize, total: usize) -> Element<'_, Message> {
        let is_selected =
            !self.manual_pricing_selected && self.selected_printer.as_ref() == Some(&record.id);
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

        let card = BadgeOverlay::new(base, self.recording_badge(is_recording), is_recording)
            .margin(6.0);
        let row = row![self.printer_drag_handle(&record.id), card]
            .spacing(PRINTER_ROW_SPACING)
            .width(Length::Fill)
            .align_items(Alignment::Center);

        mouse_area(row)
            .on_move(move |point| {
                let drop_index = if point.y < PRINTER_DROP_SPLIT_Y {
                    index
                } else {
                    index + 1
                };
                Message::HoverPrinterReorderDrop(drop_index)
            })
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
                        .placeholder("Auto match")
                        .style(profile_pick_list_style())
                        .menu_style(profile_pick_list_menu_style());
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
                self.boxed_printer_tab_scroll_view(if let Some(record) = record {
                    self.printer_oids_view(record)
                } else if selection_missing {
                    self.empty_printer_tab_view("Selected printer not found.")
                } else {
                    self.empty_printer_tab_view("Select a printer to edit OIDs.")
                })
            }
            PrinterTab::Recording => self.recording_tab_view(),
            PrinterTab::Pricing => self.pricing_tab_view(),
            PrinterTab::AddPrinters => {
                self.boxed_printer_tab_scroll_view(self.printer_add_printers_view())
            }
        };

        let content = column![self.printer_tab_bar(), header, body].spacing(12);

        container(content)
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Custom(right_content_panel_style()))
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

        self.printer_tab_scroll_view(content, 24.0)
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

    fn manual_input<'a, F>(
        &self,
        label: &str,
        placeholder: &str,
        value: &'a str,
        on_change: F,
    ) -> Element<'a, Message>
    where
        F: Fn(String) -> Message + 'static,
    {
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

    fn pricing_input<'a>(
        &self,
        label: &str,
        placeholder: &str,
        value: &'a str,
        on_change: fn(String) -> Message,
    ) -> Element<'a, Message> {
        self.manual_input(label, placeholder, value, on_change)
    }

    fn oids_input<'a>(
        &self,
        label: &str,
        placeholder: &str,
        value: &'a str,
        on_change: fn(String) -> Message,
    ) -> Element<'a, Message> {
        self.manual_input(label, placeholder, value, on_change)
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
        end_unlocked: bool,
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

        let start = self.recording_readonly_value(start_value, Length::FillPortion(1));
        let end: Element<'_, Message> = if end_unlocked {
            row![
                button(text("=").size(13))
                    .on_press(Message::RecordingEndResetToPolled(category))
                    .padding(2)
                    .style(theme::Button::custom(indicator_button_style(
                        Color::from_rgb8(0x1f, 0x2a, 0x37),
                    ))),
                text_input("N/A", end_value)
                    .on_input(move |value| Message::RecordingEndChanged { category, value })
                    .padding(4)
                    .size(12)
                    .width(Length::Fill),
            ]
            .spacing(6)
            .align_items(Alignment::Center)
            .width(Length::FillPortion(1))
            .into()
        } else {
            self.recording_readonly_value(end_value, Length::FillPortion(1))
        };
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

    fn recording_readonly_value(&self, value: &str, width: Length) -> Element<'_, Message> {
        let display = if value.trim().is_empty() {
            "N/A".to_string()
        } else {
            value.to_string()
        };
        container(
            text(display)
                .size(13)
                .width(Length::Fill)
                .horizontal_alignment(Horizontal::Right)
                .style(theme::Text::Color(Color::BLACK)),
        )
        .padding(4)
        .width(width)
        .style(theme::Container::Box)
        .into()
    }

    fn recording_end_toggle_button(&self, end_fields_unlocked: bool) -> Element<'_, Message> {
        let icon_bytes = if end_fields_unlocked {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../assets/unlocked-svgrepo-com.svg"
            ))
            .as_slice()
        } else {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../assets/locked-svgrepo-com.svg"
            ))
            .as_slice()
        };
        let icon = iced::widget::svg(iced::widget::svg::Handle::from_memory(icon_bytes))
            .width(16)
            .height(16)
            .style(|_theme, _status| iced::widget::svg::Style { color: None });

        mouse_area(icon)
            .interaction(iced::mouse::Interaction::Pointer)
            .on_press(Message::RecordingEndFieldsUnlockedChanged(
                !end_fields_unlocked,
            ))
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

        container(
            row![console, debug_panel]
                .spacing(16)
                .align_items(Alignment::Start),
        )
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::Container::Custom(right_content_panel_style()))
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
