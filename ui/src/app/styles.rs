use iced::gradient::Linear;
use iced::overlay::menu;
use iced::widget::{button, checkbox, container, pick_list, scrollable, text};
use iced::{Background, Border, Color, Shadow, Theme, Vector, border};

pub(crate) const SIDEBAR_BRAND_SAMPLE: f32 = 1.0 / 6.0;
pub(crate) const CONTENT_BRAND_SAMPLE: f32 = 2.0 / 3.0;
pub(crate) const CONTROLS_BRAND_SAMPLE: f32 = 5.0 / 6.0;

fn brand_gradient_start() -> Color {
    Color::from_rgb8(0x51, 0xb0, 0xdb)
}

fn brand_gradient_end() -> Color {
    Color::from_rgb8(0x55, 0xbf, 0xec)
}

fn right_panel_background_color() -> Color {
    Color::from_rgb8(0xf5, 0xf3, 0xf7)
}

fn right_content_background_color() -> Color {
    Color::from_rgb8(0xf8, 0xf7, 0xf8)
}

fn top_controls_button_color() -> Color {
    Color::from_rgb8(0xd8, 0xda, 0xdf)
}

fn muted_content_button_color() -> Color {
    Color::from_rgb8(0xec, 0xef, 0xf3)
}

fn interpolate_color(start: Color, end: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);

    Color {
        r: start.r + (end.r - start.r) * t,
        g: start.g + (end.g - start.g) * t,
        b: start.b + (end.b - start.b) * t,
        a: start.a + (end.a - start.a) * t,
    }
}

fn shift_color(color: Color, amount: f32) -> Color {
    Color {
        r: (color.r + amount).clamp(0.0, 1.0),
        g: (color.g + amount).clamp(0.0, 1.0),
        b: (color.b + amount).clamp(0.0, 1.0),
        a: color.a,
    }
}

pub(crate) fn recording_active_color() -> Color {
    Color::from_rgb8(0xe0, 0x4f, 0x4f)
}

pub(crate) fn sampled_brand_color(position: f32) -> Color {
    interpolate_color(brand_gradient_start(), brand_gradient_end(), position)
}

pub(crate) fn firefox_tab_style(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let base_background = if active {
            sampled_brand_color(CONTENT_BRAND_SAMPLE)
        } else {
            Color::from_rgb8(0xef, 0xf1, 0xf4)
        };
        let background = match status {
            button::Status::Pressed => {
                shift_color(base_background, if active { -0.04 } else { -0.02 })
            }
            button::Status::Hovered => {
                shift_color(base_background, if active { 0.03 } else { 0.01 })
            }
            button::Status::Disabled => Color {
                a: 0.6,
                ..base_background
            },
            button::Status::Active => base_background,
        };

        button::Style {
            background: Some(Background::Color(background)),
            text_color: if active {
                Color::WHITE
            } else {
                Color::from_rgb8(0x3e, 0x43, 0x4b)
            },
            border: Border {
                color: if active {
                    Color::TRANSPARENT
                } else {
                    Color::from_rgb8(0xd7, 0xdc, 0xe2)
                },
                width: if active { 0.0 } else { 1.0 },
                radius: 7.0.into(),
            },
            shadow: if active {
                Shadow {
                    color: Color::from_rgba8(0x12, 0x45, 0x5d, 0.10),
                    offset: Vector::new(0.0, 2.0),
                    blur_radius: 6.0,
                }
            } else {
                Shadow::default()
            },
            ..button::Style::default()
        }
    }
}

pub(crate) fn indicator_button_style(
    color: Color,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, _status| button::Style {
        background: None,
        text_color: color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

pub(crate) fn solid_brand_button_style(
    position: f32,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    solid_color_button_style(
        sampled_brand_color(position),
        Color::from_rgba8(0x12, 0x45, 0x5d, 0.12),
    )
}

pub(crate) fn solid_recording_button_style() -> impl Fn(&Theme, button::Status) -> button::Style {
    solid_color_button_style(
        recording_active_color(),
        Color::from_rgba8(0x6e, 0x19, 0x19, 0.14),
    )
}

pub(crate) fn top_controls_button_style() -> impl Fn(&Theme, button::Status) -> button::Style {
    subtle_button_style(
        top_controls_button_color(),
        shift_color(top_controls_button_color(), -0.07),
        Color::from_rgb8(0x2a, 0x2f, 0x39),
    )
}

pub(crate) fn muted_content_button_style() -> impl Fn(&Theme, button::Status) -> button::Style {
    subtle_button_style(
        muted_content_button_color(),
        Color::from_rgb8(0xd9, 0xdd, 0xe4),
        Color::from_rgb8(0x5b, 0x63, 0x70),
    )
}

pub(crate) fn manual_icon_button_style() -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, _status| button::Style {
        background: None,
        text_color: Color::from_rgb8(0x2f, 0x36, 0x42),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        shadow: Shadow::default(),
        ..button::Style::default()
    }
}

pub(crate) fn sync_role_indicator_style(accent: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xe8, 0xeb, 0xf0))),
        border: Border {
            color: accent,
            width: 1.0,
            radius: 6.0.into(),
        },
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

