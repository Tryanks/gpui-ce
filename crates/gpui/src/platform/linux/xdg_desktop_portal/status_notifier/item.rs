use std::fmt::Display;
use std::str::FromStr;

use calloop::channel::{self, Channel, Sender};
use calloop::{EventSource, Poll, PostAction, Readiness, Token, TokenFactory};
use zbus::object_server::InterfaceRef;
use zbus::zvariant::{OwnedObjectPath, Structure, StructureBuilder};
use zbus::{interface, object_server::SignalContext, zvariant::Type};

use super::dbusmenu::{DBusMenuInterface, Menu, DBUS_MENU_PATH};
use super::watcher::StatusNotifierWatcher;

const STATUS_NOTIFIER_ITEM_PATH: &str = "/StatusNotifierItem";

#[allow(dead_code)]
#[derive(Default, Debug, Type)]
pub enum Category {
    // The item describes the status of a generic application,
    // for instance the current state of a media player.
    // In the case where the category of the item can not be known,
    // such as when the item is being proxied from another incompatible or emulated system,
    // ApplicationStatus can be used a sensible default fallback.
    #[default]
    ApplicationStatus,
    // The item describes the status of communication oriented applications,
    // like an instant messenger or an email client.
    Communications,
    // The item describes services of the system not seen as a stand alone application by the user,
    // such as an indicator for the activity of a disk indexing service.
    SystemServices,
    // The item describes the state and control of a particular hardware,
    // such as an indicator of the battery charge or sound card volume control.
    Hardware,
}

impl Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[allow(dead_code)]
#[derive(Type, Default, Debug)]
pub enum Status {
    #[default]
    Active,
    Passive,
    NeedsAttention,
}

impl FromStr for Status {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Active" => Ok(Self::Active),
            "Passive" => Ok(Self::Passive),
            "NeedsAttention" => Ok(Self::NeedsAttention),
            _ => Err(()),
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Type, Default, Debug, Clone)]
pub struct Pixmap {
    pub width: i32,
    pub height: i32,
    pub bytes: Vec<u8>,
}

impl From<Pixmap> for Structure<'_> {
    fn from(value: Pixmap) -> Self {
        StructureBuilder::new()
            .add_field(value.width)
            .add_field(value.height)
            .add_field(value.bytes)
            .build()
    }
}

#[derive(Default, Debug, Clone, Type)]
pub struct ToolTip {
    pub icon: Icon,
    pub title: String,
    pub description: String,
}

impl ToolTip {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }
}

impl From<ToolTip> for Structure<'_> {
    fn from(value: ToolTip) -> Self {
        StructureBuilder::new()
            .add_field::<String>(value.icon.clone().name_or_default())
            .add_field::<Vec<Pixmap>>(value.icon.pixmaps_or_default())
            .add_field(value.title)
            .add_field(value.description)
            .build()
    }
}

#[derive(Clone, Debug, Type)]
#[zvariant(signature = "(sv)")]
pub enum Icon {
    Name(String),
    Pixmaps(Vec<Pixmap>),
}

impl Icon {
    fn name_or_default(self) -> String {
        if let Self::Name(name) = self {
            name
        } else {
            String::default()
        }
    }

    fn pixmaps_or_default(self) -> Vec<Pixmap> {
        if let Self::Pixmaps(pixmaps) = self {
            pixmaps
        } else {
            Vec::default()
        }
    }
}

impl Default for Icon {
    fn default() -> Self {
        Self::Name(String::default())
    }
}

#[derive(Default, Debug, Clone, Type)]
pub struct Attention {
    pub icon: Icon,
    pub movie_name: String,
}

#[derive(Default, Debug, Type)]
pub struct StatusNotifierItemOptions {
    pub(crate) category: Category,
    pub(crate) title: String,
    pub(crate) status: Status,
    pub(crate) window_id: u32,
    pub(crate) icon_theme_path: String,
    pub(crate) icon: Icon,
    pub(crate) overlay: Icon,
    pub(crate) attention: Attention,
    pub(crate) is_menu: bool,
    pub(crate) tooltip: ToolTip,
}

impl StatusNotifierItemOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = icon;
        self
    }

    pub fn tooltip(mut self, tooltip: ToolTip) -> Self {
        self.tooltip = tooltip;
        self
    }
}

