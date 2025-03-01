use async_channel::Sender;
use gpui::*;

use crate::{input::*, State};

// Controls has our text input and submit button
pub struct Controls {
    outgoing_tx: Sender<String>,
    text_input: Entity<TextInput>,
}

impl Controls {
    fn new(outgoing_tx: Sender<String>, cx: &mut App) -> Entity<Self> {
        let text_input = cx.new(|cx| TextInput {
            focus_handle: cx.focus_handle(),
            content: "".into(),
            placeholder: "Message...".into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
        });

        cx.new(|_| Self {
            outgoing_tx,
            text_input,
        })
    }

    // Submit puts the text input's content onto the outgoing_tx channel
    fn submit(&mut self, _: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let text = self.text_input.read(cx).content.clone();
        if !text.is_empty() {
            let result = self.outgoing_tx.try_send(text.to_string());
            match result {
                Ok(_) => println!("Message sent successfully"),
                Err(e) => println!("Failed to send message: {}", e),
            }
        }

        self.text_input
            .update(cx, |text_input, _cx| text_input.reset());
        cx.notify();
    }
}

impl Render for Controls {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let input = div()
            .flex()
            .flex_grow()
            .p_1()
            .rounded_md()
            .border_1()
            .border_color(black().alpha(0.1))
            .child(self.text_input.clone());

        let button = div()
            .flex()
            .justify_center()
            .items_center()
            .px_2()
            .min_w(px(50.0))
            .rounded_md()
            .cursor_pointer()
            .hover(|x| x.bg(rgb(0xeeeeee)))
            .border_1()
            .border_color(black().alpha(0.1))
            .child("Send")
            .on_mouse_down(MouseButton::Left, cx.listener(Self::submit));

        div().flex().gap_1().size_full().child(input).child(button)
    }
}

// Set focus to the text input when the controls are focused
impl Focusable for Controls {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.text_input.read(cx).focus_handle(cx).clone()
    }
}

pub struct MessageList {
    state: ListState,
}

impl Render for MessageList {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .child(list(self.state.clone()).w_full().h_full())
    }
}

impl MessageList {
    pub fn new(entity: Entity<State>, app: &mut App) -> Entity<Self> {
        app.new(|cx| {
            cx.observe(&entity, |this: &mut MessageList, entity, cx| {
                let items = entity.read(cx).messages.clone();
                this.state = ListState::new(
                    items.len(),
                    ListAlignment::Bottom,
                    Pixels(20.),
                    move |idx, _win, _app| {
                        let item = items.get(idx).unwrap().clone();
                        let message_bubble = MessageBubble::new(item);
                        div().child(message_bubble).into_any_element()
                    },
                );
                cx.notify();
            })
            .detach();

            MessageList {
                state: ListState::new(0, ListAlignment::Bottom, Pixels(20.), move |_, _, _| {
                    div().into_any_element()
                }),
            }
        })
    }
}

#[derive(Debug, Clone, IntoElement)]
pub struct MessageBubble {
    message: String,
}

impl MessageBubble {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl RenderOnce for MessageBubble {
    fn render(self, _win: &mut Window, _app: &mut App) -> impl IntoElement {
        div()
            .flex()
            .justify_between()
            .items_center()
            .py_2()
            .px_4()
            .border_1()
            .rounded_xl()
            .bg(blue().alpha(0.1))
            .m_1()
            .border_color(blue().alpha(0.25))
            .text_lg()
            .child(self.message.clone())
    }
}

// Workspace is the main layout that holds the message list and controls
pub struct Workspace {
    pub list_view: Entity<MessageList>,
    pub controls_view: Entity<Controls>,
}

impl Render for Workspace {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let list = div()
            .flex()
            .flex_grow()
            .justify_center()
            .items_center()
            .child(self.list_view.clone());

        let controls = div()
            .flex()
            .flex_col()
            .border_t_1()
            .p_2()
            .child(div().flex().child(self.controls_view.clone()));

        div()
            .border_1()
            .bg(white())
            .flex()
            .flex_col()
            .size_full()
            .child(list)
            .child(controls)
    }
}

impl Workspace {
    pub fn build(
        app: &mut App,
        state_entity: Entity<State>,
        outgoing_tx: Sender<String>,
    ) -> Entity<Self> {
        let list_view = MessageList::new(state_entity, app);
        let controls_view = Controls::new(outgoing_tx, app);
        app.new(|_cx| Self {
            list_view,
            controls_view,
        })
    }
}
