use gpui::*;

use crate::theme::Theme;

#[derive(IntoElement)]
pub(crate) struct Icon {
    name: IconName,
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
}

impl Icon {
    pub fn new(name: IconName) -> Self {
        Self {
            name,
            transformation: Transformation::default(),
        }
    }

    pub fn transform(mut self, transformation: Transformation) -> Self {
        self.transformation = transformation;
        self
    }

    pub fn path(&self) -> String {
        match self.name {
            IconName::ArrowCircle => "arrow_circle.svg".to_string(),
        }
    }
}

impl RenderOnce for Icon {
    fn render(self, cx: &mut WindowContext) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let path = self.path();

        svg()
            .path(path)
            .size_5()
            .text_color(theme.foreground)
            .with_transformation(self.transformation)
            .into_element()
    }
}
