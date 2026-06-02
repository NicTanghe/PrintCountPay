const PRINTER_DROP_SPLIT_Y: f32 = 32.0;
const STATISTICS_CHART_SVG_WIDTH: f32 = 1000.0;
const STATISTICS_CHART_SVG_HEIGHT: f32 = 220.0;
const STATISTICS_CHART_PAD_LEFT: f32 = 18.0;
const STATISTICS_CHART_PAD_RIGHT: f32 = 10.0;
const STATISTICS_CHART_PAD_TOP: f32 = 12.0;
const STATISTICS_CHART_PAD_BOTTOM: f32 = 12.0;
const STATISTICS_CHART_CONTAINER_PAD_LEFT: f32 = 12.0;
const STATISTICS_CHART_CONTAINER_PAD_RIGHT: f32 = 12.0;
const STATISTICS_CHART_CONTAINER_PAD_TOP: f32 = 8.0;
const STATISTICS_CHART_CONTAINER_PAD_BOTTOM: f32 = 8.0;
const STATISTICS_DATE_CONTROLS_INLINE_MIN_WIDTH: f32 = 626.0;

impl PrintCountApp {
    fn tab_bar(&self) -> Element<'_, Message> {
        let mut left_tabs = row![self.tab_button(Tab::Printers, "Printers")]
            .spacing(8)
            .align_items(Alignment::Center);

        if self.advanced_mode {
            left_tabs = left_tabs.push(self.statistics_tab_button());
            left_tabs = left_tabs.push(self.tab_button(Tab::Debug, "Debug"));
        }