pub(crate) fn manual_pricing_header_button_style(
    selected: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let base_color = if selected {
            sampled_brand_color(SIDEBAR_BRAND_SAMPLE)
        } else {
            Color::from_rgb8(0xe5, 0xe9, 0xef)
        };
        let background = match status {
            button::Status::Pressed => {
                shift_color(base_color, if selected { -0.04 } else { -0.02 })
            }
            button::Status::Hovered => shift_color(base_color, if selected { 0.03 } else { 0.01 }),
            button::Status::Disabled => Color {
                a: 0.75,
                ..base_color
            },
            button::Status::Active => base_color,
        };

        button::Style {
            background: Some(Background::Color(background)),
            text_color: if selected {
                Color::WHITE
            } else {
                Color::from_rgb8(0x18, 0x23, 0x33)
            },
            border: Border {
                color: if selected {
                    Color::TRANSPARENT
                } else {
                    Color::from_rgb8(0xd1, 0xd7, 0xe0)
                },
                width: if selected { 0.0 } else { 1.0 },
                radius: 8.0.into(),
            },
            shadow: Shadow {
                color: if selected {
                    Color::from_rgba8(0x2d, 0x4c, 0xb2, 0.12)
                } else {
                    Color::from_rgba8(0x10, 0x19, 0x2b, 0.02)
                },
                offset: Vector::new(0.0, if selected { 3.0 } else { 1.0 }),
                blur_radius: if selected { 8.0 } else { 4.0 },
            },
            ..button::Style::default()
        }
    }
}

pub(crate) fn profile_pick_list_style() -> impl Fn(&Theme, pick_list::Status) -> pick_list::Style {
    move |_theme, status| {
        let accent = sampled_brand_color(CONTENT_BRAND_SAMPLE);
        let background = Background::Color(Color::from_rgb8(0xf1, 0xf4, 0xf8));
        let border_color = match status {
            pick_list::Status::Hovered | pick_list::Status::Opened { .. } => accent,
            pick_list::Status::Active => Color::from_rgb8(0xd4, 0xda, 0xe2),
        };

        pick_list::Style {
            text_color: Color::from_rgb8(0x2f, 0x36, 0x42),
            placeholder_color: Color::from_rgb8(0x6f, 0x78, 0x86),
            handle_color: accent,
            background,
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 7.0.into(),
            },
        }
    }
}

pub(crate) fn profile_pick_list_menu_style() -> impl Fn(&Theme) -> menu::Style {
    move |_theme| {
        let accent = sampled_brand_color(CONTENT_BRAND_SAMPLE);

        menu::Style {
            background: Background::Color(Color::from_rgb8(0xf8, 0xf9, 0xfb)),
            border: Border {
                color: Color::from_rgb8(0xd7, 0xdc, 0xe2),
                width: 1.0,
                radius: 7.0.into(),
            },
            text_color: Color::from_rgb8(0x2f, 0x36, 0x42),
            selected_text_color: Color::WHITE,
            selected_background: Background::Color(accent),
            shadow: Shadow {
                color: Color::from_rgba8(0x10, 0x19, 0x2b, 0.06),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 10.0,
            },
        }
    }
}

pub(crate) fn statistics_date_pick_list_style()
-> impl Fn(&Theme, pick_list::Status) -> pick_list::Style {
    move |_theme, status| {
        let accent = sampled_brand_color(CONTENT_BRAND_SAMPLE);
        let (background, border_color) = match status {
            pick_list::Status::Hovered | pick_list::Status::Opened { .. } => (
                Background::Color(Color::from_rgb8(0xf3, 0xf7, 0xfc)),
                shift_color(accent, -0.02),
            ),
            pick_list::Status::Active => (
                Background::Color(Color::from_rgb8(0xea, 0xef, 0xf5)),
                Color::from_rgb8(0xc8, 0xd2, 0xde),
            ),
        };

        pick_list::Style {
            text_color: Color::from_rgb8(0x2e, 0x3d, 0x4f),
            placeholder_color: Color::from_rgb8(0x6f, 0x7d, 0x8c),
            handle_color: accent,
            background,
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 9.0.into(),
            },
        }
    }
}

