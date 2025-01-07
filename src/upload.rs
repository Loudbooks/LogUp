use crate::{Context, Error};
use poise::CreateReply;
use poise::serenity_prelude::futures;
use serenity::all::{CreateEmbed, Message};

#[poise::command(
    context_menu_command = "Upload",
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn upload(ctx: Context<'_>, msg: Message) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    upload_context(ctx, msg, false).await
}

#[poise::command(
    context_menu_command = "Upload and Display",
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn upload_display(ctx: Context<'_>, msg: Message) -> Result<(), Error> {
    ctx.defer().await?;

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
                let mut message_builder: CreateEmbed = CreateEmbed::default();

                if attachment.filename.ends_with(".log") {
                    let result = upload_context_pastes_dev(msg.clone(), index).await?;
                    message_builder = result;
                } else if let Some(content_type) = attachment.content_type {
                    if content_type.contains("text/") {
                        let result = upload_context_pastebook(ctx, msg.clone(), index).await?;
                        message_builder = result;
                    }
                }

                Ok::<_, Error>(message_builder)
            }
        });

    let results: Vec<CreateEmbed> = futures::future::try_join_all(tasks).await?;

    if display {
        ctx.send(CreateReply {
            embeds: results,
            ..CreateReply::default()
        }
        ).await?;
    } else {
        ctx.send(CreateReply {
            embeds: results,
            ephemeral: Some(true),
            ..CreateReply::default()
        }
        ).await?;
    }

    Ok(())
}

pub async fn upload_context_pastebook(ctx: Context<'_>, msg: Message, attachment_index: usize) -> Result<CreateEmbed, Error> {
    let url = msg.attachments[attachment_index].url.clone();
    let response = reqwest::get(url).await?;
    let file_name = msg.attachments[attachment_index].filename.clone();
    let bytes = response.bytes().await?;
    let human_readable_size = format_bytes(bytes.len());
    let string_content = String::from_utf8_lossy(&bytes);

    let pastebook_link = upload_to_pastebook(string_content.to_string(), format!("{} by {}", file_name, ctx.author().display_name()).as_str()).await?;
    
    let embed = CreateEmbed::default()
        .title("File Successfully Uploaded")
        .field("File Name", file_name, true)
        .field("File Size", human_readable_size, true)
        .field("Expires", format!("<t:{}:R>", (chrono::Utc::now() + chrono::Duration::days(1)).timestamp_millis() / 1000), true)
        .description(pastebook_link)
        .color(0x00FF00);

    Ok(embed)
}

pub async fn upload_context_pastes_dev(msg: Message, attachment_index: usize) -> Result<CreateEmbed, Error> {
    let url = msg.attachments[attachment_index].url.clone();
    let response = reqwest::get(url).await?;
    let file_name = msg.attachments[attachment_index].filename.clone();
    let bytes = response.bytes().await?;
    let human_readable_size = format_bytes(bytes.len());
    let string_content = String::from_utf8_lossy(&bytes);

    let pastes_dev_link = upload_to_pastes_dev(string_content.to_string()).await?;

    let embed = CreateEmbed::default()
        .title("File Successfully Uploaded")
        .field("File Name", file_name, true)
        .field("File Size", human_readable_size, true)
        .description(pastes_dev_link)
        .color(0x00FF00);

    Ok(embed)
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