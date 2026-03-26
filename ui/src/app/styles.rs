use iced::gradient::Linear;
use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Shadow, Theme, Vector, border};

pub(crate) fn firefox_tab_style(active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let palette = theme.extended_palette();
        let background = if active {
            palette.background.base.color
        } else {
            palette.background.weak.color
        };
        let text_color = if active {
            palette.background.base.text
        } else {
            palette.background.weak.text
        };

        let mut style = button::Style {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                color: palette.background.strong.color,
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: if active {
                Shadow::default()
            } else {
                Shadow {
                    offset: Vector::new(0.0, 1.0),
                    ..Shadow::default()
                }
            },
            ..button::Style::default()
        };

        if matches!(status, button::Status::Hovered)
            && !active
            && let Some(Background::Color(color)) = style.background
        {
            style.background = Some(Background::Color(Color {
                r: (color.r + 0.05).min(1.0),
                g: (color.g + 0.05).min(1.0),
                b: (color.b + 0.05).min(1.0),
                a: color.a,
            }));
        }

        style
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

pub(crate) fn sidebar_panel_style() -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(
            Linear::new(2.25)
                .add_stop(0.0, Color::from_rgba8(0xf5, 0xf6, 0xfa, 0.96))
                .add_stop(1.0, Color::from_rgba8(0xef, 0xf1, 0xf6, 0.94))
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

pub(crate) fn printer_card_style(
    selected: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = if selected {
            match status {
                button::Status::Pressed => Linear::new(1.15)
                    .add_stop(0.0, Color::from_rgb8(0x4a, 0x68, 0xe0))
                    .add_stop(1.0, Color::from_rgb8(0x3f, 0x5f, 0xda))
                    .into(),
                button::Status::Hovered => Linear::new(1.15)
                    .add_stop(0.0, Color::from_rgb8(0x6c, 0x89, 0xff))
                    .add_stop(1.0, Color::from_rgb8(0x5b, 0x7b, 0xf5))
                    .into(),
                _ => Linear::new(1.15)
                    .add_stop(0.0, Color::from_rgb8(0x66, 0x84, 0xff))
                    .add_stop(1.0, Color::from_rgb8(0x55, 0x75, 0xf0))
                    .into(),
            }
        } else {
            let color = match status {
                button::Status::Pressed => Color::from_rgba8(0xec, 0xf0, 0xf8, 0.98),
                button::Status::Hovered => Color::from_rgba8(0xf8, 0xfa, 0xfe, 0.98),
                _ => Color::from_rgba8(0xff, 0xff, 0xff, 0.94),
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
                    Color::from_rgba8(0x5f, 0x7d, 0xf1, 0.95)
                } else {
                    Color::from_rgba8(0xd4, 0xdc, 0xea, 0.9)
                },
                width: 1.0,
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
            Some(Background::Color(Color::from_rgb8(0xe0, 0x4f, 0x4f)))
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

        pub(crate) fn Primary(theme: &Theme, status: button::Status) -> button::Style {
            button::primary(theme, status)
        }

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
