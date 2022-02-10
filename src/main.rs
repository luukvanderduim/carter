#![allow(unused_imports)]
use std::{error::Error, ops::Deref};

use futures_concurrency::prelude::*;
use futures_lite::{stream, StreamExt};
use ratspie::{a11y_bus_connection, proxies::text::TextProxy, registry::EventListenerRegistered};
use std::iter::Iterator;

use zbus::{
    fdo::{DBusProxy, RequestNameFlags},
    PropertyStream, ProxyDefault,
};
use zvariant::OwnedObjectPath;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Get a connection on the a11y bus.
    let a11y_bus = ratspie::a11y_bus_connection().await?;
    let dbus_proxy = DBusProxy::new(&a11y_bus).await?;

    assert!(a11y_bus.is_bus());

    dbus_proxy
        .request_name(
            "org.a11y.Carter".try_into().unwrap(),
            RequestNameFlags::ReplaceExisting.into(),
        )
        .await?;

    // Signal apps that we would like them to signal a11y events.
    // Courteous apps have subscribed to the accessibility is_enabled property(?)
    ratspie::set_session_accessibility_status(true).await?;

    // Get a proxy to the Registry service
    let registry = ratspie::registry::RegistryProxy::new(&a11y_bus).await?;

    let mut registered_events: Vec<(String, String)> = Vec::new();
    if registry
        .register_event("object:text-caret-moved")
        .await
        .is_ok()
    {
        registered_events = registry.get_registered_events().await?;
    }

    for ev in registered_events {
        println!("registered event: {:?}", ev);
    }

    // Match on all 'signal's
    dbus_proxy.add_match(r#"type='signal'"#).await?;

    // Get list of children
    let app_list = ratspie::get_accessible_children(&a11y_bus).await?;

    // Convert children in a list of proxies
    let app_text_proxies =
        ratspie::get_proxies::<TextProxy>(registry.connection(), app_list).await?;

    let mut txt_proxies_streams: Vec<PropertyStream<i32>> = Vec::new();

    for p in app_text_proxies {
        let st = p.receive_caret_offset_changed().await;
        txt_proxies_streams.push(st);
    }

    let combined_txt_streams = txt_proxies_streams.merge();

    // Drain the stream
    // while let v = &mut combined_txt_streams {
    //     println!("GOT = {:?}", v);
    // }

    combined_txt_streams
        .for_each(|v| println!("GOT = {:?}", &v.name()))
        .await;

    Ok(())
}
