use anyhow::{anyhow, Result};
use freedesktop_icons::lookup;
use tokio_stream::StreamExt;
use zbus::fdo::MonitoringProxy;
use zbus::MatchRule;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::mpsc::UnboundedSender;
use zbus::{connection::Connection, MessageStream};
use zbus::message::{Message, Type};

use super::Notification;

pub async fn notification_listener(tx: UnboundedSender<Arc<Notification>>) -> Result<()> {
    let connection = Connection::session().await?;
    let monitor_proxy = MonitoringProxy::new(&connection).await?;

    let rule = MatchRule::builder()
        .msg_type(Type::MethodCall)
        .path("/org/freedesktop/Notifications")?
        .interface("org.freedesktop.Notifications")?
        .member("Notify")?
        .build();

    monitor_proxy.become_monitor(&[rule], 0).await?;

    println!("ready to listen notifications");
    let mut stream = MessageStream::from(&connection);
    loop {
        if let Some(Ok(msg)) = stream.next().await {
            match parse_notification(&msg).await {
                Ok(notification) => {
                    tx.send(Arc::new(notification))?;
                },
                Err(e) => eprintln!("parse notification error: {:?}", e),
            }
        }
    }
}

async fn parse_notification(msg: &Message) -> Result<Notification> {
    let body = msg.body();
    let body: zbus::zvariant::Structure = body.deserialize()?;
    let fields = body.fields();

    use zbus::zvariant::Value;
    let [Value::Str(app_name), _, Value::Str(icon), Value::Str(title), Value::Str(message), ..] = fields else {
        return Err(anyhow!("is not notification"));
    };

    Ok(Notification {
        app_id: app_name.to_string(),
        app_name: app_name.to_string(),
        title: title.to_string(),
        message: message.to_string(),
        icon: read_icon(icon).await,
        timestamp: SystemTime::now(),
    })
}

async fn read_icon(icon: &str) -> Option<Vec<u8>> {
    let path: PathBuf = lookup(icon)
        .with_cache()
        .find()
        .or_else(|| fs::exists(icon)
            .ok()
            .and_then(|exists| exists.then_some(PathBuf::from(icon))))?;

    tokio::fs::read(path).await.ok()
}
