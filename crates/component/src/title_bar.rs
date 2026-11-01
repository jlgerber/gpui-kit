use std::rc::Rc;

use crate::{
    ActiveTheme, Icon, IconName, InteractiveElementExt as _, Sizable as _, StyledExt, h_flex,
};
use gpui::{
    AnyElement, App, Background, ClickEvent, Context, Decorations, Hsla, InteractiveElement,
    IntoElement, MouseButton, ParentElement, Pixels, Render, RenderOnce, Rgba,
    StatefulInteractiveElement as _, StyleRefinement, Styled, TitlebarOptions, Window,
    WindowControlArea, WindowOptions, div, linear_color_stop, linear_gradient,
    prelude::FluentBuilder as _, px,
};
use gpui_base::TestSupportExt as _;
use smallvec::SmallVec;

pub const TITLE_BAR_HEIGHT: Pixels = px(34.);
#[cfg(target_os = "macos")]
const TITLE_BAR_LEFT_PADDING: Pixels = px(80.);
#[cfg(not(target_os = "macos"))]
const TITLE_BAR_LEFT_PADDING: Pixels = px(12.);

fn default_title_bar_background(title_bar: Hsla, background: Hsla) -> Background {
    let title_bar_rgb = title_bar.to_rgb();
    let background_rgb = background.to_rgb();
    let mixed = Hsla::from(Rgba {
        r: title_bar_rgb.r * 0.55 + background_rgb.r * 0.45,
        g: title_bar_rgb.g * 0.55 + background_rgb.g * 0.45,
        b: title_bar_rgb.b * 0.55 + background_rgb.b * 0.45,
        a: title_bar_rgb.a * 0.55 + background_rgb.a * 0.45,
    });

    linear_gradient(
        180.,
        linear_color_stop(mixed, 0.),
        linear_color_stop(title_bar, 1.),
    )
}

/// TitleBar used to customize the appearance of the title bar.
///
/// We can put some elements inside the title bar.
#[derive(IntoElement)]
pub struct TitleBar {
    style: StyleRefinement,
    children: SmallVec<[AnyElement; 1]>,
    on_close_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,
    on_maximize_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,
}

impl TitleBar {
    /// Create a new TitleBar.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            children: SmallVec::new(),
            on_close_window: None,
            on_maximize_window: None,
        }
    }

    /// Returns the default title bar options for compatible with the [`crate::TitleBar`].
    pub fn title_bar_options() -> TitlebarOptions {
        TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: Some(gpui::point(px(9.0), px(9.0))),
        }
    }

    /// Returns the default window options for compatible with the [`crate::TitleBar`].
    ///
    /// Use this as the base of the [`WindowOptions`] of any window that renders a
    /// [`crate::TitleBar`], so the title bar owns dragging and double clicking itself:
    ///
    /// ```no_run
    /// # mod gpui_kit { pub use gpui::*; pub extern crate gpui_component as component; }
    /// # use gpui_kit::WindowOptions;
    /// # use gpui_kit::component::TitleBar;
    /// let options = WindowOptions {
    ///     window_min_size: None,
    ///     ..TitleBar::window_options()
    /// };
    /// ```
    pub fn window_options() -> WindowOptions {
        WindowOptions {
            titlebar: Some(Self::title_bar_options()),
            // The title bar draws itself and moves the window via `start_window_move`,
            // so AppKit must not treat it as a system window-move region. Otherwise macOS
            // handles title bar double clicks on its own (in addition to `on_double_click`
            // below) and delays title bar clicks while disambiguating double clicks.
            app_owns_titlebar_drag: true,
            ..Default::default()
        }
    }

    /// Add custom for close window event, default is None, then click X button will call `window.remove_window()`.
    /// Linux only, this will do nothing on other platforms.
    pub fn on_close_window(
        mut self,
        f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        if cfg!(target_os = "linux") {
            self.on_close_window = Some(Rc::new(Box::new(f)));
        }
        self
    }

    /// Add custom for maximize / restore window event, default is None, then
    /// the maximize button and a title bar double click call
    /// `window.zoom_window()`. For a compositor that ignores the client's
    /// maximize request and must be asked some other way.
    /// Linux only, this will do nothing on other platforms.
    pub fn on_maximize_window(
        mut self,
        f: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        if cfg!(target_os = "linux") {
            self.on_maximize_window = Some(Rc::new(Box::new(f)));
        }
        self
    }
}

