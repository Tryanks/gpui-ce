use std::collections::HashMap;
use calloop::channel::Sender;

use serde::Serialize;
use zbus::{
    interface,
    object_server::SignalContext,
    zvariant::{Structure, StructureBuilder, Type, Value},
};

pub const DBUS_MENU_PATH: &str = "/MenuBar";

#[derive(Default, Serialize, Type)]
pub struct DBusMenuLayoutItem<'a> {
    pub id: i32,
    #[zvariant(signature = "dict")]
    pub properties: HashMap<String, Value<'a>>,
    pub children: Vec<Value<'a>>,
}

impl<'a> Clone for DBusMenuLayoutItem<'a> {
    fn clone(&self) -> Self {
        let properties: HashMap<String, Value<'a>> = self
            .properties
            .iter()
            .map(|(k, v)| (k.clone(), v.try_clone().unwrap()))
            .collect();
        let children: Vec<_> = self
            .children
            .iter()
            .map(|it| it.try_clone().unwrap())
            .collect();
        Self {
            id: self.id.clone(),
            properties,
            children,
        }
    }
}

impl<'a> From<DBusMenuLayoutItem<'a>> for Structure<'a> {
    fn from(value: DBusMenuLayoutItem<'a>) -> Self {
        StructureBuilder::new()
            .add_field(value.id)
            .add_field(value.properties)
            .add_field(value.children)
            .build()
    }
}

#[derive(Default, Clone)]
pub struct Submenu {
    pub id: i32,
    pub icon_name: Option<String>,
    pub label: Option<String>,
    pub children: Vec<Submenu>,
}

impl<'a> From<Submenu> for DBusMenuLayoutItem<'a> {
    fn from(value: Submenu) -> Self {
        let mut menu = DBusMenuLayoutItem {
            id: value.id,
            ..Default::default()
        };
        if let Some(icon) = value.icon_name {
            menu.properties
                .insert("icon-name".into(), Value::from(icon));
        }
        if let Some(label) = value.label {
            menu.properties.insert("label".into(), Value::from(label));
        }
        if !value.children.is_empty() {
            menu.properties
                .insert("children-display".into(), Value::from("submenu"));
            for child in value.children {
                menu.children.push(Value::from(Self::from(child)));
            }
        }
        menu
    }
}

#[derive(Default)]
pub struct Menu {
    pub children: Vec<Submenu>,
}

#[derive(Default)]
pub struct DBusMenuInterface {
    pub menu: Menu,
    // Forward DBus menu events to the StatusNotifierItem event stream
    pub event_sender: Option<Sender<super::item::StatusNotifierItemEvents>>,
}

#[interface(name = "com.canonical.dbusmenu")]
impl DBusMenuInterface {
    // TODO: This is not done.
    #[zbus(out_args("revision", "layout"))]
    pub async fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<String>,
    ) -> (u32, DBusMenuLayoutItem) {
        let mut main_menu = DBusMenuLayoutItem::default();
        if !self.menu.children.is_empty() {
            main_menu
                .properties
                .insert("children-display".into(), Value::from("submenu"));
            for child in &self.menu.children {
                let submenu = DBusMenuLayoutItem::from(child.clone());
                main_menu.children.push(Value::from(submenu));
            }
        }
        (0, main_menu)
    }

    // Minimal event handling: translate a "clicked" event into a MenuClick with the same id
    pub async fn event(&self, id: i32, event_id: String, _event_data: Value<'_>, _timestamp: u32) {
        if event_id == "clicked" {
            if let Some(sender) = &self.event_sender {
                // Ignore send errors (receiver dropped) on purpose
                let _ = sender.send(super::item::StatusNotifierItemEvents::MenuEvent(
                    DBusMenuEvents::MenuClick(id),
                ));
            }
        }
    }

    // TODO: This is not done.
    pub async fn about_to_show(&self, id: i32) -> bool {
        false
    }

    #[zbus(signal, name = "LayoutUpdated")]
    pub async fn layout_updated(
        &self,
        cx: &SignalContext<'_>,
        revision: u32,
        parent: i32,
    ) -> zbus::Result<()>;
}

#[derive(Debug, Clone, Type, Serialize)]
pub enum DBusMenuEvents {
    MenuClick(i32),
}

pub struct DBusMenu(zbus::Connection);

impl DBusMenu {
    pub async fn new(
        menu: Menu,
        event_sender: Sender<super::item::StatusNotifierItemEvents>,
    ) -> zbus::Result<Self> {
        let iface = DBusMenuInterface {
            menu,
            event_sender: Some(event_sender),
        };
        let conn = zbus::connection::Builder::session()?
            .serve_at(DBUS_MENU_PATH, iface)?
            .build()
            .await?;
        Ok(Self(conn))
    }
}
