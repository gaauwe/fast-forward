mod blink_cursor;

use std::ops::Range;
use blink_cursor::BlinkCursor;
use gpui::{
    actions, div, fill, point, prelude::*, px, relative, size, AppContext, Bounds, CursorStyle, ElementId, ElementInputHandler, FocusHandle, FocusableView, Global, GlobalElementId, KeyBinding, KeyDownEvent, LayoutId, Model, PaintQuad, Pixels, ShapedLine, SharedString, Style, TextRun, UTF16Selection, UnderlineStyle, View, ViewContext, ViewInputHandler, WindowContext
};
use unicode_segmentation::*;

use crate::{applications::{Applications, IndexType}, theme::Theme};

actions!(
    text_input,
    [
        Tab,
        ShiftTab,
        Backspace,
        Left,
        Right,
    ]
);

pub struct SearchQuery {
    pub value: String,
}

impl Global for SearchQuery {}

pub struct TextInput {
    focus_handle: FocusHandle,
    pub value: SharedString,
    placeholder: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
    blink_cursor: Model<BlinkCursor>,
}

// Mostly copied from the TextInput example in the gpui repository, with some modifications.
// - https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/input.rs
impl TextInput {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let blink_cursor = cx.new_model(|_| BlinkCursor::new());

        let input = Self {
            value: "".into(),
            placeholder: "Switch to...".into(),
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            focus_handle,
            blink_cursor,
        };

        // Observe the blink cursor to repaint the view when it changes.
        cx.observe(&input.blink_cursor, |_, _, cx| cx.notify())
            .detach();
        // Blink the cursor when the window is active, pause when it's not.
        cx.observe_window_activation(|input, cx| {
            if cx.is_window_active() {
                // TODO: This is a hack to clear the search query when the window is re-activated.
                input.value = "".into();
                input.selected_range = 0..0;

                let focus_handle = input.focus_handle.clone();
                if focus_handle.is_focused(cx) {
                    input.blink_cursor.update(cx, |blink_cursor, cx| {
                        blink_cursor.start(cx);
                    });
                }
            } else {
                input.blink_cursor.update(cx, |blink_cursor, cx| {
                    blink_cursor.stop(cx);
                });
            }
        })
        .detach();

        // Attach key listeners.
        cx.bind_keys([
            KeyBinding::new("tab", Tab, None),
            // TODO: Shift+Tab isn't recognized yet.
            KeyBinding::new("shift-tab", Tab, None),
            KeyBinding::new("backspace", Backspace, None),
            KeyBinding::new("left", Left, None),
            KeyBinding::new("right", Right, None),
        ]);

        cx.set_global(SearchQuery {
            value: String::new()
        });

        input
    }

    fn left(&mut self, _: &Left, cx: &mut ViewContext<Self>) {
        self.pause_blink_cursor(cx);
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    fn right(&mut self, _: &Right, cx: &mut ViewContext<Self>) {
        self.pause_blink_cursor(cx);
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    fn tab(&mut self, _: &Tab, cx: &mut ViewContext<Self>) {
        Applications::update_active_index(cx, IndexType::Next);
        cx.notify();
    }

    fn shift_tab(&mut self, _: &ShiftTab, cx: &mut ViewContext<Self>) {
        Applications::update_active_index(cx, IndexType::Previous);
        cx.notify();
    }

    fn backspace(&mut self, _: &Backspace, cx: &mut ViewContext<Self>) {
        self.pause_blink_cursor(cx);
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", cx)
    }

    fn move_to(&mut self, offset: usize, cx: &mut ViewContext<Self>) {
        self.pause_blink_cursor(cx);
        self.selected_range = offset..offset;
        cx.notify()
    }

    fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn pause_blink_cursor(&mut self, cx: &mut ViewContext<Self>) {
        self.blink_cursor.update(cx, |cursor, cx| {
            cursor.pause(cx);
        });
    }

    fn on_key_down_for_blink_cursor(&mut self, _: &KeyDownEvent, cx: &mut ViewContext<Self>) {
        self.pause_blink_cursor(cx)
    }

    pub(crate) fn show_cursor(&self, cx: &WindowContext) -> bool {
        self.focus_handle.is_focused(cx) && self.blink_cursor.read(cx).visible()
    }

    fn select_to(&mut self, offset: usize, cx: &mut ViewContext<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify()
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.value.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.value.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.value
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.value
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.value.len())
    }
}

impl ViewInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _cx: &mut ViewContext<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.value[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _cx: &mut ViewContext<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(&self, _cx: &mut ViewContext<Self>) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _cx: &mut ViewContext<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        cx: &mut ViewContext<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.value =
            (self.value[0..range.start].to_owned() + new_text + &self.value[range.end..])
                .into();
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range.take();

        cx.set_global(SearchQuery {
            value: self.value.clone().into(),
        });

        // Reset the active index after filtering.
        Applications::update_active_index(cx, IndexType::Start);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        cx: &mut ViewContext<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.value =
            (self.value[0..range.start].to_owned() + new_text + &self.value[range.end..])
                .into();
        self.marked_range = Some(range.start..range.start + new_text.len());
        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());

        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _cx: &mut ViewContext<Self>,
    ) -> Option<Bounds<Pixels>> {
        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(range.start),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(range.end),
                bounds.bottom(),
            ),
        ))
    }
}

impl Global for TextInput {}

struct TextElement {
    input: View<TextInput>,
}

struct PrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();

    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        cx: &mut WindowContext,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = cx.line_height().into();
        (cx.request_layout(style, []), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        cx: &mut WindowContext,
    ) -> Self::PrepaintState {
        let theme = cx.global::<Theme>();
        let input = self.input.read(cx);
        let value = input.value.clone();
        let selected_range = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let style = cx.text_style();

        let (display_text, text_color) = if value.is_empty() {
            (input.placeholder.clone(), theme.muted_foreground)
        } else {
            (value.clone(), style.color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked_range) = input.marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run.clone()
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(cx.rem_size());
        let line = cx
            .text_system()
            .shape_line(display_text, font_size, &runs)
            .unwrap();

        let cursor_pos = line.x_for_index(cursor);
        let cursor = if selected_range.is_empty() && input.show_cursor(cx) {
            Some(fill(
                Bounds::new(
                    point(bounds.left() + cursor_pos, bounds.top() + Pixels(1.5)),
                    size(px(2.), bounds.bottom() - bounds.top() - Pixels(6.)),
                ),
                theme.primary,
            ))
        } else {
            None
        };
        PrepaintState {
            line: Some(line),
            cursor,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        cx: &mut WindowContext,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        cx.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
        );
        let line = prepaint.line.take().unwrap();
        line.paint(bounds.origin, cx.line_height(), cx).unwrap();

        if focus_handle.is_focused(cx) {
            if let Some(cursor) = prepaint.cursor.take() {
                cx.paint_quad(cursor);
            }
        }

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

impl Render for TextInput {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();

        div()
            .flex()
            .key_context("TextInput")
            .track_focus(&self.focus_handle(cx))
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::tab))
            .on_action(cx.listener(Self::shift_tab))
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_key_down(cx.listener(Self::on_key_down_for_blink_cursor))
            .border_b_1()
            .border_color(theme.border)
            .child(
                div()
                    .w_full()
                    .p(px(4.))
                    .px(px(6.))
                    .text_color(theme.foreground)
                    .child(TextElement {
                        input: cx.view().clone(),
                    }),
            )
    }
}

impl FocusableView for TextInput {
    fn focus_handle(&self, _: &AppContext) -> FocusHandle {
        self.focus_handle.clone()
    }
}
