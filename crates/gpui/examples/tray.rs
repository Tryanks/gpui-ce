use gpui::{
    App, Application, Context, Div, Global, MenuItem, QuitMode, SharedString, Stateful, Tray,
    Window, WindowOptions, actions, div, prelude::*,
};

struct Example;

impl Render for Example {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        fn button(id: &'static str) -> Stateful<Div> {
            div()
                .id(id)
                .py_0p5()
                .px_3()
                .bg(gpui::black())
                .active(|this| this.bg(gpui::black().opacity(0.8)))
                .text_color(gpui::white())
        }

        let app_state = cx.global::<AppState>();

        div()
            .bg(gpui::white())
            .flex()
            .flex_col()
            .gap_4()
            .size_full()
            .justify_center()
            .items_center()
            .child("Example for set Tray Icon")
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_3()
                    .child(
                        button("toggle-visible")
                            .child(format!("Visible: {}", app_state.tray.visible))
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Box::new(ToggleVisible), cx);
                            }),
                    )
                    .child(
                        button("toggle-mode")
                            .child(format!("Mode: {}", app_state.view_mode.as_str()))
                            .on_click(|_, window, cx| {
                                window.dispatch_action(Box::new(ToggleCheck), cx);
                            }),
                    ),
            )
    }
}

fn main() {
    Application::new()
        .with_quit_mode(QuitMode::Explicit)
        .run(|cx: &mut App| {
            cx.set_global(AppState::new());

            // Bring the menu bar to the foreground (so you can see the menu bar)
            cx.activate(true);
            // Register the `quit` function so it can be referenced by the `MenuItem::action` in the menu bar
            cx.on_action(quit);
            cx.on_action(toggle_check);
            cx.on_action(toggle_visible);
            cx.on_action(hide_window);
            cx.on_action(show_window);

            // Hide Dock icon when last window is closed
            cx.on_window_closed(|cx| {
                if cx.windows().is_empty() {
                    cx.set_shows_in_dock(false);
                }
            })
            .detach();

            cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Example))
                .unwrap();

            let app_state = cx.global::<AppState>();
            cx.set_tray(app_state.tray.clone());
        });
}

#[derive(PartialEq)]
enum ViewMode {
    List,
    Grid,
}

impl ViewMode {
    fn as_str(&self) -> &'static str {
        match self {
            ViewMode::List => "List",
            ViewMode::Grid => "Grid",
        }
    }

    fn toggle(&mut self) {
        *self = match self {
            ViewMode::List => ViewMode::Grid,
            ViewMode::Grid => ViewMode::List,
        }
    }
}

impl Into<SharedString> for ViewMode {
    fn into(self) -> SharedString {
        match self {
            ViewMode::List => "List",
            ViewMode::Grid => "Grid",
        }
        .into()
    }
}

struct AppState {
    view_mode: ViewMode,
    tray: Tray,
}

impl AppState {
    fn new() -> Self {
        Self {
            view_mode: ViewMode::List,
            tray: Tray::new()
                .icon(gpui::Image::from_bytes(
                    gpui::ImageFormat::Png,
                    include_bytes!("image/app-icon.png").to_vec(),
                ))
                .icon_as_template(cfg!(target_os = "macos"))
                .title("Tray App")
                .tooltip("This is a tray icon")
                .menu(Self::build_menus),
        }
    }

    fn build_menus(cx: &mut App) -> Vec<MenuItem> {
        let app_state = cx.global::<AppState>();

        vec![
            MenuItem::action(ViewMode::List, ToggleCheck)
                .checked(app_state.view_mode == ViewMode::List),
            MenuItem::action(ViewMode::Grid, ToggleCheck)
                .checked(app_state.view_mode == ViewMode::Grid),
            MenuItem::separator(),
            MenuItem::action("Hide Window", HideWindow),
            MenuItem::action("Show Window", ShowWindow),
            MenuItem::separator(),
            MenuItem::action("Hide Tray Icon", ToggleVisible),
            MenuItem::submenu(gpui::Menu {
                name: "Submenu".into(),
                items: vec![
                    MenuItem::action("Toggle Check", ToggleCheck),
                    MenuItem::action("Toggle Visible", ToggleVisible),
                ],
            }),
            MenuItem::separator(),
            MenuItem::action("Quit", Quit),
        ]
    }
}

impl Global for AppState {}

// Associate actions using the `actions!` macro (or `Action` derive macro)
actions!(
    example,
    [Quit, ToggleCheck, ToggleVisible, HideWindow, ShowWindow]
);

// Define the quit function that is registered with the App
fn quit(_: &Quit, cx: &mut App) {
    println!("Gracefully quitting the application . . .");
    cx.quit();
}

fn toggle_check(_: &ToggleCheck, cx: &mut App) {
    println!("Toggling view mode . . .");

    {
        let app_state = cx.global_mut::<AppState>();
        app_state.view_mode.toggle();
        app_state.tray.title = Some(format!("Mode: {}", app_state.view_mode.as_str()).into());
        app_state.tray.tooltip =
            Some(format!("This is a tooltip, mode: {}", app_state.view_mode.as_str()).into());
    }

    let app_state = cx.global::<AppState>();
    cx.set_tray(app_state.tray.clone());
    cx.refresh_windows();
}

fn toggle_visible(_: &ToggleVisible, cx: &mut App) {
    let app_state = cx.global_mut::<AppState>();
    app_state.tray.visible = !app_state.tray.visible;

    let app_state = cx.global::<AppState>();
    cx.set_tray(app_state.tray.clone());
    cx.refresh_windows();
}

fn hide_window(_: &HideWindow, cx: &mut App) {
    // Use defer to avoid reentrancy conflict when closing the active window
    cx.defer(|cx| {
        let handles: Vec<_> = cx.windows().iter().cloned().collect();
        for handle in handles {
            let _ = handle.update(cx, |_, window, _| {
                window.remove_window();
            });
        }
    });
}

fn show_window(_: &ShowWindow, cx: &mut App) {
    cx.set_shows_in_dock(true);

    if cx.active_window().is_some() || !cx.windows().is_empty() {
        cx.activate(true);
        return;
    }

    cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Example))
        .unwrap();
    cx.activate(true);
}
