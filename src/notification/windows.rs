use std::{sync::Arc, time::SystemTime};

use anyhow::{anyhow, Context, Result};
use log::info;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use windows::{
    ApplicationModel::AppDisplayInfo,
    Foundation::{Size, TypedEventHandler},
    Storage::Streams::{DataReader, IRandomAccessStreamWithContentType},
    UI::Notifications::{
        KnownNotificationBindings,
        Management::{UserNotificationListener, UserNotificationListenerAccessStatus},
        UserNotification, UserNotificationChangedEventArgs,
        UserNotificationChangedKind,
    },
};

use super::Notification;

async fn read_logo(display_info: AppDisplayInfo) -> Result<Vec<u8>> {
    let logo_stream = display_info
        .GetLogo(Size {
            Width: 0.,
            Height: 0.,
        })
        .context("failed to get logo with size")?
        .OpenReadAsync()
        .context("failed to open for reading")?
        .await
        .context("awaiting opening for reading failed")?;

    read_stream_to_bytes(logo_stream)
        .await
        .context("failed to read stream to bytes")
}

pub async fn notif_to_message(notif: UserNotification) -> Result<Notification> {
    let app_info = notif.AppInfo()?;
    let display_info = app_info.DisplayInfo()?;
    let app_name = display_info.DisplayName()?.to_string();
    let app_icon = read_logo(display_info).await.ok();

    let toast_binding = notif
        .Notification()?
        .Visual()?
        .GetBinding(&KnownNotificationBindings::ToastGeneric()?)?;

    let text_elements = toast_binding.GetTextElements()?;
    let title = text_elements.GetAt(0)?.Text()?.to_string();
    let message = text_elements
        .into_iter()
        .skip(1)
        .map(|element| element.Text())
        .filter_map(|el| el.ok())
        .fold(String::new(), |a, b| a + &b.to_string() + "\n");

    Ok(Notification {
        app_name,
        app_icon,
        title,
        message,
        timestamp: SystemTime::now(),
    })
}

pub async fn listening_notification_handler(listener: UserNotificationListener, tx: UnboundedSender<Arc<Notification>>) -> Result<()> {
    let (new_notif_tx, mut new_notif_rx) = unbounded_channel::<u32>();
    listener
        .NotificationChanged(&TypedEventHandler::new(
            move |_sender, args: &Option<UserNotificationChangedEventArgs>| {
                if let Some(event) = args {
                    if event.ChangeKind()? == UserNotificationChangedKind::Added {
                        log::info!("handling new notification event");
                        let id = event.UserNotificationId()?;
                        if let Err(e) = new_notif_tx.send(id) {
                            log::error!("Error sending ID of new notification: {e}");
                        }
                    };
                }
                Ok(())
            },
        ))
        .context("failed to register notification change handler")?;

    while let Some(notif_id) = new_notif_rx.recv().await {
        let notif = listener
            .GetNotification(notif_id)
            .context(format!("failed to get notification {notif_id}"))?;
        let msg = notif_to_message(notif).await;
        match msg {
            Err(e) => println!("Failed to convert notification to message: {e}"),
            Ok(msg) => {
                if let Err(e) = tx.send(Arc::new(msg)) {
                    log::error!("Error sending notification to channel: {e}");
                };
            },
        };
    }

    Ok(())
}

pub async fn notification_listener(tx: UnboundedSender<Arc<Notification>>) -> Result<()> {
    let listener = UserNotificationListener::Current()
        .context("failed to initialize user notification listener")?;
    info!("Requesting notification access");
    let access_status = listener
        .RequestAccessAsync()
        .context("Notification access request failed")?
        .await
        .context("Notification access request failed")?;
    if access_status != UserNotificationListenerAccessStatus::Allowed {
        return Err(anyhow!(
            "Notification access was not granted, was instead {:?}",
            access_status
        ));
    }
    info!("Notification access granted");

    listening_notification_handler(listener, tx).await
}

async fn read_stream_to_bytes(stream: IRandomAccessStreamWithContentType) -> Result<Vec<u8>> {
    let stream_len = stream.Size()? as usize;
    let mut data = vec![0u8; stream_len];
    let reader = DataReader::CreateDataReader(&stream)?;
    reader.LoadAsync(stream_len as u32)?.await?;
    reader.ReadBytes(&mut data)?;
    reader.Close()?;
    stream.Close()?;
    Ok(data)
}
