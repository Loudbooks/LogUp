use crate::{Context, Error};
use poise::CreateReply;
use poise::serenity_prelude::futures;
use serenity::all::Message;

#[poise::command(
    context_menu_command = "Upload",
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn upload(ctx: Context<'_>, msg: Message) -> Result<(), Error> {
    upload_context(ctx, msg, false).await
}

#[poise::command(
    context_menu_command = "Upload and Display",
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn upload_display(ctx: Context<'_>, msg: Message) -> Result<(), Error> {
    upload_context(ctx, msg, true).await
}

pub async fn upload_context(ctx: Context<'_>, msg: Message, display: bool) -> Result<(), Error> {
    let attachments = msg.attachments.clone();
    
    if attachments.is_empty() {
        ctx.send(CreateReply::default()
            .content("No file attached.")
            .ephemeral(true)
        ).await?;
        return Ok(());
    }

    let tasks = attachments
        .into_iter()
        .enumerate()
        .map(|(index, attachment)| {
            let msg = msg.clone();
            async move {
                if attachment.filename.ends_with(".log") {
                    upload_context_pastes_dev(ctx, msg.clone(), display).await
                } else if attachment.content_type.unwrap().contains("text/") {
                    upload_context_pastebook(ctx, msg.clone(), display, index).await
                } else {
                    ctx.send(CreateReply::default()
                        .content("Unsupported file type.")
                        .ephemeral(true)
                    ).await?;
                    Ok(())
                }
            }
        });

    futures::future::try_join_all(tasks).await?;

    Ok(())
}

pub async fn upload_context_pastebook(ctx: Context<'_>, msg: Message, display: bool, attachment_index: usize) -> Result<(), Error> {
    if msg.attachments.is_empty() {
        ctx.send(CreateReply::default()
            .content("No file attached.")
            .ephemeral(true)
        ).await?;
        return Ok(());
    }

    if display {
        ctx.defer().await?;
    } else {
        ctx.defer_ephemeral().await?;
    }

    if !msg.attachments.is_empty() {
        let url = msg.attachments[attachment_index].url.clone();
        let response = reqwest::get(url).await?;
        let file_name = msg.attachments[attachment_index].filename.clone();
        let bytes = response.bytes().await?;
        let human_readable_size = format_bytes(bytes.len());
        let string_content = String::from_utf8_lossy(&bytes);

        let pastebook_link = upload_to_pastebook(string_content.to_string(), format!("{} by {}", file_name, ctx.author().display_name()).as_str()).await?;

        ctx.send(CreateReply::default()
            .content(format!(
                "**File Name:** `{}`\n**File Size:** `{}`\n**Expires:** <t:{}:R>\n\n{}",
                file_name,
                human_readable_size,
                (chrono::Utc::now() + chrono::Duration::days(1)).timestamp_millis() / 1000,
                pastebook_link,
            ))
            .ephemeral(!display)
        ).await?;
    }

    Ok(())
}

pub async fn upload_context_pastes_dev(ctx: Context<'_>, msg: Message, display: bool) -> Result<(), Error> {
    if msg.attachments.is_empty() {
        ctx.send(CreateReply::default()
            .content("No file attached.")
            .ephemeral(true)
        ).await?;
        return Ok(());
    }

    if display {
        ctx.defer().await?;
    } else {
        ctx.defer_ephemeral().await?;
    }

    if !msg.attachments.is_empty() {
        let url = msg.attachments[0].url.clone();
        let response = reqwest::get(url).await?;
        let file_name = msg.attachments[0].filename.clone();
        let bytes = response.bytes().await?;
        let human_readable_size = format_bytes(bytes.len());
        let string_content = String::from_utf8_lossy(&bytes);

        let pastes_dev_link = upload_to_pastes_dev(string_content.to_string()).await?;

        ctx.send(CreateReply::default()
            .content(format!(
                "**File Name:** `{}`\n**File Size:** `{}`\n\n{}",
                file_name,
                human_readable_size,
                pastes_dev_link,
            ))
            .ephemeral(!display)
        ).await?;
    }

    Ok(())
}

async fn upload_to_pastebook(content: String, file_name: &str) -> Result<String, Error> {
    let client = reqwest::Client::new();
    let response = client.post("https://api.pastebook.dev/upload")
        .header("title", file_name)
        .header("Content-Type", "text/plain")
        .body(content)
        .send()
        .await?;

    let response = response.text().await?;
    let url = format!("https://pastebook.dev/p/{}", response);

    Ok(url)
}

async fn upload_to_pastes_dev(content: String) -> Result<String, Error> {
    let client = reqwest::Client::new();
    let response = client.post("https://api.pastes.dev/post")
        .header("Content-Type", "text/log")
        .body(content)
        .send()
        .await?;

    let response = response.headers().get("Location").unwrap().to_str().unwrap();
    let url = format!("<https://pastes.dev/{}>", response);

    Ok(url)
}

fn format_bytes(bytes: usize) -> String {
    let kb = bytes / 1024;
    let mb = kb / 1024;
    let gb = mb / 1024;

    if gb > 0 {
        format!("{} GB", gb)
    } else if mb > 0 {
        format!("{} MB", mb)
    } else if kb > 0 {
        format!("{} KB", kb)
    } else {
        format!("{} bytes", bytes)
    }
}