// The Windows control buttons have a fixed width of 35px.
//
// We don't need implementation the click event for the control buttons.
// If user clicked in the bounds, the window event will be triggered.
#[derive(IntoElement, Clone)]
enum ControlIcon {
    Minimize,
    Restore {
        on_maximize_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,
    },
    Maximize {
        on_maximize_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,
    },
    Close {
        on_close_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,
    },
}

impl ControlIcon {
    fn minimize() -> Self {
        Self::Minimize
    }

    fn restore(
        on_maximize_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,
    ) -> Self {
        Self::Restore { on_maximize_window }
    }

    fn maximize(
        on_maximize_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,
    ) -> Self {
        Self::Maximize { on_maximize_window }
    }

    fn close(on_close_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>) -> Self {
        Self::Close { on_close_window }
    }

    fn id(&self) -> &'static str {
        match self {
            Self::Minimize => "minimize",
            Self::Restore { .. } => "restore",
            Self::Maximize { .. } => "maximize",
            Self::Close { .. } => "close",
        }
    }

    fn icon(&self) -> IconName {
        match self {
            Self::Minimize => IconName::WindowMinimize,
            Self::Restore { .. } => IconName::WindowRestore,
            Self::Maximize { .. } => IconName::WindowMaximize,
            Self::Close { .. } => IconName::WindowClose,
        }
    }

    fn window_control_area(&self) -> WindowControlArea {
        match self {
            Self::Minimize => WindowControlArea::Min,
            Self::Restore { .. } | Self::Maximize { .. } => WindowControlArea::Max,
            Self::Close { .. } => WindowControlArea::Close,
        }
    }

    fn is_close(&self) -> bool {
        matches!(self, Self::Close { .. })
    }

    #[inline]
    fn hover_fg(&self, cx: &App) -> Hsla {
        if self.is_close() {
            cx.theme().danger_foreground
        } else {
            cx.theme().secondary_foreground
        }
    }

    #[inline]
    fn hover_bg(&self, cx: &App) -> Hsla {
        if self.is_close() {
            cx.theme().danger
        } else {
            cx.theme().secondary_hover
        }
    }

    #[inline]
    fn active_bg(&self, cx: &mut App) -> Hsla {
        if self.is_close() {
            cx.theme().danger_active
        } else {
            cx.theme().secondary_active
        }
    }
}

impl RenderOnce for ControlIcon {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_linux = cfg!(target_os = "linux");
        let is_windows = cfg!(target_os = "windows");
        let hover_fg = self.hover_fg(cx);
        let hover_bg = self.hover_bg(cx);
        let active_bg = self.active_bg(cx);
        let icon = self.clone();
        let on_close_window = match &self {
            ControlIcon::Close { on_close_window } => on_close_window.clone(),
            _ => None,
        };

        div()
            .id(self.id())
            .test_support()
            .flex()
            .w(TITLE_BAR_HEIGHT)
            .h_full()
            .flex_shrink_0()
            .justify_center()
            .content_center()
            .items_center()
            .text_color(cx.theme().foreground)
            .hover(|style| style.bg(hover_bg).text_color(hover_fg))
            .active(|style| style.bg(active_bg).text_color(hover_fg))
            .when(is_windows, |this| {
                this.window_control_area(self.window_control_area())
            })
            .when(is_linux, |this| {
                this.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    window.prevent_default();
                    cx.stop_propagation();
                })
                .on_click(move |_, window, cx| {
                    cx.stop_propagation();
                    match &icon {
                        Self::Minimize => window.minimize_window(),
                        Self::Restore { on_maximize_window }
                        | Self::Maximize { on_maximize_window } => {
                            if let Some(f) = on_maximize_window {
                                f(&ClickEvent::default(), window, cx);
                            } else {
                                window.zoom_window();
                            }
                        }
                        Self::Close { .. } => {
                            if let Some(f) = on_close_window.clone() {
                                f(&ClickEvent::default(), window, cx);
                            } else {
                                window.remove_window();
                            }
                        }
                    }
                })
            })
            .child(Icon::new(self.icon()).small())
    }
}

