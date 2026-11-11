use super::*;
use std::time::Duration;

pub(super) struct WidthTransitionFixture {
    width: Option<f64>,
    duration: Option<Duration>,
    visible: bool,
}

#[derive(Clone)]
pub(super) enum Message {
    Width(f64),
    ClearTransition,
    ZeroDuration,
    ClearWidth,
    Hide,
    Show,
}

impl Component for WidthTransitionFixture {
    type Input = ();
    type Message = Message;

    fn create(_: &(), _: &ComponentContext<Self>) -> Self {
        Self {
            width: Some(60.0),
            duration: Some(Duration::from_secs(1)),
            visible: true,
        }
    }

    fn update(&mut self, message: Message, _: &ComponentContext<Self>) {
        match message {
            Message::Width(width) => self.width = Some(width),
            Message::ClearTransition => self.duration = None,
            Message::ZeroDuration => self.duration = Some(Duration::ZERO),
            Message::ClearWidth => self.width = None,
            Message::Hide => self.visible = false,
            Message::Show => {
                self.visible = true;
                self.width = Some(60.0);
                self.duration = Some(Duration::from_secs(1));
            }
        }
    }

    fn view(&self, _: &(), context: &mut ViewContext<Self>) -> View {
        context.window_visuals(WindowVisuals::new().client_size(500.0, 600.0));
        let mut children = vec![keyed(
            "reference",
            Border::new()
                .automation_id("reference")
                .automation_name("Reference")
                .width(60.0)
                .height(8.0)
                .horizontal_alignment(HorizontalAlignment::Left)
                .background(ThemeBrush::Accent),
        )];
        for (id, message) in [
            ("expand", Message::Width(240.0)),
            ("contract", Message::Width(60.0)),
            ("clear-transition", Message::ClearTransition),
            ("zero-duration", Message::ZeroDuration),
            ("clear-width", Message::ClearWidth),
            ("hide", Message::Hide),
            ("show", Message::Show),
        ] {
            children.push(keyed(
                id,
                Button::new()
                    .automation_id(id)
                    .content(id)
                    .on_click(context.message(message)),
            ));
        }
        if self.visible {
            let mut indicator = Border::new()
                .automation_id("indicator")
                .automation_name("Indicator")
                .height(8.0)
                .content(Border::new().width(20.0).height(8.0))
                .horizontal_alignment(HorizontalAlignment::Left)
                .background(ThemeBrush::Accent);
            if let Some(width) = self.width {
                indicator = indicator.width(width);
            }
            if let Some(duration) = self.duration {
                indicator = indicator.width_transition(duration);
            }
            children.push(keyed("indicator", indicator));
        }
        context.window_frame(
            "Reactor UI fixture: width-transition",
            StackPanel::new()
                .margin(20.0)
                .spacing(8.0)
                .keyed_children(children),
        )
    }
}
