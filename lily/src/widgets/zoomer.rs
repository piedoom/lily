use std::ops::RangeInclusive;

use femtovg::{Paint, Path};
use glam::Vec2;
use lily_derive::Handle;
use vizia::{
    Actions, Binding, Context, DrawContext, Element, Handle, Lens, LensExt, MouseButton, Units::*,
    View, WindowEvent, ZStack,
};

const HANDLE_SIZE: f32 = 16.0;
const SMALLEST_RANGE: f32 = 0.1;

#[allow(clippy::type_complexity)]
#[derive(Handle)]
pub struct Zoomer<R>
where
    R: Lens<Target = RangeInclusive<f32>>,
{
    range: R,
    status: ZoomerEvent,
    #[callback(f32, f32)]
    on_changing_both: Option<Box<dyn Fn(&mut Context, f32, f32)>>,
    #[callback(f32)]
    on_changing_end: Option<Box<dyn Fn(&mut Context, f32)>>,
    #[callback(f32)]
    on_changing_start: Option<Box<dyn Fn(&mut Context, f32)>>,
}

#[derive(Debug, Clone, Copy)]
pub enum ZoomerEvent {
    SetStart,
    SetEnd,
    SetBoth,
    FinishSet,
}

struct ZoomerControl;

impl ZoomerControl {
    pub fn new(cx: &mut Context) -> Handle<Self> {
        Self {}.build(cx, |_| {})
    }
}

impl View for ZoomerControl {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut vizia::Canvas) {
        let current = cx.current();
        let bounds = cx.cache().get_bounds(current);
        let background_color = cx
            .background_color(cx.current())
            .cloned()
            .unwrap_or_default()
            .into();

        let mut path = Path::new();
        path.rect(bounds.x, bounds.y, bounds.w, bounds.h);

        // Fill with background color
        let paint = Paint::color(background_color);
        // Fill the quad
        canvas.fill_path(&mut path, paint);

        // paint some grabby lines
        let height_offset = bounds.h / 6f32;
        let third = bounds.w / 3f32;
        let x1 = bounds.x + third;
        let line_1 = [
            Vec2::new(x1, bounds.y + height_offset),
            Vec2::new(x1, bounds.y + bounds.h - height_offset),
        ];
        let x2 = bounds.x + (third * 2f32);
        let line_2 = [
            Vec2::new(x2, bounds.y + height_offset),
            Vec2::new(x2, bounds.y + bounds.h - height_offset),
        ];

        let mut path = Path::new();
        path.move_to(line_1[0].x, line_1[0].y);
        path.line_to(line_1[1].x, line_1[1].y);
        path.move_to(line_2[0].x, line_2[0].y);
        path.line_to(line_2[1].x, line_2[1].y);

        // TODO: figure out new way of doing cx.hovered/cx.catured
        // let mut paint = Paint::color(border_color.into());
        // let data = cx.data().unwrap();
        // if cx.hovered == cx.current() || cx.captured == cx.current {
        //     paint = paint.with_line_width(2f32);
        // } else {
        // };

        canvas.stroke_path(&mut path, paint);
    }
}

impl<R> Zoomer<R>
where
    R: Lens<Target = RangeInclusive<f32>>,
{
    pub fn new(cx: &mut Context, range: R) -> Handle<Self> {
        Self {
            on_changing_start: None,
            on_changing_end: None,
            on_changing_both: None,
            status: ZoomerEvent::FinishSet,
            range: range.clone(),
        }
        .build(cx, |cx| {
            let parent_entity = cx.current;

            Binding::new(cx, range.clone(), move |cx, _internal| {
                ZStack::new(cx, |cx| {
                    // Bar
                    Element::new(cx)
                        .height(Stretch(1.0))
                        .left(Pixels(0.0))
                        .right(Stretch(1.0))
                        .class("bar")
                        .bind(range.clone(), |handle, value| {
                            let val = value.get(handle.cx);
                            let width = val.end() - val.start();
                            handle
                                .width(Percentage(width * 100.0))
                                .left(Percentage(val.start() * 100.0))
                                .on_press(|cx| {
                                    cx.emit(ZoomerEvent::SetBoth);
                                });
                        });

                    // Start handle
                    ZoomerControl::new(cx)
                        .class("handle")
                        .height(Stretch(1.0))
                        .bind(range.clone(), move |handle, value| {
                            let val = value.get(handle.cx);
                            handle
                                .left(Percentage(*val.start() * 100.0))
                                .width(Pixels(HANDLE_SIZE))
                                .on_press(move |cx| {
                                    cx.emit(ZoomerEvent::SetStart);
                                });
                        });

                    // End handle
                    let w = cx.cache.get_width(parent_entity);
                    ZoomerControl::new(cx)
                        .class("handle")
                        .height(Stretch(1.0))
                        .bind(range.clone(), move |handle, value| {
                            let val = value.get(handle.cx);

                            handle
                                .left(Stretch(1f32))
                                .right(Pixels((1f32 - *val.end()) * w))
                                .width(Pixels(HANDLE_SIZE))
                                .on_press(move |cx| {
                                    cx.emit(ZoomerEvent::SetEnd);
                                });
                        });
                });
            });
        })
        .width(Stretch(1.0))
        .height(Pixels(24f32))
    }
}

impl<R> View for Zoomer<R>
where
    R: Lens<Target = RangeInclusive<f32>>,
{
    fn element(&self) -> Option<String> {
        Some("zoomer".to_string())
    }

    fn event(&mut self, cx: &mut Context, event: &mut vizia::Event) {
        event.map(|ev: &ZoomerEvent, _| {
            self.status = *ev;
        });
        #[allow(clippy::collapsible_match)]
        event.map(|ev: &WindowEvent, _| match *ev {
            // Respond to cursor movements when we are setting the start or end
            WindowEvent::MouseMove(x, _y) => {
                let width = cx.cache.get_width(cx.current);
                let range = self.range.get(cx);
                // adjust X to be relative
                let mut x = x - cx.cache.get_bounds(cx.current).x;
                // get new data x
                x /= width;
                match self.status {
                    ZoomerEvent::SetStart => {
                        // Set the zoomer amount based on the mouse positioning
                        let x = x.clamp(0f32, *range.end() - SMALLEST_RANGE);
                        if let Some(callback) = &self.on_changing_start {
                            (callback)(cx, x);
                        }
                    }
                    ZoomerEvent::SetEnd => {
                        let x = x.clamp(*range.start() + SMALLEST_RANGE, 1f32);
                        if let Some(callback) = &self.on_changing_end {
                            (callback)(cx, x);
                        }
                    }
                    ZoomerEvent::SetBoth => {
                        // TODO:
                    }
                    _ => (),
                }
            }
            WindowEvent::MouseDown(button) => {
                if button == MouseButton::Left {
                    cx.capture();
                }
            }
            WindowEvent::MouseUp(button) => {
                if button == MouseButton::Left {
                    cx.emit(ZoomerEvent::FinishSet);
                    cx.release();
                }
            }
            _ => (),
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut vizia::Canvas) {
        let current = cx.current();
        let (width, height) = (
            cx.cache().get_width(current),
            cx.cache().get_height(current),
        );
        let background_color: femtovg::Color = cx
            .background_color(cx.current())
            .cloned()
            .unwrap_or_default()
            .into();
        // Draw background rect
        let mut path = Path::new();
        path.rect(0f32, 0f32, width, height);
        canvas.fill_path(&mut path, Paint::color(background_color));
    }
}