pub(crate) fn statistics_date_pick_list_menu_style() -> impl Fn(&Theme) -> menu::Style {
    move |_theme| {
        let accent = sampled_brand_color(CONTENT_BRAND_SAMPLE);
        menu::Style {
            background: Background::Color(Color::from_rgb8(0xf7, 0xfa, 0xfd)),
            border: Border {
                color: Color::from_rgb8(0xc8, 0xd2, 0xde),
                width: 1.0,
                radius: 9.0.into(),
            },
            text_color: Color::from_rgb8(0x2e, 0x3d, 0x4f),
            selected_text_color: Color::WHITE,
            selected_background: Background::Color(accent),
            shadow: Shadow {
                color: Color::from_rgba8(0x10, 0x19, 0x2b, 0.08),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 10.0,
            },
        }
    }
}

pub(crate) fn statistics_date_picker_group_style() -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xe9, 0xee, 0xf4))),
        border: Border {
            color: Color::from_rgb8(0xca, 0xd4, 0xdf),
            width: 1.0,
            radius: 10.0.into(),
        },
        ..container::Style::default()
    }
}

pub(crate) fn statistics_date_today_button_style()
-> impl Fn(&Theme, button::Status) -> button::Style {
    subtle_button_style(
        Color::from_rgb8(0xe8, 0xed, 0xf4),
        Color::from_rgb8(0xc5, 0xcf, 0xdb),
        Color::from_rgb8(0x4e, 0x5d, 0x6d),
    )
}

fn inset_scrollable_style(
    inactive_scroller_color: Color,
) -> impl Fn(&Theme, scrollable::Status) -> scrollable::Style {
    move |theme, status| {
        let mut style = scrollable::default(theme, status);
        let brand = sampled_brand_color(CONTENT_BRAND_SAMPLE);
        let scroller_color = match status {
            scrollable::Status::Hovered {
                is_vertical_scrollbar_hovered: true,
                ..
            }
            | scrollable::Status::Dragged {
                is_vertical_scrollbar_dragged: true,
                ..
            } => brand,
            _ => inactive_scroller_color,
        };

        style.vertical_rail = scrollable::Rail {
            background: None,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 999.0.into(),
            },
            scroller: scrollable::Scroller {
                background: Background::Color(scroller_color),
                border: Border {
                    color: Color::TRANSPARENT,
                    width: 0.0,
                    radius: 999.0.into(),
                },
            },
        };

        style
    }
}

pub(crate) fn printer_list_scrollable_style()
-> impl Fn(&Theme, scrollable::Status) -> scrollable::Style {
    inset_scrollable_style(Color::from_rgb8(0xf5, 0xf3, 0xf7))
}

pub(crate) fn manual_pricing_scrollable_style()
-> impl Fn(&Theme, scrollable::Status) -> scrollable::Style {
    inset_scrollable_style(Color::from_rgb8(0xe2, 0xe2, 0xe9))
}

fn solid_color_button_style(
    base_color: Color,
    shadow_color: Color,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match status {
            button::Status::Pressed => shift_color(base_color, -0.04),
            button::Status::Hovered => shift_color(base_color, 0.03),
            button::Status::Disabled => Color {
                a: 0.55,
                ..base_color
            },
            button::Status::Active => base_color,
        };

        button::Style {
            background: Some(Background::Color(background)),
            text_color: Color::WHITE,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 6.0.into(),
            },
            shadow: Shadow {
                color: shadow_color,
                offset: Vector::new(0.0, 2.0),
                blur_radius: 6.0,
            },
            ..button::Style::default()
        }
    }
}

pub(crate) fn brand_checkbox_style(
    position: f32,
) -> impl Fn(&Theme, checkbox::Status) -> checkbox::Style {
    move |_theme, status| {
        let accent = sampled_brand_color(position);
        let unchecked = Color::from_rgb8(0xff, 0xff, 0xff);

        match status {
            checkbox::Status::Active { is_checked }
            | checkbox::Status::Hovered { is_checked }
            | checkbox::Status::Disabled { is_checked } => checkbox::Style {
                background: Background::Color(if is_checked { accent } else { unchecked }),
                icon_color: if is_checked {
                    Color::WHITE
                } else {
                    Color::TRANSPARENT
                },
                border: Border {
                    color: if is_checked {
                        accent
                    } else {
                        Color::from_rgb8(0xc5, 0xc6, 0xd0)
                    },
                    width: if is_checked { 0.0 } else { 1.0 },
                    radius: 4.0.into(),
                },
                text_color: None,
            },
        }
    }
}

