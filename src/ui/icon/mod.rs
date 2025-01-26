use gpui::*;

use crate::theme::Theme;

#[derive(IntoElement)]
pub(crate) struct Icon {
    name: IconName,
    size: IconSize,
    color: IconColor,
    transformation: Transformation,
}

#[derive(
    Debug,
    PartialEq,
    Eq,
    Copy,
    Clone,
)]
pub enum IconName {
    ArrowCircle,
    ExternalLink
}

pub enum IconSize {
    Small,
    Default,
}

pub enum IconColor {
    Default,
    Muted,
}

impl Icon {
    pub fn new(name: IconName, size: IconSize, color: IconColor) -> Self {
        Self {
            name,
            size,
            color,
            transformation: Transformation::default(),
        }
    }

    pub fn transform(mut self, transformation: Transformation) -> Self {
        self.transformation = transformation;
        self
    }

    pub fn path(&self) -> String {
        match self.name {
            IconName::ArrowCircle => "icons/arrow_circle.svg".to_string(),
            IconName::ExternalLink => "icons/external_link.svg".to_string(),
        }
    }
}

impl RenderOnce for Icon {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let path = self.path();

        let color = match self.color {
            IconColor::Default => theme.foreground,
            IconColor::Muted => theme.muted_foreground,
        };

        let size = match self.size {
            IconSize::Small => 16.0,
            IconSize::Default => 20.0,
        };

        svg()
            .path(path)
            .size(px(size))
            .text_color(color)
            .with_transformation(self.transformation)
            .into_element()
    }
}