#[derive(Default)]
struct Callbacks {
    on_context_menu: Option<Box<dyn Fn(i32, i32) + Sync + Send>>,
    on_activate: Option<Box<dyn Fn(i32, i32) + Sync + Send>>,
    on_secondary_activate: Option<Box<dyn Fn(i32, i32) + Sync + Send>>,
    on_scroll: Option<Box<dyn Fn(i32, String) + Sync + Send>>,
    on_provide_xdg_activation_token: Option<Box<dyn Fn(String) + Sync + Send>>,
}

struct StatusNotifierItemInterface {
    id: String,
    callbacks: Callbacks,
    options: StatusNotifierItemOptions,
    menu: Option<OwnedObjectPath>,
}

#[interface(name = "org.kde.StatusNotifierItem")]
impl StatusNotifierItemInterface {
    #[zbus(property, name = "Category")]
    pub async fn category(&self) -> String {
        self.options.category.to_string()
    }

    #[zbus(property, name = "Id")]
    pub async fn id(&self) -> String {
        self.id.clone()
    }

    #[zbus(property, name = "Title")]
    pub async fn title(&self) -> String {
        self.options.title.clone()
    }

    #[zbus(property, name = "Status")]
    pub async fn status(&self) -> String {
        self.options.status.to_string()
    }

    #[zbus(property, name = "WindowId")]
    pub async fn window_id(&self) -> u32 {
        self.options.window_id
    }

    #[zbus(property, name = "IconThemePath")]
    pub async fn icon_theme_path(&self) -> String {
        self.options.icon_theme_path.clone()
    }

    #[zbus(property, name = "IconName")]
    pub async fn icon_name(&self) -> String {
        self.options.icon.clone().name_or_default()
    }

    #[zbus(property, name = "IconPixmap")]
    pub async fn icon_pixmap(&self) -> Vec<Pixmap> {
        self.options.icon.clone().pixmaps_or_default()
    }

    #[zbus(property, name = "OverlayIconName")]
    pub async fn overlay_icon_name(&self) -> String {
        self.options.overlay.clone().name_or_default()
    }

    #[zbus(property, name = "OverlayIconPixmap")]
    pub async fn overlay_icon_pixmap(&self) -> Vec<Pixmap> {
        self.options.overlay.clone().pixmaps_or_default()
    }

    #[zbus(property, name = "AttentionIconName")]
    pub async fn attention_icon_name(&self) -> String {
        self.options.attention.icon.clone().name_or_default()
    }

    #[zbus(property, name = "AttentionIconPixmap")]
    pub async fn attention_icon_pixmap(&self) -> Vec<Pixmap> {
        self.options.attention.icon.clone().pixmaps_or_default()
    }

    #[zbus(property, name = "AttentionMovieName")]
    pub async fn attention_movie_name(&self) -> String {
        self.options.attention.movie_name.clone()
    }

    #[zbus(property, name = "ToolTip")]
    pub async fn tooltip(&self) -> ToolTip {
        self.options.tooltip.clone()
    }

    #[zbus(property, name = "ItemIsMenu")]
    pub async fn item_is_menu(&self) -> bool {
        self.options.is_menu
    }

    #[zbus(property, name = "Menu")]
    pub async fn menu(&self) -> OwnedObjectPath {
        self.menu.clone().unwrap_or_default()
    }

    #[zbus(property, name = "Category")]
    pub async fn set_title(&mut self, title: String) {
        self.options.title = title;
    }

    pub async fn context_menu(&self, x: i32, y: i32) {
        self.callbacks
            .on_context_menu
            .as_ref()
            .map(move |menu| menu(x, y));
    }

    pub async fn activate(&self, x: i32, y: i32) {
        self.callbacks
            .on_activate
            .as_ref()
            .map(move |activate| activate(x, y));
    }

    pub async fn secondary_activate(&self, x: i32, y: i32) {
        self.callbacks
            .on_secondary_activate
            .as_ref()
            .map(move |activate| activate(x, y));
    }

    pub async fn scroll(&self, delta: i32, orientation: String) {
        self.callbacks
            .on_scroll
            .as_ref()
            .map(move |scroll| scroll(delta, orientation));
    }

    pub async fn provide_xdg_activation_token(&self, token: String) {
        self.callbacks
            .on_provide_xdg_activation_token
            .as_ref()
            .map(move |xdg_activation_token| xdg_activation_token(token));
    }

