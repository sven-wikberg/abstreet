//! Generic UI tools. Some of this should perhaps be lifted to widgetry.

use anyhow::Result;

use geom::Polygon;
use widgetry::{
    hotkeys, Choice, Color, DrawBaselayer, EventCtx, GfxCtx, Key, Line, Menu, Outcome, Panel,
    State, Text, TextBox, Transition, Widget,
};

use crate::load::FutureLoader;
use crate::tools::grey_out_map;
use crate::AppLike;

/// Choose something from a menu, then feed the answer to a callback.
pub struct ChooseSomething<A: AppLike, T> {
    panel: Panel,
    // Wrapped in an Option so that we can consume it once
    cb: Option<Box<dyn FnOnce(T, &mut EventCtx, &mut A) -> Transition<A>>>,
}

impl<A: AppLike + 'static, T: 'static> ChooseSomething<A, T> {
    pub fn new_state<I: Into<String>>(
        ctx: &mut EventCtx,
        query: I,
        choices: Vec<Choice<T>>,
        cb: Box<dyn FnOnce(T, &mut EventCtx, &mut A) -> Transition<A>>,
    ) -> Box<dyn State<A>> {
        Box::new(ChooseSomething {
            panel: Panel::new_builder(Widget::col(vec![
                Widget::row(vec![
                    Line(query).small_heading().into_widget(ctx),
                    ctx.style().btn_close_widget(ctx),
                ]),
                Menu::widget(ctx, choices).named("menu"),
            ]))
            .build(ctx),
            cb: Some(cb),
        })
    }
}

impl<A: AppLike + 'static, T: 'static> State<A> for ChooseSomething<A, T> {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut A) -> Transition<A> {
        match self.panel.event(ctx) {
            Outcome::Clicked(x) => match x.as_ref() {
                "close" => Transition::Pop,
                _ => {
                    let data = self.panel.take_menu_choice::<T>("menu");
                    // If the callback doesn't replace or pop this ChooseSomething state, then
                    // it'll break when the user tries to interact with the menu again.
                    (self.cb.take().unwrap())(data, ctx, app)
                }
            },
            _ => {
                if ctx.normal_left_click() && ctx.canvas.get_cursor_in_screen_space().is_none() {
                    return Transition::Pop;
                }
                Transition::Keep
            }
        }
    }

    fn draw_baselayer(&self) -> DrawBaselayer {
        DrawBaselayer::PreviousState
    }

    fn draw(&self, g: &mut GfxCtx, app: &A) {
        grey_out_map(g, app);
        self.panel.draw(g);
    }
}

/// Prompt for arbitrary text input, then feed the answer to a callback.
pub struct PromptInput<A: AppLike> {
    panel: Panel,
    cb: Option<Box<dyn FnOnce(String, &mut EventCtx, &mut A) -> Transition<A>>>,
}

impl<A: AppLike + 'static> PromptInput<A> {
    pub fn new_state(
        ctx: &mut EventCtx,
        query: &str,
        initial: String,
        cb: Box<dyn FnOnce(String, &mut EventCtx, &mut A) -> Transition<A>>,
    ) -> Box<dyn State<A>> {
        Box::new(PromptInput {
            panel: Panel::new_builder(Widget::col(vec![
                Widget::row(vec![
                    Line(query).small_heading().into_widget(ctx),
                    ctx.style().btn_close_widget(ctx),
                ]),
                TextBox::default_widget(ctx, "input", initial),
                ctx.style()
                    .btn_outline
                    .text("confirm")
                    .hotkey(Key::Enter)
                    .build_def(ctx),
            ]))
            .build(ctx),
            cb: Some(cb),
        })
    }
}

impl<A: AppLike + 'static> State<A> for PromptInput<A> {
    fn event(&mut self, ctx: &mut EventCtx, app: &mut A) -> Transition<A> {
        match self.panel.event(ctx) {
            Outcome::Clicked(x) => match x.as_ref() {
                "close" => Transition::Pop,
                "confirm" => {
                    let data = self.panel.text_box("input");
                    (self.cb.take().unwrap())(data, ctx, app)
                }
                _ => unreachable!(),
            },
            _ => {
                if ctx.normal_left_click() && ctx.canvas.get_cursor_in_screen_space().is_none() {
                    return Transition::Pop;
                }
                Transition::Keep
            }
        }
    }

    fn draw_baselayer(&self) -> DrawBaselayer {
        DrawBaselayer::PreviousState
    }

    fn draw(&self, g: &mut GfxCtx, app: &A) {
        grey_out_map(g, app);
        self.panel.draw(g);
    }
}

/// Display a message dialog.
pub struct PopupMsg {
    panel: Panel,
}

impl PopupMsg {
    pub fn new_state<A: AppLike>(
        ctx: &mut EventCtx,
        title: &str,
        lines: Vec<impl AsRef<str>>,
    ) -> Box<dyn State<A>> {
        let mut txt = Text::new();
        txt.add_line(Line(title).small_heading());
        for l in lines {
            txt.add_line(l);
        }
        Box::new(PopupMsg {
            panel: Panel::new_builder(Widget::col(vec![
                txt.into_widget(ctx),
                ctx.style()
                    .btn_solid_primary
                    .text("OK")
                    .hotkey(hotkeys(vec![Key::Enter, Key::Escape]))
                    .build_def(ctx),
            ]))
            .build(ctx),
        })
    }
}

impl<A: AppLike> State<A> for PopupMsg {
    fn event(&mut self, ctx: &mut EventCtx, _: &mut A) -> Transition<A> {
        match self.panel.event(ctx) {
            Outcome::Clicked(x) => match x.as_ref() {
                "OK" => Transition::Pop,
                _ => unreachable!(),
            },
            _ => {
                if ctx.normal_left_click() && ctx.canvas.get_cursor_in_screen_space().is_none() {
                    return Transition::Pop;
                }
                Transition::Keep
            }
        }
    }

    fn draw_baselayer(&self) -> DrawBaselayer {
        DrawBaselayer::PreviousState
    }

    fn draw(&self, g: &mut GfxCtx, _: &A) {
        // This is a copy of grey_out_map from map_gui, with no dependencies on App
        g.fork_screenspace();
        g.draw_polygon(
            Color::BLACK.alpha(0.6),
            Polygon::rectangle(g.canvas.window_width, g.canvas.window_height),
        );
        g.unfork();

        self.panel.draw(g);
    }
}

pub struct FilePicker;

impl FilePicker {
    pub fn new_state<A: 'static + AppLike>(
        ctx: &mut EventCtx,
        start_dir: Option<String>,
        on_load: Box<dyn FnOnce(&mut EventCtx, &mut A, Result<Option<String>>) -> Transition<A>>,
    ) -> Box<dyn State<A>> {
        let (_, outer_progress_rx) = futures_channel::mpsc::channel(1);
        let (_, inner_progress_rx) = futures_channel::mpsc::channel(1);
        FutureLoader::<A, Option<String>>::new_state(
            ctx,
            Box::pin(async move {
                let mut builder = rfd::AsyncFileDialog::new();
                if let Some(dir) = start_dir {
                    builder = builder.set_directory(&dir);
                }
                let result = builder.pick_file().await.map(|x| {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        x.path().display().to_string()
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        format!("TODO rfd on wasm: {:?}", x)
                    }
                });
                let wrap: Box<dyn Send + FnOnce(&A) -> Option<String>> =
                    Box::new(move |_: &A| result);
                Ok(wrap)
            }),
            outer_progress_rx,
            inner_progress_rx,
            "Waiting for a file to be chosen",
            on_load,
        )
    }
}
