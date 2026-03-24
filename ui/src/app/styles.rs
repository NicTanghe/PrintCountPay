use iced::widget::{button, container, text};
use iced::{Background, Border, Color, Shadow, Theme, Vector};

pub(crate) fn firefox_tab_style(
    active: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
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

        if matches!(status, button::Status::Hovered) && !active
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

        pub(crate) fn Primary(
            theme: &Theme,
            status: button::Status,
        ) -> button::Style {
            button::primary(theme, status)
        }

        pub(crate) fn Secondary(
            theme: &Theme,
            status: button::Status,
        ) -> button::Style {
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