pub(crate) fn window_shell_style() -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(
            Linear::new(2.2)
                .add_stop(0.0, Color::from_rgba8(0xfb, 0xfc, 0xff, 0.99))
                .add_stop(1.0, Color::from_rgba8(0xf5, 0xf7, 0xfb, 0.98))
                .into(),
        ),
        border: Border {
            color: Color::from_rgba8(0xff, 0xff, 0xff, 0.78),
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0x10, 0x19, 0x2b, 0.16),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 28.0,
        },
        ..container::Style::default()
    }
}

fn subtle_button_style(
    base_color: Color,
    border_color: Color,
    text_color: Color,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match status {
            button::Status::Pressed => shift_color(base_color, -0.03),
            button::Status::Hovered => shift_color(base_color, 0.015),
            button::Status::Disabled => Color {
                a: 0.65,
                ..base_color
            },
            button::Status::Active => base_color,
        };

        button::Style {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                color: border_color,
                width: 1.0,
                radius: 6.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba8(0x10, 0x19, 0x2b, 0.03),
                offset: Vector::new(0.0, 1.0),
                blur_radius: 4.0,
            },
            ..button::Style::default()
        }
    }
}

pub(crate) fn right_panel_style() -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(right_panel_background_color())),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: border::right(10.0),
        },
        ..container::Style::default()
    }
}

pub(crate) fn right_content_panel_style() -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(right_content_background_color())),
        border: Border {
            color: Color::from_rgba8(0xd8, 0xd5, 0xdc, 0.9),
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0x10, 0x19, 0x2b, 0.04),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..container::Style::default()
    }
}

pub(crate) fn sidebar_panel_style() -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(
            Linear::new(std::f32::consts::FRAC_PI_2)
                .add_stop(0.0, Color::from_rgb8(0xe8, 0xe8, 0xed))
                .add_stop(1.0, Color::from_rgb8(0xdf, 0xe0, 0xe7))
                .into(),
        ),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: border::left(10.0),
        },
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

pub(crate) fn printer_drop_indicator_style() -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(sampled_brand_color(CONTENT_BRAND_SAMPLE))),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 999.0.into(),
        },
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

pub(crate) fn printer_card_container_style(
    selected: bool,
    base_color: Color,
    dragging: bool,
) -> impl Fn(&Theme) -> container::Style {
    move |_theme| {
        let background = if selected {
            let selected_color = sampled_brand_color(SIDEBAR_BRAND_SAMPLE);
            Background::Color(if dragging {
                shift_color(selected_color, -0.03)
            } else {
                selected_color
            })
        } else {
            let color = if dragging {
                Color {
                    r: (base_color.r + 0.01).min(1.0),
                    g: (base_color.g + 0.01).min(1.0),
                    b: (base_color.b + 0.01).min(1.0),
                    a: 0.99,
                }
            } else {
                Color {
                    a: 0.96,
                    ..base_color
                }
            };
            Background::Color(color)
        };

        container::Style {
            text_color: Some(if selected {
                Color::WHITE
            } else {
                Color::from_rgb8(0x18, 0x23, 0x33)
            }),
            background: Some(background),
            border: Border {
                color: if selected {
                    Color::TRANSPARENT
                } else {
                    Color::from_rgb8(0xc5, 0xc6, 0xd0)
                },
                width: if selected { 0.0 } else { 1.0 },
                radius: 6.0.into(),
            },
            shadow: Shadow {
                color: if selected {
                    Color::from_rgba8(0x2d, 0x4c, 0xb2, 0.14)
                } else {
                    Color::from_rgba8(0x10, 0x19, 0x2b, 0.02)
                },
                offset: Vector::new(0.0, if selected { 4.0 } else { 1.0 }),
                blur_radius: if selected { 10.0 } else { 4.0 },
            },
            ..container::Style::default()
        }
    }
}

pub(crate) fn statistics_indicator_style() -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        text_color: Some(Color::WHITE),
        background: Some(Background::Color(Color::from_rgba8(0x1c, 0x27, 0x39, 0.92))),
        border: Border {
            color: Color::from_rgba8(0xff, 0xff, 0xff, 0.16),
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0x10, 0x19, 0x2b, 0.18),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..container::Style::default()
    }
}