/// `XDG_CURRENT_DESKTOP` names of compositors / window managers that grant
/// server-side decorations but draw no window buttons themselves: at most a
/// border (Hyprland, niri, river, dwl) or a title-only bar (sway, i3, qtile).
/// On these a window that skipped its own controls would have no close button
/// at all, so [`WindowControls`] draws them even under server decorations.
///
/// Deliberately absent: environments whose server-side title bars carry
/// buttons (KDE/KWin, labwc, Wayfire's decoration plugin, COSMIC, Xfce,
/// Openbox and the other stacking X11 WMs). Drawing ours there stacks a
/// second set of controls on theirs. GNOME's Mutter never grants server
/// decorations to Wayland clients, so it takes the client-decorated path.
#[cfg(any(target_os = "linux", test))]
const DESKTOPS_WITHOUT_SERVER_CONTROLS: &[&str] =
    &["Hyprland", "sway", "i3", "niri", "river", "dwl", "qtile"];

/// Whether a window should draw its own min/max/close controls.
///
/// `desktop` is the raw `XDG_CURRENT_DESKTOP` value: a colon-separated list,
/// matched case-insensitively. A client-decorated window always draws them;
/// a server-decorated one only when the desktop is known to draw none.
#[cfg(any(target_os = "linux", test))]
fn draws_own_controls(decorations_client: bool, desktop: Option<&str>) -> bool {
    decorations_client
        || desktop.is_some_and(|desktop| {
            desktop.split(':').map(str::trim).any(|name| {
                DESKTOPS_WITHOUT_SERVER_CONTROLS
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(name))
            })
        })
}

/// `XDG_CURRENT_DESKTOP`, read once: the session's desktop does not change
/// under a running process, and this is consulted on every title bar render.
#[cfg(target_os = "linux")]
fn current_desktop() -> Option<&'static str> {
    static DESKTOP: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    DESKTOP
        .get_or_init(|| std::env::var("XDG_CURRENT_DESKTOP").ok())
        .as_deref()
}

#[derive(IntoElement)]
struct WindowControls {
    on_close_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,
    on_maximize_window: Option<Rc<Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>>>,
}

impl RenderOnce for WindowControls {
    fn render(self, window: &mut Window, _: &mut App) -> impl IntoElement {
        if cfg!(target_os = "macos") || cfg!(target_family = "wasm") {
            return div().id("window-controls").test_support();
        }

        // Under server-side decorations the window manager usually renders
        // its own title bar, complete with min/max/close; drawing ours as
        // well stacks a duplicate set of controls on top of it (most
        // visibly two close buttons). gpui falls back to server-side
        // decorations on X11 sessions without a compositor, and a Wayland
        // compositor may grant server mode even when client mode was
        // requested. Tiling compositors such as Hyprland grant server mode
        // and then draw no buttons at all, so on the desktops listed in
        // `DESKTOPS_WITHOUT_SERVER_CONTROLS` ours are drawn regardless.
        #[cfg(target_os = "linux")]
        if !draws_own_controls(
            matches!(window.window_decorations(), Decorations::Client { .. }),
            current_desktop(),
        ) {
            return div().id("window-controls").test_support();
        }

        // The window manager declares which controls it can honor; a tiling
        // compositor may support neither minimize nor maximize. Close is
        // always ours to offer.
        let supported = window.window_controls();

        h_flex()
            .id("window-controls")
            .test_support()
            .items_center()
            .flex_shrink_0()
            .h_full()
            .when(supported.minimize, |this| {
                this.child(ControlIcon::minimize())
            })
            .when(supported.maximize, |this| {
                let on_maximize_window = self.on_maximize_window.clone();
                this.child(if window.is_maximized() {
                    ControlIcon::restore(on_maximize_window)
                } else {
                    ControlIcon::maximize(on_maximize_window)
                })
            })
            .child(ControlIcon::close(self.on_close_window))
    }
}

impl Styled for TitleBar {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for TitleBar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

struct TitleBarState {
    should_move: bool,
}

// TODO: Remove this when GPUI has released v0.2.3
impl Render for TitleBarState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl RenderOnce for TitleBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_client_decorated = matches!(window.window_decorations(), Decorations::Client { .. });
        let is_web = cfg!(target_family = "wasm");
        let is_linux = cfg!(target_os = "linux");
        let is_macos = cfg!(target_os = "macos");

        let state = window.use_state(cx, |_, _| TitleBarState { should_move: false });

