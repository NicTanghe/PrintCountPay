use iced::advanced::layout::{self, Layout};
use iced::advanced::mouse;
use iced::advanced::overlay;
use iced::advanced::renderer;
use iced::advanced::widget::{self, Widget};
use iced::advanced::{Clipboard, Shell};
use iced::event::Event;
use iced::{Element, Length, Pixels, Point, Rectangle, Size, Vector};

#[allow(missing_debug_implementations)]
pub(crate) struct BadgeOverlay<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    content: Element<'a, Message, Theme, Renderer>,
    badge: Element<'a, Message, Theme, Renderer>,
    show_badge: bool,
    margin: f32,
}

impl<'a, Message, Theme, Renderer> BadgeOverlay<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    const DEFAULT_MARGIN: f32 = 6.0;

    pub(crate) fn new(
        content: impl Into<Element<'a, Message, Theme, Renderer>>,
        badge: impl Into<Element<'a, Message, Theme, Renderer>>,
        show_badge: bool,
    ) -> Self {
        Self {
            content: content.into(),
            badge: badge.into(),
            show_badge,
            margin: Self::DEFAULT_MARGIN,
        }
    }

    pub(crate) fn margin(mut self, margin: impl Into<Pixels>) -> Self {
        self.margin = margin.into().0;
        self
    }
}

impl<'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for BadgeOverlay<'a, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn children(&self) -> Vec<widget::Tree> {
        vec![
            widget::Tree::new(&self.content),
            widget::Tree::new(&self.badge),
        ]
    }

    fn diff(&self, tree: &mut widget::Tree) {
        tree.diff_children(&[
            self.content.as_widget(),
            self.badge.as_widget(),
        ]);
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut widget::Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        inherited_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            inherited_style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut widget::Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        let mut children = tree.children.iter_mut();

        let content = self.content.as_widget_mut().overlay(
            children.next().unwrap(),
            layout,
            renderer,
            viewport,
            translation,
        );

        let badge = if self.show_badge {
            Some(overlay::Element::new(Box::new(BadgeOverlayLayer {
                position: layout.position() + translation,
                content_bounds: layout.bounds(),
                badge: &mut self.badge,
                state: children.next().unwrap(),
                margin: self.margin,
            })))
        } else {
            None
        };

        if content.is_some() || badge.is_some() {
            Some(
                overlay::Group::with_children(
                    content.into_iter().chain(badge).collect(),
                )
                .overlay(),
            )
        } else {
            None
        }
    }
}

impl<'a, Message, Theme, Renderer> From<BadgeOverlay<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: iced::advanced::Renderer + 'a,
{
    fn from(
        overlay: BadgeOverlay<'a, Message, Theme, Renderer>,
    ) -> Element<'a, Message, Theme, Renderer> {
        Element::new(overlay)
    }
}

struct BadgeOverlayLayer<'a, 'b, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    position: Point,
    content_bounds: Rectangle,
    badge: &'b mut Element<'a, Message, Theme, Renderer>,
    state: &'b mut widget::Tree,
    margin: f32,
}

impl<'a, 'b, Message, Theme, Renderer>
    overlay::Overlay<Message, Theme, Renderer>
    for BadgeOverlayLayer<'a, 'b, Message, Theme, Renderer>
where
    Renderer: iced::advanced::Renderer,
{
    fn layout(&mut self, renderer: &Renderer, bounds: Size) -> layout::Node {
        let badge_layout = self.badge.as_widget_mut().layout(
            self.state,
            renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );

        let badge_bounds = badge_layout.bounds();
        let mut x = self.position.x
            + (self.content_bounds.width - badge_bounds.width)
            - self.margin;
        let mut y = self.position.y
            + (self.content_bounds.height - badge_bounds.height)
            - self.margin;

        if x < self.position.x {
            x = self.position.x;
        }
        if y < self.position.y {
            y = self.position.y;
        }

        layout::Node::with_children(
            badge_bounds.size(),
            vec![badge_layout],
        )
        .translate(Vector::new(x, y))
    }

    fn draw(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        inherited_style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
    ) {
        self.badge.as_widget().draw(
            self.state,
            renderer,
            theme,
            inherited_style,
            layout.children().next().unwrap(),
            cursor,
            &Rectangle::with_size(Size::INFINITE),
        );
    }

}