        left_tabs.into()
    }

    fn window_controls_bar(&self) -> Element<'_, Message> {
        let right_controls = row![
            self.sync_role_indicator(),
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

    fn sync_role_indicator(&self) -> Element<'_, Message> {
        let (mark, label, color) = match self.sync_role {
            SyncRole::Master => ("I", "Master", sampled_brand_color(CONTROLS_BRAND_SAMPLE)),
            SyncRole::Client => ("II", "Client", Color::from_rgb8(0x48, 0x8d, 0x6b)),
            SyncRole::Searching => ("...", "Sync", Color::from_rgb8(0x8c, 0x94, 0xa3)),
        };

        let content = row![
            text(mark)
                .size(12)
                .style(theme::Text::Color(color)),
            text(label)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x2a, 0x2f, 0x39))),
        ]
        .spacing(6)
        .align_items(Alignment::Center);

        container(content)
            .height(Length::Fixed(24.0))
            .padding([4, 8])
            .style(theme::Container::Custom(sync_role_indicator_style(color)))
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

    fn statistics_tab_button(&self) -> Element<'_, Message> {
        let active = self.active_tab == Tab::Statistics;
        let style: Box<
            dyn Fn(&Theme, iced::widget::button::Status) -> iced::widget::button::Style,
        > = if active {
            Box::new(solid_brand_button_style(SIDEBAR_BRAND_SAMPLE))
        } else {
            Box::new(theme::Button::custom(top_controls_button_style()))
        };

        let icon_color = if active {
            Color::WHITE
        } else {
            Color::from_rgb8(0x2f, 0x36, 0x42)
        };
        let icon = row![
            self.statistics_tab_bar(6.0, 10.0, icon_color),
            self.statistics_tab_bar(6.0, 7.0, icon_color),
            self.statistics_tab_bar(6.0, 13.0, icon_color),
        ]
        .spacing(2)
        .align_items(Alignment::End);
        let content = row![
            icon,
            text("Statistics")
                .size(12)
                .style(theme::Text::Color(icon_color)),
        ]
        .spacing(6)
        .align_items(Alignment::Center);

        self.top_bar_content_button(content.into(), style, Message::SelectTab(Tab::Statistics))
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
        self.top_bar_button(
            label,
            theme::Button::custom(top_controls_button_style()),
            message,
        )
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

        self.top_bar_content_button(label.into(), style, message)
    }

    fn top_bar_content_button<'a>(
        &self,
        content: Element<'a, Message>,
        style: impl Fn(&Theme, iced::widget::button::Status) -> iced::widget::button::Style + 'static,
        message: Message,
    ) -> Element<'a, Message> {
        button(content)
            .style(style)
            .padding([4, 8])
            .on_press(message)
            .into()
    }

    fn statistics_tab_bar(&self, width: f32, height: f32, color: Color) -> Element<'_, Message> {
        container(
            Space::new()
                .width(Length::Fixed(width))
                .height(Length::Fixed(height)),
        )
        .width(Length::Fixed(width))
        .height(Length::Fixed(height))
        .style(theme::Container::Custom(statistics_tab_icon_style(color)))
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
            let bill_id = bill.id.clone();
            let bill_locked = bill.locked;
            let subject_input = text_input("Bill subject", &bill.subject)
                .on_input(Message::ManualPricingBillSubjectChanged)
                .padding(6)
                .size(12)
                .width(Length::Fixed(220.0));
            let delete_button: Element<'_, Message> = if bill_locked {
                button("Unlock to delete")
                    .style(theme::Button::custom(muted_content_button_style()))
                    .into()
            } else {
                button("Delete bill")
                    .style(theme::Button::custom(muted_content_button_style()))
                    .on_press(Message::DeleteSelectedManualPricingBill)
                    .into()
            };

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
                self.manual_bill_lock_toggle_button(bill_id, bill_locked),
                delete_button,
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
                ]
                .spacing(4),
                horizontal_space(),
                button("Reset")
                    .style(theme::Button::custom(muted_content_button_style()))
                    .on_press(Message::ResetManualPricingCalculator),
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

        let mut content = column![title_block].spacing(12).height(Length::Fill);

        if self.selected_manual_bill().is_none() {
            content = content.push(self.manual_pricing_tab_bar());
        }

        content = content.push(
            scrollable(
                container(self.manual_pricing_body_view()).padding(iced::Padding {
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

    fn manual_booklet_tabs_view(
        &self,
        manual: &ManualPricingSettings,
        active_booklet_index: Option<usize>,
    ) -> Element<'_, Message> {
        let mut tabs = row![
            button(text("Order").size(12))
                .padding([4, 10])
                .style(theme::Button::custom(firefox_tab_style(
                    active_booklet_index.is_none(),
                )))
                .on_press(Message::SelectManualBookletTab(None))
        ]
        .spacing(4)
        .align_items(Alignment::Center);

        for (index, booklet) in manual.booklets.iter().enumerate() {
            let label = booklet.display_name(index);
            tabs = tabs.push(
                button(text(label).size(12))
                    .padding([4, 10])
                    .style(theme::Button::custom(firefox_tab_style(
                        active_booklet_index == Some(index),
                    )))
                    .on_press(Message::SelectManualBookletTab(Some(index))),
            );
        }

        tabs = tabs.push(
            button("New booklet")
                .style(theme::Button::custom(solid_brand_button_style(
                    CONTENT_BRAND_SAMPLE,
                )))
                .on_press(Message::ManualPricingBookletAdded),
        );

        tabs.into()
    }

    fn manual_receipt_cell(value: &str, width: usize, align_right: bool) -> String {
        if align_right {
            format!("{value:>width$}")
        } else {
            format!("{value:<width$}")
        }
    }

    fn manual_receipt_chunks(value: impl Into<String>, width: usize) -> Vec<String> {
        let value = value.into().replace(" EUR", " €");
        if value.trim().is_empty() {
            return vec![String::new()];
        }

        let mut lines = Vec::new();
        let mut current = String::new();

        for word in value.split_whitespace() {
            if word.chars().count() > width {
                if !current.is_empty() {
                    lines.push(current);
                    current = String::new();
                }
                let mut chunk = String::new();
                for character in word.chars() {
                    if chunk.chars().count() == width {
                        lines.push(chunk);
                        chunk = String::new();
                    }
                    chunk.push(character);
                }
                if !chunk.is_empty() {
                    current = chunk;
                }
                continue;
            }

            let next_len = if current.is_empty() {
                word.chars().count()
            } else {
                current.chars().count() + 1 + word.chars().count()
            };

            if next_len > width && !current.is_empty() {
                lines.push(current);
                current = word.to_string();
            } else {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
            }
        }

        if !current.is_empty() {
            lines.push(current);
        }

        if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        }
    }

    fn manual_receipt_rows(
        what: impl Into<String>,
        amount: impl Into<String>,
        unit_price: impl Into<String>,
        total_price: impl Into<String>,
    ) -> Vec<String> {
        const WHAT_WIDTH: usize = 34;
        const AMOUNT_WIDTH: usize = 8;
        const UNIT_WIDTH: usize = 12;
        const TOTAL_WIDTH: usize = 12;

        let what = Self::manual_receipt_chunks(what, WHAT_WIDTH);
        let amount = Self::manual_receipt_chunks(amount, AMOUNT_WIDTH);
        let unit_price = Self::manual_receipt_chunks(unit_price, UNIT_WIDTH);
        let total_price = Self::manual_receipt_chunks(total_price, TOTAL_WIDTH);
        let line_count = what
            .len()
            .max(amount.len())
            .max(unit_price.len())
            .max(total_price.len());

        (0..line_count)
            .map(|index| {
                format!(
                    "{} | {} | {} | {}",
                    Self::manual_receipt_cell(
                        what.get(index).map(String::as_str).unwrap_or(""),
                        WHAT_WIDTH,
                        false,
                    ),
                    Self::manual_receipt_cell(
                        amount.get(index).map(String::as_str).unwrap_or(""),
                        AMOUNT_WIDTH,
                        true,
                    ),
                    Self::manual_receipt_cell(
                        unit_price.get(index).map(String::as_str).unwrap_or(""),
                        UNIT_WIDTH,
                        true,
                    ),
                    Self::manual_receipt_cell(
                        total_price.get(index).map(String::as_str).unwrap_or(""),
                        TOTAL_WIDTH,
                        true,
                    ),
                )
            })
            .collect()
    }

    fn manual_average_unit_price(total_cents: u64, amount: u64) -> String {
        if amount == 0 {
            return "N/A".to_string();
        }

        format_cents((total_cents.saturating_add(amount / 2)) / amount)
    }

    fn manual_receipt_line_label(
        &self,
        manual: &ManualPricingSettings,
        line_item: &ManualPricingLineItem,
    ) -> String {
        let print_mode = match line_item.print_mode {
            ManualPrintMode::Bw => "BW",
            ManualPrintMode::Color => "FC",
        };
        let mut parts = vec![line_item.size.to_string(), print_mode.to_string()];
        if let Some(modifier_index) = line_item.modifier_index
            && let Some(modifier) = manual.modifiers.get(modifier_index)
        {
            parts.push(modifier.display_name());
        }
        if line_item.double_sided {
            parts.push("RV".to_string());
        }
        parts.join(" ")
    }

    fn manual_receipt_finisher_label(
        &self,
        manual: &ManualPricingSettings,
        finisher_item: &ManualFinisherLineItem,
    ) -> String {
        match finisher_item.finisher_type {
            ManualFinisherType::Laminate => format!("Laminate {}", finisher_item.laminate_size),
            ManualFinisherType::Folding => "Folding".to_string(),
            ManualFinisherType::Binding => manual
                .binding_modifier(finisher_item.binding_modifier_index)
                .map(|modifier| {
                    format!(
                        "Binding {} {}",
                        finisher_item.binding_size,
                        modifier.display_name()
                    )
                })
                .unwrap_or_else(|| "Binding".to_string()),
        }
    }

    fn manual_order_summary_receipt_rows(
        &self,
        manual: &ManualPricingSettings,
        totals: &ManualPricingTotals,
    ) -> Vec<String> {
        let mut rows = Self::manual_receipt_rows(
            "What",
            "Amount",
            "Unit price",
            "Total price",
        );

        for (index, line_item) in manual.line_items.iter().enumerate() {
            let Some(ManualLineState::Ready(line)) = totals.line_states.get(index) else {
                continue;
            };
            rows.extend(Self::manual_receipt_rows(
                self.manual_receipt_line_label(manual, line_item),
                line.sides.to_string(),
                Self::manual_average_unit_price(line.total_cents, line.sides),
                format_cents(line.total_cents),
            ));
        }

        for (index, finisher_item) in manual.finisher_items.iter().enumerate() {
            let Some(ManualFinisherState::Ready(finisher)) = totals.finisher_states.get(index)
            else {
                continue;
            };
            rows.extend(Self::manual_receipt_rows(
                self.manual_receipt_finisher_label(manual, finisher_item),
                finisher.amount.to_string(),
                format_cents(finisher.unit_price_cents),
                format_cents(finisher.total_cents),
            ));
        }

        if manual.cutting_enabled {
            rows.extend(Self::manual_receipt_rows(
                "Cutting",
                "1",
                format_cents(totals.cutting_cents),
                format_cents(totals.cutting_cents),
            ));
        }

        for (index, booklet) in manual.booklets.iter().enumerate() {
            let Some(booklet_totals) = totals.booklet_totals.get(index) else {
                continue;
            };
            let (Some(price_per_booklet), Some(copies), Some(total_cents)) = (
                booklet_totals.price_per_booklet_cents,
                booklet_totals.copies,
                booklet_totals.total_cents,
            ) else {
                continue;
            };
            rows.extend(Self::manual_receipt_rows(
                booklet.display_name(index),
                copies.to_string(),
                format_cents(price_per_booklet),
                format_cents(total_cents),
            ));
        }

        rows
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

        for size in ManualPrintSize::FLAT_PRICED {
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

        let mut tiered_prices = iced::widget::Column::new().spacing(12);
        for size in ManualPrintSize::TIERED_PRICED {
            tiered_prices = tiered_prices.push(self.manual_tiered_price_box(size));
        }
        tiered_prices = tiered_prices.push(
            text("A5, A6, A7, and buisnesscard print prices are divided from A3 tiers.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        );

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
            text("Paper modifiers are charged per sheet. Configure each print size separately for each modifier.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(8);

        for (index, modifier) in manual.modifiers.iter().enumerate() {
            modifier_setup = modifier_setup.push(self.manual_pricing_modifier_row(index, modifier));
        }

        let modifier_setup = container(modifier_setup)
            .padding(12)
            .width(Length::Fill)
            .style(theme::Container::Box);

        let active_booklet_index = self
            .selected_manual_booklet_index
            .filter(|index| manual.booklets.get(*index).is_some());
        let active_booklet = active_booklet_index.and_then(|index| manual.booklets.get(index));
        let active_booklet_totals =
            active_booklet_index.and_then(|index| totals.booklet_totals.get(index));
        let line_items = manual.line_items(active_booklet_index);
        let finisher_items = manual.finisher_items(active_booklet_index);
        let line_states = active_booklet_totals
            .map(|booklet_totals| &booklet_totals.line_states)
            .unwrap_or(&totals.line_states);
        let finisher_states = active_booklet_totals
            .map(|booklet_totals| &booklet_totals.finisher_states)
            .unwrap_or(&totals.finisher_states);
        let per_book = active_booklet_index.is_some();
        let active_title = active_booklet_index
            .and_then(|index| manual.booklets.get(index).map(|booklet| booklet.display_name(index)))
            .unwrap_or_else(|| "Order lines".to_string());
        let line_hint = if per_book {
            "Set printed sides, paper type, and finishers for one booklet. The multiplier is applied in the summary."
        } else {
            "Use sheets for paper count and printed sides for actual printed faces."
        };

        let mut header_actions = row![
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
        .align_items(Alignment::Center);

        if let Some(index) = active_booklet_index {
            header_actions = header_actions.push(
                button("Delete booklet")
                    .style(theme::Button::custom(muted_content_button_style()))
                    .on_press(Message::ManualPricingBookletRemoved(index)),
            );
        }

        let mut calculator_section = column![
            self.manual_booklet_tabs_view(manual, active_booklet_index),
            row![
                text(active_title)
                    .size(15)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                horizontal_space(),
                header_actions,
            ]
            .spacing(10)
            .align_items(Alignment::Center),
            text(line_hint)
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(8);

        if let (Some(index), Some(booklet)) = (active_booklet_index, active_booklet) {
            let name_input = text_input("Booklet name", &booklet.name_input)
                .on_input(move |value| Message::ManualPricingBookletNameChanged(index, value))
                .padding(6)
                .size(12)
                .width(Length::Fill);
            let copies_input = text_input("1", &booklet.copies_input)
                .on_input(move |value| Message::ManualPricingBookletCopiesChanged(index, value))
                .padding(6)
                .size(12)
                .width(Length::Fixed(96.0));

            calculator_section = calculator_section.push(
                row![
                    column![
                        text("Name")
                            .size(12)
                            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                        name_input,
                    ]
                    .spacing(4)
                    .width(Length::Fill),
                    column![
                        text("Multiplier")
                            .size(12)
                            .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                        copies_input,
                    ]
                    .spacing(4),
                ]
                .spacing(12)
                .align_items(Alignment::Center),
            );
        }

        for (index, line_item) in line_items.iter().enumerate() {
            let line_state = line_states
                .get(index)
                .cloned()
                .unwrap_or(ManualLineState::Invalid);
            calculator_section = calculator_section.push(self.manual_pricing_line_item_row(
                index, line_item, line_state, per_book,
            ));
        }

        calculator_section = calculator_section.push(
            text("Finishers")
                .size(13)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
        );

        if finisher_items.is_empty() {
            calculator_section = calculator_section.push(
                text("No finishers added. Use Add finisher for laminate, folding, or binding.")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            );
        } else {
            for (index, finisher_item) in finisher_items.iter().enumerate() {
                let finisher_state = finisher_states
                    .get(index)
                    .cloned()
                    .unwrap_or(ManualFinisherState::Invalid);
                calculator_section = calculator_section.push(self.manual_pricing_finisher_row(
                    index,
                    finisher_item,
                    finisher_state,
                    per_book,
                ));
            }
        }

        if !per_book {
            calculator_section = calculator_section.push(
                checkbox(manual.cutting_enabled)
                    .label("Cutting (+3 EUR)")
                    .on_toggle(Message::ManualPricingCuttingChanged)
                    .size(12)
                    .style(theme::Checkbox::custom(brand_checkbox_style(
                        CONTENT_BRAND_SAMPLE,
                    ))),
            );
        }

        calculator_section = calculator_section
            .push(self.manual_input(
                "Discount (%)",
                "0",
                &manual.discount_input,
                Message::ManualPricingDiscountChanged,
            ))
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
                            Message::ManualPricingRoundingToggled(
                                ManualRoundingMode::FiveCents,
                                value,
                            )
                        })
                        .size(12)
                        .style(theme::Checkbox::custom(brand_checkbox_style(
                            CONTENT_BRAND_SAMPLE,
                        ))),
                    checkbox(manual.rounding_mode == ManualRoundingMode::HalfEuro)
                        .label("Round down to 0.50 EUR")
                        .on_toggle(|value| {
                            Message::ManualPricingRoundingToggled(
                                ManualRoundingMode::HalfEuro,
                                value,
                            )
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
                text("Fix any invalid line, booklet multiplier, finisher, size price, modifier price, finisher price, or discount input to calculate the total.")
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
        ]
        .spacing(6);

        if let Some(booklet_totals) = active_booklet_totals {
            let booklet_lines_label = booklet_totals
                .lines_total_cents
                .map(format_cents)
                .unwrap_or_else(|| "Invalid line input".to_string());
            let booklet_finishers_label = booklet_totals
                .finishers_total_cents
                .map(format_cents)
                .unwrap_or_else(|| "Invalid finisher input".to_string());
            let booklet_subtotal_label = booklet_totals
                .subtotal_cents
                .map(format_cents)
                .unwrap_or_else(|| "Invalid booklet input".to_string());
            let booklet_discount_label = booklet_totals
                .discount_cents
                .map(|value| format!("-{}", format_cents(value)))
                .unwrap_or_else(|| "Invalid discount input".to_string());
            let price_per_booklet_label = booklet_totals
                .price_per_booklet_cents
                .map(format_cents)
                .unwrap_or_else(|| "N/A".to_string());
            let copies_label = booklet_totals
                .copies
                .map(|value| value.to_string())
                .unwrap_or_else(|| "Invalid multiplier".to_string());
            let booklet_total_label = booklet_totals
                .total_cents
                .map(format_cents)
                .unwrap_or_else(|| "N/A".to_string());

            summary = summary
                .push(self.value_line("Lines per booklet", Some(booklet_lines_label)))
                .push(self.value_line("Finishers per booklet", Some(booklet_finishers_label)))
                .push(self.value_line("1 booklet before discount", Some(booklet_subtotal_label)))
                .push(self.value_line("Discount per booklet", Some(booklet_discount_label)))
                .push(self.value_line("Price of 1 booklet", Some(price_per_booklet_label)))
                .push(self.value_line("Multiplier", Some(copies_label)))
                .push(self.value_line("Booklet total", Some(booklet_total_label)))
                .push(self.value_line("Final total", Some(total_label)));
        } else {
            for row_text in self.manual_order_summary_receipt_rows(manual, &totals) {
                summary = summary.push(
                    text(row_text)
                        .size(10)
                        .font(iced::Font::MONOSPACE)
                        .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37))),
                );
            }

            let footer_rows = [
                ("Subtotal before discount", subtotal_label),
                ("Discount", discount_label),
                ("Before rounding", before_rounding_label),
                ("Rounding", manual.rounding_mode.to_string()),
                ("Final total", total_label),
            ];
            for (label, value) in footer_rows {
                for row_text in Self::manual_receipt_rows(label, "", "", value) {
                    summary = summary.push(
                        text(row_text)
                            .size(10)
                            .font(iced::Font::MONOSPACE)
                            .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37))),
                    );
                }
            }
        }

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
            ManualPricingTab::Finishers => column![self.manual_pricing_finishers_config_view()]
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

        let folding_prices = container(
            column![
                text("Other finishers")
                    .size(15)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                text("Folding uses one flat unit price. The calculator amount field controls how many times it is applied.")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                self.manual_input(
                    "Folding (EUR)",
                    "0.00",
                    &manual.folding_input,
                    Message::ManualPricingFoldingPriceChanged,
                ),
            ]
            .spacing(8),
        )
        .padding(12)
        .width(Length::Fill)
        .style(theme::Container::Box);

        let mut binding_prices = column![
            row![
                text("Binding pricing")
                    .size(15)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                horizontal_space(),
                button("Add modifier")
                    .style(theme::Button::custom(solid_brand_button_style(
                        CONTENT_BRAND_SAMPLE,
                    )))
                    .on_press(Message::ManualPricingBindingModifierAdded),
            ]
            .align_items(Alignment::Center),
            text("Binding modifiers are charged per binding. Configure each print size separately for each modifier.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(8);

        if manual.binding_modifiers.is_empty() {
            binding_prices = binding_prices.push(
                text("No binding modifiers configured. Add a modifier before pricing binding finishers.")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            );
        } else {
            for (index, modifier) in manual.binding_modifiers.iter().enumerate() {
                binding_prices =
                    binding_prices.push(self.manual_pricing_binding_modifier_row(index, modifier));
            }
        }

        let binding_prices = container(binding_prices)
            .padding(12)
            .width(Length::Fill)
            .style(theme::Container::Box);

        column![
            self.manual_pricing_storage_controls_view(),
            laminate_prices,
            folding_prices,
            binding_prices,
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
        per_book: bool,
    ) -> Element<'_, Message> {
        let modifier_choices =
            self.manual_modifier_choices(line_item.size, line_item.modifier_index);
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
        let modifier_picker = pick_list(modifier_choices, Some(selected_modifier), move |choice| {
            Message::ManualPricingLineModifierChanged(index, choice.index)
        })
        .placeholder("Modifier")
        .text_size(11)
        .width(Length::Fill)
        .style(profile_pick_list_style())
        .menu_style(profile_pick_list_menu_style());
        let sides_input = text_input("0", &line_item.sides_input)
            .on_input(move |value| Message::ManualPricingLineSidesChanged(index, value))
            .padding(6)
            .size(12);
        let double_sided_toggle = checkbox(line_item.double_sided)
            .label("RV")
            .on_toggle(move |value| Message::ManualPricingLineDoubleSidedChanged(index, value))
            .size(12)
            .style(theme::Checkbox::custom(brand_checkbox_style(
                CONTENT_BRAND_SAMPLE,
            )));
        let remove_button =
            self.manual_remove_icon_button(Message::ManualPricingLineRemoved(index));
        let placeholder_label = || {
            text(" ")
                .size(12)
                .style(theme::Text::Color(Color::TRANSPARENT))
        };

        let sides_label = if per_book {
            "Sides/book"
        } else {
            "Zijden"
        };
        let sheets_label = if per_book {
            "Sheets/book"
        } else {
            "Vellen"
        };
        let sides_width = if per_book { 68.0 } else { 42.0 };
        let sheets_width = if per_book { 76.0 } else { 54.0 };
        let sides_input = sides_input.width(Length::Fixed(sides_width));
        let sheets_value =
            self.recording_readonly_value(&line_item.sheets_input, Length::Fixed(sheets_width));

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
                text(sides_label)
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                sides_input,
            ]
            .spacing(4)
            .width(Length::Fixed(sides_width)),
            column![
                text(sheets_label)
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                sheets_value,
            ]
            .spacing(4)
            .width(Length::Fixed(sheets_width)),
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
            ManualLineState::Invalid => {
                text("Enter valid sheets, sides, size pricing, and modifier pricing.")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0xe0, 0x4f, 0x4f)))
            }
            ManualLineState::Ready(line) => {
                text(manual_line_summary(&line))
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)))
            }
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
        per_book: bool,
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
        let (size_control, modifier_control): (Element<'_, Message>, Element<'_, Message>) =
            match finisher_item.finisher_type {
                ManualFinisherType::Laminate => (
                    pick_list(
                        &ManualLaminateSize::ALL[..],
                        Some(finisher_item.laminate_size),
                        move |size| Message::ManualPricingFinisherSizeChanged(index, size),
                    )
                    .placeholder("Size")
                    .text_size(11)
                    .width(Length::Fill)
                    .style(profile_pick_list_style())
                    .menu_style(profile_pick_list_menu_style())
                    .into(),
                    self.recording_readonly_value("n/a", Length::Fill),
                ),
                ManualFinisherType::Binding => {
                    let binding_size_picker = pick_list(
                        &ManualPrintSize::ALL[..],
                        Some(finisher_item.binding_size),
                        move |size| Message::ManualPricingFinisherBindingSizeChanged(index, size),
                    )
                    .placeholder("Size")
                    .text_size(11)
                    .width(Length::Fill)
                    .style(profile_pick_list_style())
                    .menu_style(profile_pick_list_menu_style());
                    let binding_modifier_choices = self.manual_binding_modifier_choices(
                        finisher_item.binding_size,
                        finisher_item.binding_modifier_index,
                    );
                    let selected_binding_modifier = binding_modifier_choices
                        .iter()
                        .find(|choice| choice.index == finisher_item.binding_modifier_index)
                        .cloned();
                    (
                        binding_size_picker.into(),
                        pick_list(
                            binding_modifier_choices,
                            selected_binding_modifier,
                            move |choice| {
                                Message::ManualPricingFinisherBindingModifierChanged(
                                    index,
                                    choice.index,
                                )
                            },
                        )
                        .placeholder("Modifier")
                        .text_size(11)
                        .width(Length::Fill)
                        .style(profile_pick_list_style())
                        .menu_style(profile_pick_list_menu_style())
                        .into(),
                    )
                }
                ManualFinisherType::Folding => (
                    self.recording_readonly_value("n/a", Length::Fixed(84.0)),
                    self.recording_readonly_value("n/a", Length::Fill),
                ),
            };
        let amount_input = text_input("0", &finisher_item.amount_input)
            .on_input(move |value| Message::ManualPricingFinisherAmountChanged(index, value))
            .padding(6)
            .size(12)
            .width(Length::Fixed(84.0));
        let remove_button = button("Remove")
            .style(theme::Button::custom(muted_content_button_style()))
            .on_press(Message::ManualPricingFinisherRemoved(index));

        let amount_label = if per_book {
            "Per book"
        } else {
            "Amount"
        };

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
            .spacing(4)
            .width(Length::FillPortion(2)),
            column![
                text("Modifier")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                modifier_control,
            ]
            .spacing(4)
            .width(Length::FillPortion(2)),
            column![
                text(amount_label)
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
            ManualFinisherState::Invalid => text("Enter a valid amount and finisher price.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0xe0, 0x4f, 0x4f))),
            ManualFinisherState::Ready(finisher) => {
                let amount_summary = if finisher.booklet_copies > 1 {
                    format!(
                        "{} ({} per book x {} booklets)",
                        finisher.amount, finisher.amount_per_book, finisher.booklet_copies
                    )
                } else {
                    finisher.amount.to_string()
                };
                text(format!(
                    "{} x {} = {}",
                    amount_summary,
                    finisher.label,
                    format_cents(finisher.total_cents),
                ))
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x1f, 0x2a, 0x37)))
            }
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
                manual
                    .bw_tier_input(size, ManualBwTier::FirstFive)
                    .unwrap_or(""),
                move |value| Message::ManualPricingBwTierChanged(
                    size,
                    ManualBwTier::FirstFive,
                    value
                ),
            ),
            self.manual_input(
                "6-10 sides (EUR)",
                "0.00",
                manual
                    .bw_tier_input(size, ManualBwTier::NextFive)
                    .unwrap_or(""),
                move |value| Message::ManualPricingBwTierChanged(
                    size,
                    ManualBwTier::NextFive,
                    value
                ),
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
                manual
                    .color_tier_input(size, ManualColorTier::FirstFive)
                    .unwrap_or(""),
                move |value| {
                    Message::ManualPricingColorTierChanged(size, ManualColorTier::FirstFive, value)
                },
            ),
            self.manual_input(
                "6+ sides (EUR)",
                "0.00",
                manual
                    .color_tier_input(size, ManualColorTier::Rest)
                    .unwrap_or(""),
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
                row![
                    bw.width(Length::FillPortion(1)),
                    color.width(Length::FillPortion(1))
                ]
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
                    .width(Length::Fixed(96.0))
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

        let mut applies = column![
            text("Per-size setup")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
        ]
        .spacing(8);
        for size in ManualPrintSize::ALL {
            applies = applies.push(size_row(size));
        }

        container(column![controls, applies].spacing(8))
            .padding(10)
            .width(Length::Fill)
            .style(theme::Container::Box)
            .into()
    }

    fn manual_pricing_binding_modifier_row(
        &self,
        index: usize,
        modifier: &ManualBindingModifier,
    ) -> Element<'_, Message> {
        let name_input = text_input("Spiral", &modifier.name_input)
            .on_input(move |value| Message::ManualPricingBindingModifierNameChanged(index, value))
            .padding(6)
            .size(12)
            .width(Length::Fill);
        let remove_button = button("Remove")
            .style(theme::Button::custom(muted_content_button_style()))
            .on_press(Message::ManualPricingBindingModifierRemoved(index));

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
                    .width(Length::Fixed(96.0))
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                checkbox(enabled)
                    .label("Applies")
                    .on_toggle(move |value| {
                        Message::ManualPricingBindingModifierAppliesChanged(index, size, value)
                    })
                    .size(12)
                    .style(theme::Checkbox::custom(brand_checkbox_style(
                        CONTENT_BRAND_SAMPLE,
                    ))),
                text_input("0.00", price_value)
                    .on_input(move |value| {
                        Message::ManualPricingBindingModifierPriceChanged(index, size, value)
                    })
                    .padding(6)
                    .size(12)
                    .width(Length::Fixed(110.0)),
                text("EUR per binding")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            ]
            .spacing(10)
            .align_items(Alignment::Center)
        };

        let mut applies = column![
            text("Per-size setup")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
        ]
        .spacing(8);
        for size in ManualPrintSize::ALL {
            applies = applies.push(size_row(size));
        }

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
            && !choices
                .iter()
                .any(|choice| choice.index == Some(selected_index))
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

    fn manual_binding_modifier_choices(
        &self,
        size: ManualPrintSize,
        selected_index: Option<usize>,
    ) -> Vec<ManualModifierChoice> {
        let manual = self.active_manual_pricing();
        let mut choices = Vec::new();

        for (index, modifier) in manual.binding_modifiers.iter().enumerate() {
            if modifier.applies_to_size(size) {
                choices.push(ManualModifierChoice {
                    index: Some(index),
                    label: modifier.display_name(),
                });
            }
        }

        if let Some(selected_index) = selected_index
            && !choices
                .iter()
                .any(|choice| choice.index == Some(selected_index))
        {
            let label = manual
                .binding_modifiers
                .get(selected_index)
                .map(|modifier| format!("{} (not for {size})", modifier.display_name()))
                .unwrap_or_else(|| "Missing binding modifier".to_string());
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
                text("A0-A7, buisnesscard, paper types")
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

    fn manual_bill_lock_icon(&self, locked: bool) -> Element<'_, Message> {
        let icon_bytes = if locked {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../assets/locked-svgrepo-com.svg"
            ))
            .as_slice()
        } else {
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../assets/unlocked-svgrepo-com.svg"
            ))
            .as_slice()
        };
        let icon = iced::widget::svg(iced::widget::svg::Handle::from_memory(icon_bytes))
            .width(16)
            .height(16)
            .style(|_theme, _status| iced::widget::svg::Style { color: None });

        icon.into()
    }

    fn manual_bill_lock_toggle_button(
        &self,
        bill_id: String,
        locked: bool,
    ) -> Element<'_, Message> {
        mouse_area(
            container(self.manual_bill_lock_icon(locked))
                .width(Length::Fixed(24.0))
                .height(Length::Fixed(24.0))
                .align_x(Horizontal::Center)
                .align_y(iced::alignment::Vertical::Center),
        )
        .interaction(iced::mouse::Interaction::Pointer)
        .on_press(Message::ManualPricingBillLockedChanged(bill_id, !locked))
        .into()
    }

    fn manual_pricing_bill_row(&self, bill: &ManualPricingBill) -> Element<'_, Message> {
        let is_selected = self.manual_pricing_selected
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

        let mut card_content = row![
            column![
                text(bill_id.clone())
                    .size(11)
                    .style(theme::Text::Color(secondary_color)),
                text(bill_subject)
                    .size(14)
                    .style(theme::Text::Color(name_color)),
            ]
            .spacing(4)
            .width(Length::Fill),
        ]
        .spacing(8)
        .align_items(Alignment::Center);
        if bill.locked {
            card_content = card_content.push(self.manual_bill_lock_icon(true));
        }

        let card = button(card_content)
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
        let mut list_items = column![].spacing(10);

        if self.active_tab != Tab::Statistics {
            list_items = list_items.push(self.manual_pricing_row());
            for bill in &self.manual_bills {
                list_items = list_items.push(self.manual_pricing_bill_row(bill));
            }
        }

        if self.printers.is_empty() {
            list_items = list_items.push(
                text("No printers discovered or added yet.")
                    .size(14)
                    .style(theme::Text::Color(Color::from_rgb8(0x4a, 0x4a, 0x4a))),
            );
        } else {
            let active_drop_index = self
                .active_printer_drag
                .as_ref()
                .map(|drag| drag.drop_index);
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
            text(if self.active_tab == Tab::Statistics {
                "Statistics"
            } else {
                "Printers"
            })
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
        container(Space::new().height(Length::Fixed(4.0)))
            .width(Length::Fill)
            .height(Length::Fixed(4.0))
            .style(theme::Container::Custom(printer_drop_indicator_style()))
            .into()
    }

    fn printer_row(
        &self,
        record: &PrinterRecord,
        index: usize,
        total: usize,
    ) -> Element<'_, Message> {
        let statistics_mode = self.active_tab == Tab::Statistics;
        let is_selected = if statistics_mode {
            self.statistics_selected_printers.contains(&record.id)
        } else {
            !self.manual_pricing_selected && self.selected_printer.as_ref() == Some(&record.id)
        };
        let is_pending_drag = self
            .pending_printer_drag
            .as_ref()
            .is_some_and(|pending| pending.printer_id == record.id);
        let is_active_drag = self
            .active_printer_drag
            .as_ref()
            .is_some_and(|drag| drag.printer_id == record.id);
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
        let name = record
            .model
            .as_deref()
            .unwrap_or("Unknown name")
            .to_string();
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
            text(name).size(16).style(theme::Text::Color(name_color)),
            details,
        ]
        .spacing(6);

        let base = container(content)
            .width(Length::Fill)
            .padding([14, 16])
            .style(theme::Container::Custom(printer_card_container_style(
                is_selected,
                base_color,
                is_pending_drag || is_active_drag,
            )));

        let card =
            BadgeOverlay::new(base, self.recording_badge(is_recording), is_recording).margin(6.0);
        if statistics_mode {
            return mouse_area(card)
                .interaction(iced::mouse::Interaction::Pointer)
                .on_press(Message::ToggleStatisticsPrinter(record.id.clone()))
                .into();
        }
        mouse_area(card)
            .on_press(Message::StartPrinterReorderDrag(record.id.clone()))
            .on_release(Message::CompletePrinterCardPress(record.id.clone()))
            .on_move(move |point| {
                let drop_index = if point.y < PRINTER_DROP_SPLIT_Y {
                    index
                } else {
                    index + 1
                };
                Message::HoverPrinterReorderDrop(drop_index)
            })
            .on_exit(Message::CancelPendingPrinterReorder(record.id.clone()))
            .into()
    }

    fn statistics_view(&self) -> Element<'_, Message> {
        let time_window = self.statistics_time_window();
        let (range_start, range_end) = self.statistics_selected_date_range();
        let selected_printers: Vec<_> = self
            .printers
            .iter()
            .filter(|record| self.statistics_selected_printers.contains(&record.id))
            .collect();
        let available = available_series(
            &self.statistics_store,
            &self.statistics_selected_printers,
            &self.pricing,
            Some(time_window),
        );
        let aggregated_series = available
            .iter()
            .enumerate()
            .map(|(index, definition)| StatisticsChartSeries {
                key: definition.key.clone(),
                label: definition.label.clone(),
                color: statistics_series_color(&definition.key, index),
                points: aggregate_series_points(
                    &self.statistics_store,
                    &self.statistics_selected_printers,
                    &self.pricing,
                    &definition.key,
                    96,
                    Some(time_window),
                ),
            })
            .collect::<Vec<_>>();
        let visible_series = aggregated_series
            .iter()
            .filter(|series| {
                self.statistics_visible_series.contains(&series.key) && !series.points.is_empty()
            })
            .cloned()
            .collect::<Vec<_>>();
        let chart_bounds = statistics_chart_bounds(&visible_series);
        let series_y_bounds = visible_series
            .iter()
            .filter_map(|series| {
                self.statistics_effective_series_y_bounds(series)
                    .map(|bounds| (series.key.clone(), bounds))
            })
            .collect::<HashMap<_, _>>();

        let header = column![
            text("Statistics")
                .size(20)
                .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            text("Select one or more printers on the left. Poll snapshots are stored every 15 minutes even when no recording is active, and each checked series is summed across the current selection.")
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(4);

        let body: Element<'_, Message> = if selected_printers.is_empty() {
            self.empty_printer_tab_view("Select one or more printers to display statistics.")
        } else {
            let content = column![
                self.statistics_chart_card(
                    &selected_printers,
                    &aggregated_series,
                    &visible_series,
                    chart_bounds,
                    &series_y_bounds,
                    range_start,
                    range_end,
                ),
                self.statistics_series_controls(&aggregated_series),
            ]
            .spacing(12)
            .width(Length::Fill);
            self.printer_tab_scroll_view(content, 16.0)
        };

        container(column![header, body].spacing(12))
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(theme::Container::Custom(right_content_panel_style()))
            .into()
    }

    fn statistics_chart_card(
        &self,
        selected_printers: &[&PrinterRecord],
        aggregated_series: &[StatisticsChartSeries],
        visible_series: &[StatisticsChartSeries],
        chart_bounds: Option<StatisticsChartBounds>,
        series_y_bounds: &HashMap<String, StatisticsSeriesYBounds>,
        range_start: Date,
        range_end: Date,
    ) -> Element<'_, Message> {
        let selected_label = self.statistics_selection_summary(selected_printers);
        let range_label = self.statistics_date_range_summary(range_start, range_end);
        let chart_body: Element<'_, Message> = if aggregated_series.is_empty() {
            self.statistics_chart_empty_state(
                "Waiting for the first stored statistics sample. Poll snapshots are saved every 15 minutes.",
            )
        } else if visible_series.is_empty() {
            self.statistics_chart_empty_state(
                "All series are hidden. Enable at least one toggle below to draw the graph.",
            )
        } else {
            self.statistics_line_chart(
                visible_series,
                chart_bounds.expect("bounds should exist"),
                series_y_bounds,
            )
        };

        container(
            column![
                row![
                    column![
                        text("Combined graph")
                            .size(16)
                            .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                        text(selected_label)
                            .size(12)
                            .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                        text("Aggregation: checked categories are added together across the selected printers.")
                            .size(11)
                            .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                        text("Y scale uses each visible series' own min/max settings.")
                            .size(11)
                            .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                    ]
                    .spacing(4),
                    horizontal_space(),
                    text(format!(
                        "{} visible / {} stored",
                        visible_series.len(),
                        aggregated_series.len()
                    ))
                    .size(11)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                ]
                .spacing(12)
                .align_items(Alignment::Center),
                row![
                    self.statistics_summary_tile(
                        "Printers",
                        selected_printers.len().to_string(),
                    ),
                    self.statistics_summary_tile(
                        "Range",
                        range_label,
                    ),
                    self.statistics_summary_tile(
                        "From",
                        format_calendar_date(range_start),
                    ),
                    self.statistics_summary_tile(
                        "To",
                        format_calendar_date(range_end),
                    ),
                ]
                .spacing(10),
                self.statistics_range_controls(range_start, range_end),
                chart_body,
                if let Some(bounds) = chart_bounds {
                    self.statistics_axis(bounds)
                } else {
                    Space::new()
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .into()
                },
            ]
            .spacing(12),
        )
        .padding(12)
        .width(Length::Fill)
        .style(theme::Container::Box)
        .into()
    }

    fn statistics_series_controls(
        &self,
        aggregated_series: &[StatisticsChartSeries],
    ) -> Element<'_, Message> {
        let body: Element<'_, Message> = if aggregated_series.is_empty() {
            container(
                text("No stored series are available yet.")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            )
            .width(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .into()
        } else {
            let mut series_rows = column![].spacing(10);
            for series in aggregated_series {
                series_rows = series_rows.push(self.statistics_series_toggle_row(series));
            }
            series_rows.into()
        };

        container(
            column![
                text("Series")
                    .size(16)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
                text("Toggle the lines you want to compare. Matching metrics from multiple selected printers are added together into one line. Y Min/Y Max is stored per series.")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                body,
            ]
            .spacing(8),
        )
        .padding(12)
        .width(Length::Fill)
        .style(theme::Container::Box)
        .into()
    }

    fn statistics_series_toggle_row(&self, series: &StatisticsChartSeries) -> Element<'_, Message> {
        let series_key = series.key.clone();
        let checked = self.statistics_visible_series.contains(&series.key);
        let toggle = checkbox(checked)
            .label(series.label.clone())
            .text_size(13)
            .on_toggle(move |_| Message::ToggleStatisticsSeries(series_key.clone()))
            .style(theme::Checkbox::custom(brand_checkbox_style(
                CONTENT_BRAND_SAMPLE,
            )));
        let summary = self.statistics_series_summary(series);
        let dot = container(
            Space::new()
                .width(Length::Fixed(10.0))
                .height(Length::Fixed(10.0)),
        )
        .width(Length::Fixed(10.0))
        .height(Length::Fixed(10.0))
        .style(theme::Container::Custom(statistics_tab_icon_style(
            series.color,
        )));

        let axis_inputs = self.statistics_axis_inputs_by_series.get(&series.key);
        let y_min_input = axis_inputs
            .map(|entry| entry.min_input.as_str())
            .unwrap_or("");
        let y_max_input = axis_inputs
            .map(|entry| entry.max_input.as_str())
            .unwrap_or("");
        let auto_y_bounds = statistics_series_auto_y_bounds(series);
        let auto_y_min = auto_y_bounds
            .map(|bounds| self.statistics_series_value_text(&series.key, bounds.min_value))
            .unwrap_or_else(|| "auto".to_string());
        let auto_y_max = auto_y_bounds
            .map(|bounds| self.statistics_series_value_text(&series.key, bounds.max_value))
            .unwrap_or_else(|| "auto".to_string());
        let invalid_range = auto_y_bounds.is_some()
            && !y_min_input.trim().is_empty()
            && !y_max_input.trim().is_empty()
            && self.statistics_effective_series_y_bounds(series) == auto_y_bounds;
        let invalid_note: Element<'_, Message> = if invalid_range {
            text("Manual range ignored because min is not smaller than max.")
                .size(11)
                .style(theme::Text::Color(recording_active_color()))
                .into()
        } else {
            Space::new()
                .width(Length::Shrink)
                .height(Length::Fixed(0.0))
                .into()
        };
        let bounds_hint = if statistics_series_is_currency_key(&series.key) {
            "Y bounds use EUR values for this series."
        } else {
            "Y bounds use raw counter values for this series."
        };
        let min_key = series.key.clone();
        let max_key = series.key.clone();
        let reset_key = series.key.clone();

        container(
            column![
                row![
                    dot,
                    column![
                        toggle,
                        text(summary)
                            .size(11)
                            .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                    ]
                    .spacing(4),
                ]
                .spacing(10)
                .align_items(Alignment::Center),
                row![
                    self.manual_input("Y Min", &auto_y_min, y_min_input, move |value| {
                        Message::StatisticsAxisMinChanged {
                            series_key: min_key.clone(),
                            value,
                        }
                    }),
                    self.manual_input("Y Max", &auto_y_max, y_max_input, move |value| {
                        Message::StatisticsAxisMaxChanged {
                            series_key: max_key.clone(),
                            value,
                        }
                    }),
                    container(
                        button("Auto")
                            .padding([6, 10])
                            .style(theme::Button::custom(muted_content_button_style()))
                            .on_press(Message::ResetStatisticsAxisBounds(reset_key)),
                    )
                    .align_y(iced::alignment::Vertical::Bottom)
                    .width(Length::Shrink),
                ]
                .spacing(10)
                .align_items(Alignment::End),
                text(bounds_hint)
                    .size(11)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                invalid_note,
            ]
            .spacing(6),
        )
        .padding([8, 10])
        .width(Length::Fill)
        .style(theme::Container::Custom(statistics_chart_track_style()))
        .into()
    }

    fn statistics_line_chart(
        &self,
        visible_series: &[StatisticsChartSeries],
        x_bounds: StatisticsChartBounds,
        series_y_bounds: &HashMap<String, StatisticsSeriesYBounds>,
    ) -> Element<'_, Message> {
        let chart_height = STATISTICS_CHART_SVG_HEIGHT
            + STATISTICS_CHART_CONTAINER_PAD_TOP
            + STATISTICS_CHART_CONTAINER_PAD_BOTTOM;
        let hover = self.statistics_chart_hover;
        let series = visible_series.to_vec();
        let bounds_by_series = series_y_bounds.clone();
        let hover_timestamps = series
            .iter()
            .flat_map(|entry| entry.points.iter().map(|(timestamp, _)| *timestamp))
            .collect::<Vec<_>>();

        iced::widget::responsive(move |size| {
            let svg_markup = statistics_line_chart_svg(&series, x_bounds, &bounds_by_series, hover);
            let chart = iced::widget::svg(iced::widget::svg::Handle::from_memory(
                svg_markup.into_bytes(),
            ))
            .width(Length::Fill)
            .height(Length::Fixed(STATISTICS_CHART_SVG_HEIGHT))
            .style(|_theme, _status| iced::widget::svg::Style { color: None });

            let chart_card = container(chart)
                .padding(iced::Padding {
                    top: STATISTICS_CHART_CONTAINER_PAD_TOP,
                    right: STATISTICS_CHART_CONTAINER_PAD_RIGHT,
                    bottom: STATISTICS_CHART_CONTAINER_PAD_BOTTOM,
                    left: STATISTICS_CHART_CONTAINER_PAD_LEFT,
                })
                .width(Length::Fill)
                .height(Length::Fixed(chart_height))
                .style(theme::Container::Custom(statistics_chart_track_style()));
            let hover_timestamps = hover_timestamps.clone();

            mouse_area(chart_card)
                .on_move(move |point| {
                    statistics_chart_hover_from_cursor(
                        point,
                        size.width,
                        x_bounds,
                        &hover_timestamps,
                    )
                    .map(Message::StatisticsChartHoverMoved)
                    .unwrap_or(Message::StatisticsChartHoverCleared)
                })
                .on_exit(Message::StatisticsChartHoverCleared)
                .into()
        })
        .height(Length::Fixed(chart_height))
        .width(Length::Fill)
        .into()
    }

    fn statistics_chart_empty_state(&self, label: &str) -> Element<'_, Message> {
        container(
            text(label.to_string())
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        )
        .height(Length::Fixed(236.0))
        .width(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(theme::Container::Custom(statistics_chart_track_style()))
        .into()
    }

    fn statistics_axis(&self, bounds: StatisticsChartBounds) -> Element<'_, Message> {
        let first = self.statistics_timestamp_text(bounds.min_timestamp);
        let last = self.statistics_timestamp_text(bounds.max_timestamp);

        row![
            text(first)
                .size(11)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            horizontal_space(),
            text(last)
                .size(11)
                .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
        ]
        .spacing(8)
        .align_items(Alignment::Center)
        .into()
    }

    fn statistics_summary_tile(&self, label: &str, value: String) -> Element<'_, Message> {
        container(
            column![
                text(label.to_string())
                    .size(11)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
                text(value)
                    .size(14)
                    .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12))),
            ]
            .spacing(4),
        )
        .padding([8, 10])
        .width(Length::FillPortion(1))
        .style(theme::Container::Custom(statistics_chart_track_style()))
        .into()
    }

    fn statistics_selection_summary(&self, selected_printers: &[&PrinterRecord]) -> String {
        if selected_printers.is_empty() {
            return "No printers selected.".to_string();
        }

        let labels = selected_printers
            .iter()
            .map(|record| {
                record
                    .model
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(|value| value.to_string())
                    .or_else(|| record.ip_or_hostname.clone())
                    .unwrap_or_else(|| record.id.to_string())
            })
            .collect::<Vec<_>>();
        if labels.len() <= 3 {
            format!("Selected printers: {}", labels.join(", "))
        } else {
            format!(
                "Selected printers: {}, {} and {} more",
                labels[0],
                labels[1],
                labels.len().saturating_sub(2)
            )
        }
    }

    fn statistics_series_summary(&self, series: &StatisticsChartSeries) -> String {
        let Some((captured_at, latest)) = series.points.last().copied() else {
            return "No stored points yet.".to_string();
        };
        let min = series
            .points
            .iter()
            .map(|(_, value)| *value)
            .min()
            .unwrap_or(latest);
        let max = series
            .points
            .iter()
            .map(|(_, value)| *value)
            .max()
            .unwrap_or(latest);

        format!(
            "{} points | latest {} at {} | min {} | max {}",
            series.points.len(),
            self.statistics_series_value_text(&series.key, latest),
            self.statistics_timestamp_text(captured_at),
            self.statistics_series_value_text(&series.key, min),
            self.statistics_series_value_text(&series.key, max),
        )
    }

    fn statistics_range_controls(
        &self,
        range_start: Date,
        range_end: Date,
    ) -> Element<'_, Message> {
        let today = self.statistics_today();
        let preset_rows = column![
            row![
                self.statistics_range_preset_button(StatisticsRangePreset::Day),
                self.statistics_range_preset_button(StatisticsRangePreset::Week),
                self.statistics_range_preset_button(StatisticsRangePreset::Month),
            ]
            .spacing(6),
            row![
                self.statistics_range_preset_button(StatisticsRangePreset::ThreeMonths),
                self.statistics_range_preset_button(StatisticsRangePreset::Year),
                self.statistics_range_preset_button(StatisticsRangePreset::Custom),
            ]
            .spacing(6),
        ]
        .spacing(6);

        let range_note = if self.statistics_range_preset == StatisticsRangePreset::Custom {
            format!(
                "Custom window from {} through {}. End date can stay on today or be set manually.",
                format_calendar_date(range_start),
                format_calendar_date(range_end),
            )
        } else {
            format!(
                "Showing {} ending on today ({}).",
                self.statistics_range_preset,
                format_calendar_date(today),
            )
        };

        let custom_controls: Element<'_, Message> =
            if self.statistics_range_preset == StatisticsRangePreset::Custom {
                iced::widget::responsive(move |size| {
                    let start_picker = || {
                        container(self.statistics_date_picker(
                            "Start date",
                            StatisticsDateTarget::Start,
                            range_start,
                        ))
                        .padding([7, 8])
                        .style(theme::Container::Custom(statistics_date_picker_group_style()))
                        .width(Length::Shrink)
                    };

                    let end_picker = || {
                        container(self.statistics_date_picker(
                            "End date",
                            StatisticsDateTarget::End,
                            range_end,
                        ))
                        .padding([7, 8])
                        .style(theme::Container::Custom(statistics_date_picker_group_style()))
                        .width(Length::Shrink)
                    };

                    let today_button = || {
                        container(
                            button("Today")
                                .padding([7, 12])
                                .style(theme::Button::custom(statistics_date_today_button_style()))
                                .on_press(Message::StatisticsDateSetToday(
                                    StatisticsDateTarget::End,
                                )),
                        )
                        .align_y(iced::alignment::Vertical::Bottom)
                        .width(Length::Shrink)
                    };

                    if size.width >= STATISTICS_DATE_CONTROLS_INLINE_MIN_WIDTH {
                        row![start_picker(), end_picker(), today_button()]
                            .spacing(8)
                            .align_items(Alignment::End)
                            .into()
                    } else {
                        column![start_picker(), end_picker(), today_button()]
                            .spacing(8)
                            .align_items(Alignment::Start)
                            .into()
                    }
                })
                .width(Length::Fill)
                .into()
            } else {
                Space::new()
                    .width(Length::Shrink)
                    .height(Length::Fixed(0.0))
                    .into()
            };

        container(
            column![
                text("Time range")
                    .size(12)
                    .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
                preset_rows,
                custom_controls,
                text(range_note)
                    .size(11)
                    .style(theme::Text::Color(Color::from_rgb8(0x6a, 0x6a, 0x6a))),
            ]
            .spacing(8),
        )
        .padding([8, 10])
        .width(Length::Fill)
        .style(theme::Container::Custom(statistics_chart_track_style()))
        .into()
    }

    fn statistics_range_preset_button(
        &self,
        preset: StatisticsRangePreset,
    ) -> Element<'_, Message> {
        let style: Box<
            dyn Fn(&Theme, iced::widget::button::Status) -> iced::widget::button::Style,
        > = if self.statistics_range_preset == preset {
            Box::new(solid_brand_button_style(CONTENT_BRAND_SAMPLE))
        } else {
            Box::new(theme::Button::custom(muted_content_button_style()))
        };

        button(text(preset.to_string()).size(11))
            .padding([6, 10])
            .width(Length::Fill)
            .style(style)
            .on_press(Message::SelectStatisticsRangePreset(preset))
            .into()
    }

    fn statistics_date_picker(
        &self,
        label: &str,
        target: StatisticsDateTarget,
        date: Date,
    ) -> Element<'_, Message> {
        let today = self.statistics_today();
        let year_options = self.statistics_year_options();
        let day_options = statistics_day_options(date.year(), date.month(), today);
        let year_picker = pick_list(year_options, Some(date.year()), move |year| {
            Message::StatisticsDateYearSelected(target, year)
        })
        .text_size(12)
        .width(Length::Fixed(82.0))
        .style(statistics_date_pick_list_style())
        .menu_style(statistics_date_pick_list_menu_style());
        let month_picker = pick_list(&STATISTICS_MONTHS[..], Some(date.month()), move |month| {
            Message::StatisticsDateMonthSelected(target, month)
        })
        .text_size(12)
        .width(Length::Fixed(108.0))
        .style(statistics_date_pick_list_style())
        .menu_style(statistics_date_pick_list_menu_style());
        let day_picker = pick_list(day_options, Some(date.day()), move |day| {
            Message::StatisticsDateDaySelected(target, day)
        })
        .text_size(12)
        .width(Length::Fixed(58.0))
        .style(statistics_date_pick_list_style())
        .menu_style(statistics_date_pick_list_menu_style());

        column![
            text(label.to_string())
                .size(12)
                .style(theme::Text::Color(Color::from_rgb8(0x3a, 0x4a, 0x5a))),
            row![year_picker, month_picker, day_picker]
                .spacing(5)
                .align_items(Alignment::Center),
        ]
        .spacing(5)
        .width(Length::Shrink)
        .into()
    }

    fn statistics_date_range_summary(&self, range_start: Date, range_end: Date) -> String {
        match self.statistics_range_preset {
            StatisticsRangePreset::Custom => {
                if range_start == range_end {
                    "Custom (1 day)".to_string()
                } else {
                    let days = range_end
                        .to_julian_day()
                        .saturating_sub(range_start.to_julian_day())
                        + 1;
                    format!("Custom ({} days)", days)
                }
            }
            preset => preset.to_string(),
        }
    }

    fn statistics_year_options(&self) -> Vec<i32> {
        let today = self.statistics_today();
        let start_year = self
            .statistics_earliest_date()
            .map(|date| date.year())
            .unwrap_or(today.year())
            .min(today.year());
        (start_year..=today.year()).collect()
    }

    fn statistics_earliest_date(&self) -> Option<Date> {
        self.statistics_store
            .printers
            .iter()
            .flat_map(|entry| entry.poll_samples.iter().map(|sample| sample.captured_at))
            .min()
            .and_then(statistics_local_date)
    }

    fn statistics_timestamp_text(&self, epoch_seconds: u64) -> String {
        let (range_start, range_end) = self.statistics_selected_date_range();
        if range_start == range_end {
            format_clock_hms(epoch_seconds)
        } else {
            format_local_date_time(epoch_seconds)
        }
    }

    fn statistics_series_value_text(&self, series_key: &str, value: u64) -> String {
        if statistics_series_is_currency_key(series_key) {
            format_cents(value)
        } else {
            format_statistics_number(value)
        }
    }

    fn statistics_effective_series_y_bounds(
        &self,
        series: &StatisticsChartSeries,
    ) -> Option<StatisticsSeriesYBounds> {
        let auto_bounds = statistics_series_auto_y_bounds(series)?;
        let mut bounds = auto_bounds;
        let inputs = self.statistics_axis_inputs_by_series.get(&series.key);
        let currency = statistics_series_is_currency_key(&series.key);
        let manual_min =
            inputs.and_then(|entry| parse_statistics_axis_bound(&entry.min_input, currency));
        let manual_max =
            inputs.and_then(|entry| parse_statistics_axis_bound(&entry.max_input, currency));

        if let Some(min_value) = manual_min {
            bounds.min_value = min_value;
        }
        if let Some(max_value) = manual_max {
            bounds.max_value = max_value;
        }

        if bounds.max_value <= bounds.min_value {
            return Some(auto_bounds);
        }

        Some(bounds)
    }

    fn statistics_cleanup_indicator(&self) -> Element<'_, Message> {
        container(
            text("Cleaning statistics history...")
                .size(12)
                .style(theme::Text::Color(Color::WHITE)),
        )
        .padding([6, 10])
        .style(theme::Container::Custom(statistics_indicator_style()))
        .into()
    }

    fn printer_details_view(&self) -> Element<'_, Message> {
        let selected_id = self.selected_printer.as_ref();
        let record = selected_id
            .and_then(|selected| self.printers.iter().find(|record| &record.id == selected));
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
                let mut content = column![
                    text(title)
                        .size(20)
                        .style(theme::Text::Color(Color::from_rgb8(0x12, 0x12, 0x12)))
                ]
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
            PrinterTab::Oids => self.boxed_printer_tab_scroll_view(if let Some(record) = record {
                self.printer_oids_view(record)
            } else if selection_missing {
                self.empty_printer_tab_view("Selected printer not found.")
            } else {
                self.empty_printer_tab_view("Select a printer to edit OIDs.")
            }),
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
                        rows = rows.push(self.poll_varbind_row(&label, &varbind.value.to_string()));
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
        build_poll_label_map(
            &self.counter_oids,
            &self.recording_oids,
            self.active_profile.as_ref(),
        )
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
                    self.value_line("Black", self.toner_value(varbinds, toner.black.as_ref()),),
                    self.value_line("Cyan", self.toner_value(varbinds, toner.cyan.as_ref()),),
                    self.value_line(
                        "Magenta",
                        self.toner_value(varbinds, toner.magenta.as_ref()),
                    ),
                    self.value_line("Yellow", self.toner_value(varbinds, toner.yellow.as_ref()),),
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
                        let label =
                            format!("{} (type {}, {})", entry.label, entry.type_id, entry.unit);
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
        let value_text = value
            .map(|value| value.to_string())
            .unwrap_or_else(|| "N/A".to_string());

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

    fn value_from_oids(&self, varbinds: &[SnmpVarBind], oids: &[Oid]) -> Option<String> {
        oids.iter()
            .find_map(|oid| varbind_display_value(varbinds, oid))
    }

    fn toner_value(&self, varbinds: &[SnmpVarBind], oid: Option<&Oid>) -> Option<String> {
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
            .style(theme::Button::custom(indicator_button_style(
                indicator_color,
            )));

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
            button("Remove non-initial zero entries")
                .style(theme::Button::custom(solid_brand_button_style(
                    SIDEBAR_BRAND_SAMPLE,
                )))
                .on_press(Message::RemoveStatisticsZeroEntries),
            button("Repair duplicate statistics series")
                .style(theme::Button::custom(solid_brand_button_style(
                    SIDEBAR_BRAND_SAMPLE,
                )))
                .on_press(Message::RepairStatisticsDuplicateSeries),
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

#[derive(Debug, Clone)]
struct StatisticsChartSeries {
    key: String,
    label: String,
    color: Color,
    points: Vec<(u64, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StatisticsChartBounds {
    min_timestamp: u64,
    max_timestamp: u64,
    min_value: u64,
    max_value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StatisticsSeriesYBounds {
    min_value: u64,
    max_value: u64,
}

fn statistics_chart_bounds(series: &[StatisticsChartSeries]) -> Option<StatisticsChartBounds> {
    let mut timestamps = series
        .iter()
        .flat_map(|series| series.points.iter().map(|(timestamp, _)| *timestamp));
    let mut values = series
        .iter()
        .flat_map(|series| series.points.iter().map(|(_, value)| *value));
    let min_timestamp = timestamps.next()?;
    let min_value = values.next()?;
    let max_timestamp = series
        .iter()
        .flat_map(|series| series.points.iter().map(|(timestamp, _)| *timestamp))
        .max()
        .unwrap_or(min_timestamp);
    let max_value = series
        .iter()
        .flat_map(|series| series.points.iter().map(|(_, value)| *value))
        .max()
        .unwrap_or(min_value);
    let min_timestamp = series
        .iter()
        .flat_map(|series| series.points.iter().map(|(timestamp, _)| *timestamp))
        .min()
        .unwrap_or(min_timestamp);
    let min_value = series
        .iter()
        .flat_map(|series| series.points.iter().map(|(_, value)| *value))
        .min()
        .unwrap_or(min_value);

    Some(StatisticsChartBounds {
        min_timestamp,
        max_timestamp,
        min_value,
        max_value,
    })
}

fn statistics_series_auto_y_bounds(
    series: &StatisticsChartSeries,
) -> Option<StatisticsSeriesYBounds> {
    let mut values = series.points.iter().map(|(_, value)| *value);
    let first = values.next()?;
    let min_value = series
        .points
        .iter()
        .map(|(_, value)| *value)
        .min()
        .unwrap_or(first);
    let max_value = series
        .points
        .iter()
        .map(|(_, value)| *value)
        .max()
        .unwrap_or(first);

    Some(StatisticsSeriesYBounds {
        min_value,
        max_value,
    })
}

fn statistics_series_color(_series_key: &str, index: usize) -> Color {
    const PALETTE: [Color; 6] = [
        Color::from_rgb(0.32, 0.69, 0.86),
        Color::from_rgb(0.17, 0.55, 0.49),
        Color::from_rgb(0.83, 0.47, 0.19),
        Color::from_rgb(0.33, 0.47, 0.77),
        Color::from_rgb(0.53, 0.63, 0.21),
        Color::from_rgb(0.16, 0.64, 0.78),
    ];

    PALETTE[index % PALETTE.len()]
}

fn statistics_series_is_currency_key(_series_key: &str) -> bool {
    false
}

fn format_statistics_number(value: u64) -> String {
    let digits = value.to_string();
    let mut formatted = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().rev().enumerate() {
        if index != 0 && index % 3 == 0 {
            formatted.push(',');
        }
        formatted.push(digit);
    }
    formatted.chars().rev().collect()
}

fn parse_statistics_axis_bound(value: &str, currency_only: bool) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if currency_only {
        let normalized = trimmed.replace(',', ".");
        let parsed = normalized.parse::<f64>().ok()?;
        if parsed.is_sign_negative() {
            return None;
        }
        Some((parsed * 100.0).round() as u64)
    } else {
        trimmed.replace(',', "").parse::<u64>().ok()
    }
}

fn statistics_chart_hover_from_cursor(
    cursor: iced::Point,
    chart_width: f32,
    x_bounds: StatisticsChartBounds,
    timestamps: &[u64],
) -> Option<StatisticsChartHover> {
    let drawable_width =
        (chart_width - STATISTICS_CHART_CONTAINER_PAD_LEFT - STATISTICS_CHART_CONTAINER_PAD_RIGHT)
            .max(1.0);
    let drawable_height = STATISTICS_CHART_SVG_HEIGHT.max(1.0);
    let local_x = (cursor.x - STATISTICS_CHART_CONTAINER_PAD_LEFT).clamp(0.0, drawable_width);
    let local_y = (cursor.y - STATISTICS_CHART_CONTAINER_PAD_TOP).clamp(0.0, drawable_height);
    let cursor_x = (local_x / drawable_width) * STATISTICS_CHART_SVG_WIDTH;
    let cursor_y = (local_y / drawable_height) * STATISTICS_CHART_SVG_HEIGHT;
    let inferred_timestamp = statistics_timestamp_from_chart_x(x_bounds, cursor_x);
    let timestamp =
        statistics_nearest_timestamp(timestamps, inferred_timestamp).unwrap_or(inferred_timestamp);

    Some(StatisticsChartHover {
        cursor_x,
        cursor_y,
        timestamp,
    })
}

fn statistics_timestamp_from_chart_x(bounds: StatisticsChartBounds, chart_x: f32) -> u64 {
    let plot_left = STATISTICS_CHART_PAD_LEFT;
    let plot_right = STATISTICS_CHART_SVG_WIDTH - STATISTICS_CHART_PAD_RIGHT;
    let clamped_x = chart_x.clamp(plot_left, plot_right);
    let span = bounds.max_timestamp.saturating_sub(bounds.min_timestamp);
    if span == 0 {
        return bounds.min_timestamp;
    }

    let ratio = (clamped_x - plot_left) / (plot_right - plot_left).max(1.0);
    bounds.min_timestamp + (span as f32 * ratio).round() as u64
}

fn statistics_nearest_timestamp(timestamps: &[u64], target: u64) -> Option<u64> {
    timestamps
        .iter()
        .copied()
        .min_by_key(|timestamp| timestamp.abs_diff(target))
}

fn statistics_nearest_series_point(points: &[(u64, u64)], target: u64) -> Option<(u64, u64)> {
    points
        .iter()
        .copied()
        .min_by_key(|(timestamp, _)| timestamp.abs_diff(target))
}

fn statistics_series_tooltip_value_text(series_key: &str, value: u64) -> String {
    if statistics_series_is_currency_key(series_key) {
        format_cents(value)
    } else {
        format_statistics_number(value)
    }
}

fn statistics_escape_svg_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn statistics_line_chart_svg(
    series: &[StatisticsChartSeries],
    x_bounds: StatisticsChartBounds,
    series_y_bounds: &HashMap<String, StatisticsSeriesYBounds>,
    hover: Option<StatisticsChartHover>,
) -> String {
    use std::fmt::Write as _;

    let plot_width =
        STATISTICS_CHART_SVG_WIDTH - STATISTICS_CHART_PAD_LEFT - STATISTICS_CHART_PAD_RIGHT;
    let plot_height =
        STATISTICS_CHART_SVG_HEIGHT - STATISTICS_CHART_PAD_TOP - STATISTICS_CHART_PAD_BOTTOM;
    let mut svg = String::new();
    let _ = write!(
        svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {width} {height}" fill="none">"#,
        width = STATISTICS_CHART_SVG_WIDTH,
        height = STATISTICS_CHART_SVG_HEIGHT,
    );

    for line in 0..=4 {
        let y = STATISTICS_CHART_PAD_TOP + (plot_height / 4.0) * line as f32;
        let _ = write!(
            svg,
            r##"<line x1="{left}" y1="{y:.2}" x2="{x2}" y2="{y:.2}" stroke="#D7DEE6" stroke-width="1"/>"##,
            left = STATISTICS_CHART_PAD_LEFT,
            x2 = STATISTICS_CHART_SVG_WIDTH - STATISTICS_CHART_PAD_RIGHT,
        );
    }

    for series in series {
        let y_bounds = series_y_bounds
            .get(&series.key)
            .copied()
            .or_else(|| statistics_series_auto_y_bounds(series))
            .unwrap_or(StatisticsSeriesYBounds {
                min_value: x_bounds.min_value,
                max_value: x_bounds.max_value,
            });
        let mut polyline = String::new();
        for (index, (timestamp, value)) in series.points.iter().copied().enumerate() {
            let x = statistics_chart_x(x_bounds, timestamp, plot_width, STATISTICS_CHART_PAD_LEFT);
            let y = statistics_chart_y(y_bounds, value, plot_height, STATISTICS_CHART_PAD_TOP);
            if index > 0 {
                polyline.push(' ');
            }
            let _ = write!(polyline, "{x:.2},{y:.2}");
        }

        let color = statistics_color_hex(series.color);
        if series.points.len() >= 2 {
            let _ = write!(
                svg,
                r#"<polyline points="{polyline}" stroke="{color}" stroke-width="3" stroke-linejoin="round" stroke-linecap="round" fill="none"/>"#
            );
        }

        for (timestamp, value) in &series.points {
            let x = statistics_chart_x(x_bounds, *timestamp, plot_width, STATISTICS_CHART_PAD_LEFT);
            let y = statistics_chart_y(y_bounds, *value, plot_height, STATISTICS_CHART_PAD_TOP);
            let _ = write!(
                svg,
                r#"<circle cx="{x:.2}" cy="{y:.2}" r="3.2" fill="{color}" stroke="white" stroke-width="1.5"/>"#
            );
        }
    }

    if let Some(hover) = hover {
        let guide_x = statistics_chart_x(
            x_bounds,
            hover.timestamp,
            plot_width,
            STATISTICS_CHART_PAD_LEFT,
        );
        let guide_top = STATISTICS_CHART_PAD_TOP;
        let guide_bottom = STATISTICS_CHART_SVG_HEIGHT - STATISTICS_CHART_PAD_BOTTOM;
        let _ = write!(
            svg,
            r##"<line x1="{x:.2}" y1="{top:.2}" x2="{x:.2}" y2="{bottom:.2}" stroke="#485560" stroke-width="1" stroke-dasharray="4 3" stroke-opacity="0.7"/>"##,
            x = guide_x,
            top = guide_top,
            bottom = guide_bottom,
        );

        let mut rows: Vec<(String, String, String)> = Vec::new();
        for series in series {
            let y_bounds = series_y_bounds
                .get(&series.key)
                .copied()
                .or_else(|| statistics_series_auto_y_bounds(series))
                .unwrap_or(StatisticsSeriesYBounds {
                    min_value: x_bounds.min_value,
                    max_value: x_bounds.max_value,
                });
            let Some((point_timestamp, point_value)) =
                statistics_nearest_series_point(&series.points, hover.timestamp)
            else {
                continue;
            };
            let point_x = statistics_chart_x(
                x_bounds,
                point_timestamp,
                plot_width,
                STATISTICS_CHART_PAD_LEFT,
            );
            let point_y =
                statistics_chart_y(y_bounds, point_value, plot_height, STATISTICS_CHART_PAD_TOP);
            let color = statistics_color_hex(series.color);
            let _ = write!(
                svg,
                r##"<circle cx="{x:.2}" cy="{y:.2}" r="5.0" fill="{color}" stroke="white" stroke-width="2"/>"##,
                x = point_x,
                y = point_y,
            );
            rows.push((
                series.label.clone(),
                statistics_series_tooltip_value_text(&series.key, point_value),
                color,
            ));
        }

        if !rows.is_empty() {
            let timestamp_label = format_local_date_time(hover.timestamp);
            let mut max_chars = timestamp_label.chars().count();
            for (label, value, _) in &rows {
                max_chars = max_chars.max(label.chars().count() + value.chars().count() + 2);
            }
            let tooltip_width = (max_chars as f32 * 11.5 + 73.0).clamp(308.0, 658.0);
            let tooltip_height = 64.0 + rows.len() as f32 * 31.0;
            let mut tooltip_x = hover.cursor_x + 22.0;
            let mut tooltip_y = hover.cursor_y - tooltip_height - 22.0;
            if tooltip_x + tooltip_width > STATISTICS_CHART_SVG_WIDTH - 4.0 {
                tooltip_x = hover.cursor_x - tooltip_width - 22.0;
            }
            if tooltip_x < 4.0 {
                tooltip_x = 4.0;
            }
            if tooltip_y < 4.0 {
                tooltip_y =
                    (hover.cursor_y + 17.0).min(STATISTICS_CHART_SVG_HEIGHT - tooltip_height - 4.0);
            }
            if tooltip_y < 4.0 {
                tooltip_y = 4.0;
            }

            let _ = write!(
                svg,
                r##"<rect x="{x:.2}" y="{y:.2}" width="{width:.2}" height="{height:.2}" rx="14" fill="#FFFFFF" stroke="#8B99A8" stroke-width="2.0"/>"##,
                x = tooltip_x,
                y = tooltip_y,
                width = tooltip_width,
                height = tooltip_height,
            );
            let _ = write!(
                svg,
                r##"<text x="{x:.2}" y="{y:.2}" fill="#2B3640" font-size="20" font-weight="700" font-family="Segoe UI, Arial, sans-serif">{label}</text>"##,
                x = tooltip_x + 20.0,
                y = tooltip_y + 31.0,
                label = statistics_escape_svg_text(&timestamp_label),
            );

            for (index, (label, value, color)) in rows.iter().enumerate() {
                let row_y = tooltip_y + 64.0 + index as f32 * 31.0;
                let _ = write!(
                    svg,
                    r##"<circle cx="{x:.2}" cy="{y:.2}" r="5.6" fill="{color}"/>"##,
                    x = tooltip_x + 21.0,
                    y = row_y - 6.5,
                    color = color,
                );
                let line = format!("{label}: {value}");
                let _ = write!(
                    svg,
                    r##"<text x="{x:.2}" y="{y:.2}" fill="#33404A" font-size="18" font-family="Segoe UI, Arial, sans-serif">{line}</text>"##,
                    x = tooltip_x + 39.0,
                    y = row_y,
                    line = statistics_escape_svg_text(&line),
                );
            }
        }
    }

    svg.push_str("</svg>");
    svg
}

fn statistics_chart_x(
    bounds: StatisticsChartBounds,
    timestamp: u64,
    plot_width: f32,
    left_padding: f32,
) -> f32 {
    let span = bounds.max_timestamp.saturating_sub(bounds.min_timestamp);
    if span == 0 {
        return left_padding + plot_width * 0.5;
    }

    left_padding
        + ((timestamp.saturating_sub(bounds.min_timestamp)) as f32 / span as f32) * plot_width
}

fn statistics_chart_y(
    bounds: StatisticsSeriesYBounds,
    value: u64,
    plot_height: f32,
    top_padding: f32,
) -> f32 {
    let span = bounds.max_value.saturating_sub(bounds.min_value);
    if span == 0 {
        return top_padding + plot_height * 0.5;
    }

    let normalized = value.saturating_sub(bounds.min_value) as f32 / span as f32;
    top_padding + plot_height - normalized * plot_height
}

fn statistics_color_hex(color: Color) -> String {
    let red = (color.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let green = (color.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let blue = (color.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{red:02X}{green:02X}{blue:02X}")
}
