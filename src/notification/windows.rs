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
    
    // Check if the notification is from our own app
    if app_name == "Gate Of Notification" || app_info.AppUserModelId()?.to_string().contains("ckaznable.gateofnotification") {
        // Return an error to skip processing our own notifications
        return Err(anyhow!("Skipping our own notification"));
    }
    
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
        .NotificationChanged(&TypedEventHandler::<UserNotificationListener, UserNotificationChangedEventArgs>::new(
            move |_sender, args| {
                let args = args.as_ref();
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
            Err(e) => {
                // Only log errors that are not related to skipping our own notifications
                if !e.to_string().contains("Skipping our own notification") {
                    log::error!("Failed to convert notification to message: {e}");
                }
            },
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

/// Sends a Windows toast notification.
/// 
/// Designed to work in MSIX packaged applications without requiring an explicit AppID.
/// 
/// # Arguments
/// * `title` - The notification title
/// * `message` - The notification body text
/// * `actions` - Optional: Whether to include action buttons
pub fn send_notification(title: &str, message: &str, actions: bool) -> Result<()> {
    use windows::{
        Data::Xml::Dom::*,
        UI::Notifications::*,
        core::*,
        Foundation::*,
    };
    
    // Create notification XML content
    let toast_xml = XmlDocument::new()?;
    
    // Use empty launch attribute and silent scenario to prevent app activation
    // Avoid using protocol activationType which can show protocol error
    let xml_content = if actions {
        format!(
            r#"<toast activationType="background">
                <visual>
                    <binding template="ToastGeneric">
                        <text>{}</text>
                        <text>{}</text>
                    </binding>
                </visual>
                <actions>
                    <action content="Dismiss" arguments="dismiss" activationType="system"/>
                </actions>
                <audio silent="true"/>
            </toast>"#,
            title, message
        )
    } else {
        format!(
            r#"<toast activationType="background">
                <visual>
                    <binding template="ToastGeneric">
                        <text>{}</text>
                        <text>{}</text>
                    </binding>
                </visual>
                <audio silent="true"/>
            </toast>"#,
            title, message
        )
    };

    // Use Windows API methods
    let h_string = HSTRING::from(xml_content);
    toast_xml.LoadXml(&h_string)?;

    // Get notification manager
    let toast_notification_manager = ToastNotificationManager::GetDefault()?;
    let toast_notifier = toast_notification_manager.CreateToastNotifier()?;

    // Create notification
    let toast_notification = ToastNotification::CreateToastNotification(&toast_xml)?;
    
    // Add handler for SuppressPopup - prevent from showing in action center
    toast_notification.SetSuppressPopup(false)?;
    
    // Add handler for Activated event
    let activated_handler = TypedEventHandler::<ToastNotification, IInspectable>::new(
        |_, _| {
            // Just return OK - this prevents the app from being launched
            Ok(())
        }
    );
    toast_notification.Activated(&activated_handler)?;
    
    // Add handler for Dismissed event
    let dismissed_handler = TypedEventHandler::<ToastNotification, ToastDismissedEventArgs>::new(
        |_, _| {
            // Notification was dismissed (by user or timeout)
            Ok(())
        }
    );
    toast_notification.Dismissed(&dismissed_handler)?;
    
    // Show the notification
    toast_notifier.Show(&toast_notification)?;
    
    Ok(())
}