pub(crate) fn statistics_chart_track_style() -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(Color::from_rgb8(0xee, 0xf1, 0xf5))),
        border: Border {
            color: Color::from_rgb8(0xdb, 0xe2, 0xea),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

pub(crate) fn statistics_tab_icon_style(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 3.0.into(),
        },
        ..container::Style::default()
    }
}

pub(crate) fn printer_card_style(
    selected: bool,
    base_color: Color,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = if selected {
            let selected_color = sampled_brand_color(SIDEBAR_BRAND_SAMPLE);
            let color = match status {
                button::Status::Pressed => shift_color(selected_color, -0.04),
                button::Status::Hovered => shift_color(selected_color, 0.03),
                button::Status::Disabled => Color {
                    a: 0.75,
                    ..selected_color
                },
                button::Status::Active => selected_color,
            };

            Background::Color(color)
        } else {
            let color = match status {
                button::Status::Pressed => Color {
                    r: (base_color.r - 0.02).max(0.0),
                    g: (base_color.g - 0.02).max(0.0),
                    b: (base_color.b - 0.02).max(0.0),
                    a: 0.98,
                },
                button::Status::Hovered => Color {
                    r: (base_color.r + 0.01).min(1.0),
                    g: (base_color.g + 0.01).min(1.0),
                    b: (base_color.b + 0.01).min(1.0),
                    a: 0.98,
                },
                _ => Color {
                    a: 0.96,
                    ..base_color
                },
            };
            Background::Color(color)
        };

        button::Style {
            background: Some(background),
            text_color: if selected {
                Color::WHITE
            } else {
                Color::from_rgb8(0x18, 0x23, 0x33)
            },
            border: Border {
                color: if selected {
                    Color::TRANSPARENT
                } else {
                    Color::from_rgb8(0xc5, 0xc6, 0xd0)
                },
                width: if selected { 0.0 } else { 1.0 },
                radius: 6.0.into(),
            },
            shadow: Shadow {
                color: if selected {
                    Color::from_rgba8(0x2d, 0x4c, 0xb2, 0.14)
                } else {
                    Color::from_rgba8(0x10, 0x19, 0x2b, 0.02)
                },
                offset: Vector::new(0.0, if selected { 4.0 } else { 1.0 }),
                blur_radius: if selected { 10.0 } else { 4.0 },
            },
            ..button::Style::default()
        }
    }
}

pub(crate) fn rec_badge_style(active: bool) -> impl Fn(&Theme) -> container::Style {
    move |_theme| {
        let background = if active {
            Some(Background::Color(recording_active_color()))
        } else {
            None
        };
        let text_color = if active {
            Some(Color::from_rgb8(0xff, 0xff, 0xff))
        } else {
            Some(Color::TRANSPARENT)
        };

        container::Style {
            text_color,
            background,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 999.0.into(),
            },
            shadow: Shadow::default(),
            ..container::Style::default()
        }
    }
}

#[allow(non_snake_case)]
pub(crate) mod theme {
    use super::*;

    #[allow(non_snake_case)]
    pub(crate) mod Button {
        use super::*;

        pub(crate) fn Secondary(theme: &Theme, status: button::Status) -> button::Style {
            button::secondary(theme, status)
        }

        pub(crate) fn custom<'a>(
            style: impl Fn(&Theme, button::Status) -> button::Style + 'a,
        ) -> impl Fn(&Theme, button::Status) -> button::Style + 'a {
            style
        }
    }

    #[allow(non_snake_case)]
    pub(crate) mod Checkbox {
        use super::*;

        pub(crate) fn custom<'a>(
            style: impl Fn(&Theme, checkbox::Status) -> checkbox::Style + 'a,
        ) -> impl Fn(&Theme, checkbox::Status) -> checkbox::Style + 'a {
            style
        }
    }

    #[allow(non_snake_case)]
    pub(crate) mod Text {
        use super::*;

        pub(crate) fn Color(color: Color) -> impl Fn(&Theme) -> text::Style {
            move |_theme| text::Style { color: Some(color) }
        }
    }

    #[allow(non_snake_case)]
    pub(crate) mod Container {
        use super::*;

        pub(crate) fn Box(theme: &Theme) -> container::Style {
            container::bordered_box(theme)
        }

        pub(crate) fn Custom<'a>(
            style: impl Fn(&Theme) -> container::Style + 'a,
        ) -> impl Fn(&Theme) -> container::Style + 'a {
            style
        }
    }
}