        div().flex_shrink_0().child(
            div()
                .id("title-bar")
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .h(TITLE_BAR_HEIGHT)
                .pl(TITLE_BAR_LEFT_PADDING)
                .border_b_1()
                .border_color(cx.theme().title_bar_border)
                .bg(default_title_bar_background(
                    cx.theme().title_bar,
                    cx.theme().background,
                ))
                .refine_style(&self.style)
                .when(is_linux, |this| {
                    let on_maximize_window = self.on_maximize_window.clone();
                    this.on_double_click(move |event, window, cx| {
                        if let Some(f) = on_maximize_window.as_ref() {
                            f(event, window, cx);
                        } else {
                            window.zoom_window();
                        }
                    })
                })
                .when(is_macos, |this| {
                    this.on_double_click(|_, window, _| window.titlebar_double_click())
                })
                // After `on_double_click`, which is implemented for `Stateful` only.
                .test_support()
                .on_mouse_down_out(window.listener_for(&state, |state, _, _, _| {
                    state.should_move = false;
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    window.listener_for(&state, |state, _, _, _| {
                        state.should_move = true;
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    window.listener_for(&state, |state, _, _, _| {
                        state.should_move = false;
                    }),
                )
                .on_mouse_move(window.listener_for(&state, |state, _, window, _| {
                    if state.should_move {
                        state.should_move = false;
                        window.start_window_move();
                    }
                }))
                .child(
                    h_flex()
                        .id("bar")
                        .h_full()
                        .justify_between()
                        .flex_shrink_0()
                        .flex_1()
                        .when(!is_web, |this| {
                            this.window_control_area(WindowControlArea::Drag)
                                .when(window.is_fullscreen(), |this| this.pl_3())
                                // Unlike `WindowControls`, this stays client-only:
                                // the desktops that draw no server controls also
                                // offer no window menu (they do not advertise the
                                // `window_menu` capability), so the overlay would
                                // only swallow right clicks.
                                .when(is_linux && is_client_decorated, |this| {
                                    this.child(
                                        div()
                                            .top_0()
                                            .left_0()
                                            .absolute()
                                            .size_full()
                                            .h_full()
                                            .on_mouse_down(
                                                MouseButton::Right,
                                                move |ev, window, _| {
                                                    window.show_window_menu(ev.position)
                                                },
                                            ),
                                    )
                                })
                        })
                        .children(self.children),
                )
                .child(WindowControls {
                    on_close_window: self.on_close_window,
                    on_maximize_window: self.on_maximize_window,
                }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Rgba, linear_color_stop, linear_gradient};

    #[test]
    fn test_client_decorated_window_draws_own_controls_on_any_desktop() {
        assert!(draws_own_controls(true, None));
        assert!(draws_own_controls(true, Some("KDE")));
        assert!(draws_own_controls(true, Some("Hyprland")));
    }

    #[test]
    fn test_server_decorated_window_draws_own_controls_on_buttonless_tiling_desktops() {
        for desktop in ["Hyprland", "sway", "i3", "niri", "river", "dwl", "qtile"] {
            assert!(draws_own_controls(false, Some(desktop)), "{desktop}");
        }
    }

    #[test]
    fn test_server_decorated_window_skips_own_controls_where_the_server_draws_them() {
        for desktop in [
            "KDE",
            "labwc:wlroots",
            "Wayfire",
            "COSMIC",
            "XFCE",
            "GNOME",
            "",
        ] {
            assert!(!draws_own_controls(false, Some(desktop)), "{desktop}");
        }
        assert!(!draws_own_controls(false, None));
    }

    #[test]
    fn test_desktop_match_is_case_insensitive_over_colon_separated_entries() {
        assert!(draws_own_controls(false, Some("hyprland")));
        assert!(draws_own_controls(false, Some("SWAY")));
        assert!(draws_own_controls(false, Some("wlroots:sway")));
        assert!(draws_own_controls(false, Some("niri: wlroots")));
        assert!(!draws_own_controls(false, Some("Hyprlandish")));
    }

    #[test]
    fn test_default_title_bar_background() {
        let title_bar = Hsla::black();
        let background = Hsla::white();

        assert_eq!(
            default_title_bar_background(title_bar, background),
            linear_gradient(
                180.,
                linear_color_stop(
                    Hsla::from(Rgba {
                        r: 0.45,
                        g: 0.45,
                        b: 0.45,
                        a: 1.,
                    }),
                    0.,
                ),
                linear_color_stop(title_bar, 1.),
            )
        );
    }
}
