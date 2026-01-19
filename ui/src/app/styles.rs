use iced::{Background, Border, Color, Theme, Vector};

#[derive(Debug, Clone, Copy)]
pub(crate) struct FirefoxTabStyle {
    pub(crate) active: bool,
}

impl iced::widget::button::StyleSheet for FirefoxTabStyle {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let palette = style.extended_palette();
        let background = if self.active {
            palette.background.base.color
        } else {
            palette.background.weak.color
        };
        let text_color = if self.active {
            palette.background.base.text
        } else {
            palette.background.weak.text
        };

        iced::widget::button::Appearance {
            background: Some(Background::Color(background)),
            text_color,
            border: Border {
                color: palette.background.strong.color,
                width: 1.0,
                radius: [8.0, 8.0, 0.0, 0.0].into(),
            },
            shadow_offset: if self.active {
                Vector::new(0.0, 0.0)
            } else {
                Vector::new(0.0, 1.0)
            },
            ..iced::widget::button::Appearance::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> iced::widget::button::Appearance {
        let mut appearance = self.active(style);
        if !self.active {
            if let Some(Background::Color(color)) = appearance.background {
                let lifted = Color {
                    r: (color.r + 0.05).min(1.0),
                    g: (color.g + 0.05).min(1.0),
                    b: (color.b + 0.05).min(1.0),
                    a: color.a,
                };
                appearance.background = Some(Background::Color(lifted));
            }
        }
        appearance
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct IndicatorButtonStyle {
    pub(crate) color: Color,
}

impl iced::widget::button::StyleSheet for IndicatorButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> iced::widget::button::Appearance {
        iced::widget::button::Appearance {
            background: None,
            text_color: self.color,
            border: Border {
                color: Color::from_rgb8(0x00, 0x00, 0x00),
                width: 0.0,
                radius: 0.0.into(),
            },
            shadow_offset: Vector::new(0.0, 0.0),
            ..iced::widget::button::Appearance::default()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RecBadgeStyle {
    pub(crate) active: bool,
}

impl iced::widget::container::StyleSheet for RecBadgeStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> iced::widget::container::Appearance {
        let background = if self.active {
            Some(Background::Color(Color::from_rgb8(0xe0, 0x4f, 0x4f)))
        } else {
            None
        };
        let text_color = if self.active {
            Some(Color::from_rgb8(0xff, 0xff, 0xff))
        } else {
            Some(Color::TRANSPARENT)
        };

        iced::widget::container::Appearance {
            text_color,
            background,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 999.0.into(),
            },
            ..iced::widget::container::Appearance::default()
        }
    }
}