    #[zbus(signal, name = "NewTitle")]
    pub async fn new_title(&self, cx: &SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal, name = "NewIcon")]
    pub async fn new_icon(&self, cx: &SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal, name = "NewAttentionIcon")]
    pub async fn new_attention_icon(&self, cx: &SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal, name = "NewOverlayIcon")]
    pub async fn new_overlay_icon(&self, cx: &SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal, name = "NewToolTip")]
    pub async fn new_tooltip(&self, cx: &SignalContext<'_>) -> zbus::Result<()>;

    #[zbus(signal, name = "NewStatus")]
    pub async fn new_status(&self, cx: &SignalContext<'_>, status: String) -> zbus::Result<()>;
}

pub struct StatusNotifierItem {
    conn: zbus::Connection,
    iface_ref: InterfaceRef<StatusNotifierItemInterface>,
    channel: Channel<StatusNotifierItemEvents>,
    sender: Sender<StatusNotifierItemEvents>,
}

#[derive(Debug, Clone)]
pub enum StatusNotifierItemEvents {
    Activate(i32, i32),
    SecondaryActivate(i32, i32),
    XdgActivationToken(String),
    Scroll(i32, String),
    MenuEvent(super::dbusmenu::DBusMenuEvents),
}

impl StatusNotifierItem {
    pub async fn new(id: i32, options: StatusNotifierItemOptions, menu: Option<super::dbusmenu::Menu>) -> zbus::Result<Self> {
        let watcher = StatusNotifierWatcher::new().await?;
        let menu_path = if menu.is_some() {
            Some(
                OwnedObjectPath::try_from(DBUS_MENU_PATH)
                    .map_err(|e| zbus::Error::Failure(e.to_string()))?,
            )
        } else {
            None
        };
        let iface = StatusNotifierItemInterface {
            id: id.to_string(),
            options,
            callbacks: Default::default(),
            menu: menu_path,
        };
        let name = format!(
            "org.freedesktop.StatusNotifierItem-{}-{}",
            std::process::id(),
            id
        );

        let conn = zbus::connection::Builder::session()?
            .name(name.clone())?
            .serve_at(STATUS_NOTIFIER_ITEM_PATH, iface)?
            .build()
            .await?;
        // Create an internal channel for emitting events to calloop
        let (sender, channel) = channel::channel();

        if let Some(menu) = menu {
            // Host the DBusMenu interface on the same object server
            let iface = DBusMenuInterface { menu, event_sender: Some(sender.clone()) };
            conn.object_server().at(DBUS_MENU_PATH, iface).await?;
        }
        watcher.register_status_notifier_item(name).await?;
        let iface_ref = conn
            .object_server()
            .interface::<_, StatusNotifierItemInterface>(STATUS_NOTIFIER_ITEM_PATH)
            .await?;
        let this = Self { conn, iface_ref, channel, sender };

        // Hook up interface callbacks to forward into our event stream
        this.on_activate(Box::new({
            let tx = this.sender.clone();
            move |x, y| {
                let _ = tx.send(StatusNotifierItemEvents::Activate(x, y));
            }
        })).await;
        this.on_secondary_activate(Box::new({
            let tx = this.sender.clone();
            move |x, y| {
                let _ = tx.send(StatusNotifierItemEvents::SecondaryActivate(x, y));
            }
        })).await;
        this.on_scroll(Box::new({
            let tx = this.sender.clone();
            move |d, ori| {
                let _ = tx.send(StatusNotifierItemEvents::Scroll(d, ori));
            }
        })).await;
        this.on_provide_xdg_activation_token(Box::new({
            let tx = this.sender.clone();
            move |token| {
                let _ = tx.send(StatusNotifierItemEvents::XdgActivationToken(token));
            }
        })).await;

        Ok(this)
    }

    pub async fn on_context_menu(&self, fun: Box<dyn Fn(i32, i32) + Sync + Send>) {
        let mut iface = self.iface_ref.get_mut().await;
        iface.callbacks.on_context_menu = Some(fun);
    }

    pub async fn on_activate(&self, fun: Box<dyn Fn(i32, i32) + Sync + Send>) {
        let mut iface = self.iface_ref.get_mut().await;
        iface.callbacks.on_activate = Some(fun);
    }

    pub async fn on_secondary_activate(&self, fun: Box<dyn Fn(i32, i32) + Sync + Send>) {
        let mut iface = self.iface_ref.get_mut().await;
        iface.callbacks.on_secondary_activate = Some(fun);
    }

    pub async fn on_scroll(&self, fun: Box<dyn Fn(i32, String) + Sync + Send>) {
        let mut iface = self.iface_ref.get_mut().await;
        iface.callbacks.on_scroll = Some(fun);
    }

    pub async fn on_provide_xdg_activation_token(&self, fun: Box<dyn Fn(String) + Sync + Send>) {
        let mut iface = self.iface_ref.get_mut().await;
        iface.callbacks.on_provide_xdg_activation_token = Some(fun);
    }

    pub async fn set_title(&self, name: impl Into<String>) -> zbus::Result<()> {
        let cx = self.iface_ref.signal_context();
        let mut iface = self.iface_ref.get_mut().await;
        iface.options.title = name.into();
        iface.new_title(cx).await?;
        Ok(())
    }

    pub async fn set_icon(&self, icon: Icon) -> zbus::Result<()> {
        let cx = self.iface_ref.signal_context();
        let mut iface = self.iface_ref.get_mut().await;
        iface.options.icon = icon;
        iface.new_icon(cx).await?;
        Ok(())
    }

    pub async fn set_overlay(&self, overlay: Icon) -> zbus::Result<()> {
        let cx = self.iface_ref.signal_context();
        let mut iface = self.iface_ref.get_mut().await;
        iface.options.overlay = overlay;
        iface.new_icon(cx).await?;
        Ok(())
    }

    pub async fn set_attention(&self, attention: Attention) -> zbus::Result<()> {
        let cx = self.iface_ref.signal_context();
        let mut iface = self.iface_ref.get_mut().await;
        iface.options.attention = attention;
        iface.new_icon(cx).await?;
        Ok(())
    }

    pub async fn set_tooltip(&self, tooltip: ToolTip) -> zbus::Result<()> {
        let cx = self.iface_ref.signal_context();
        let mut iface = self.iface_ref.get_mut().await;
        iface.options.tooltip = tooltip;
        iface.new_tooltip(cx).await?;
        Ok(())
    }

    pub async fn set_tooltip_title(&self, title: String) -> zbus::Result<()> {
        let cx = self.iface_ref.signal_context();
        let mut iface = self.iface_ref.get_mut().await;
        iface.options.tooltip.title = title;
        iface.new_tooltip(cx).await?;
        Ok(())
    }

    pub async fn set_tooltip_description(&self, description: String) -> zbus::Result<()> {
        let cx = self.iface_ref.signal_context();
        let mut iface = self.iface_ref.get_mut().await;
        iface.options.tooltip.description = description;
        iface.new_tooltip(cx).await?;
        Ok(())
    }

    pub async fn set_status(&self, status: Status) -> zbus::Result<()> {
        let cx = self.iface_ref.signal_context();
        let mut iface = self.iface_ref.get_mut().await;
        let status_str = status.to_string();
        iface.options.status = status;
        iface.new_status(cx, status_str).await?;
        Ok(())
    }

    pub async fn set_category(&self, category: Category) -> zbus::Result<()> {
        let cx = self.iface_ref.signal_context();
        let mut iface = self.iface_ref.get_mut().await;
        let category_str = category.to_string();
        iface.options.category = category;
        iface.new_status(cx, category_str).await?;
        Ok(())
    }
}

impl EventSource for StatusNotifierItem {
    type Event = StatusNotifierItemEvents;
    type Metadata = ();
    type Ret = ();
    type Error = anyhow::Error;

    fn process_events<F>(
        &mut self,
        readiness: Readiness,
        token: Token,
        mut callback: F,
    ) -> Result<PostAction, Self::Error>
    where
        F: FnMut(Self::Event, &mut Self::Metadata) -> Self::Ret,
    {
        self.channel.process_events(readiness, token, |evt, _| {
            if let calloop::channel::Event::Msg(msg) = evt {
                (callback)(msg, &mut ())
            }
        })?;
        Ok(PostAction::Continue)
    }

    fn register(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> calloop::Result<()> {
        self.channel.register(poll, token_factory)
    }

    fn reregister(&mut self, poll: &mut Poll, token_factory: &mut TokenFactory) -> calloop::Result<()> {
        self.channel.reregister(poll, token_factory)
    }

    fn unregister(&mut self, poll: &mut Poll) -> calloop::Result<()> {
        self.channel.unregister(poll)
    }
}
