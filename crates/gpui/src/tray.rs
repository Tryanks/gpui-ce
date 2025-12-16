use crate::{App, MenuItem, SharedString};
use anyhow::Result;
use std::rc::Rc;

/// System tray icon.
#[derive(Clone)]
pub struct Tray {
    /// Tooltip text.
    pub tooltip: Option<SharedString>,
    /// Tray title after the Icon.
    pub title: Option<SharedString>,
    /// Tray icon image.
    pub icon: Option<Rc<gpui::Image>>,
    pub(crate) icon_data: Option<TrayIconData>,

    /// Whether the icon should be treated as a template image on platforms that support it
    /// (e.g. macOS menu bar).
    ///
    /// Template images are typically single-color glyphs that the system can automatically
    /// render appropriately for light/dark appearances.
    pub icon_is_template: bool,

    /// Function to build the context menu.
    pub menu_builder: Option<Rc<dyn Fn(&mut App) -> Vec<MenuItem>>>,
    /// Visibility of the tray icon.
    pub visible: bool,
}

impl Tray {
    pub(crate) fn render_icon(&mut self, cx: &App) -> Result<()> {
        if let Some(icon) = &self.icon {
            let image = icon.to_image_data(cx.svg_renderer())?;
            let bytes = image.as_bytes(0).unwrap_or_default();
            let size = image.size(0);

            self.icon_data = Some(TrayIconData {
                data: Rc::new(bytes.to_vec()),
                width: size.width.0 as u32,
                height: size.height.0 as u32,
            })
        }
        Ok(())
    }
}

#[derive(Clone)]
#[allow(unused)]
pub(crate) struct TrayIconData {
    pub(crate) data: Rc<Vec<u8>>,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl Tray {
    /// Create a new tray icon with default properties.
    pub fn new() -> Self {
        Self {
            tooltip: None,
            title: None,
            icon: None,
            icon_data: None,
            icon_is_template: false,
            menu_builder: None,
            visible: true,
        }
    }

    /// Set the tooltip text, defaults to None.
    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Set the title text, defaults to None.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the icon image, defaults to None.
    pub fn icon(mut self, icon: impl Into<gpui::Image>) -> Self {
        self.icon = Some(Rc::new(icon.into()));
        self
    }

    /// On supported platforms (e.g. macOS), mark the tray icon image as a template.
    ///
    /// When set to `true`, the system may automatically adjust the icon for light/dark mode.
    /// Default is `false`.
    pub fn icon_as_template(mut self, is_template: bool) -> Self {
        self.icon_is_template = is_template;
        self
    }

    /// Set the context menu.
    pub fn menu<F>(mut self, builder: F) -> Self
    where
        F: Fn(&mut App) -> Vec<MenuItem> + 'static,
    {
        self.menu_builder = Some(Rc::new(builder));
        self
    }

    /// Set visibility of the tray icon, default is true.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
}
