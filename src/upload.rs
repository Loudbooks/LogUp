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
    process_upload(ctx, msg, false).await
}

#[poise::command(
    context_menu_command = "Upload and Display",
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn upload_display(ctx: Context<'_>, msg: Message) -> Result<(), Error> {
    ctx.defer().await?;
    process_upload(ctx, msg, true).await
}

async fn process_upload(ctx: Context<'_>, msg: Message, display: bool) -> Result<(), Error> {
    let attachments = msg.attachments.clone();

    if attachments.is_empty() {
        ctx.send(
            CreateReply::default()
                .content("No file attached.")
                .ephemeral(true),
        )
            .await?;
        return Ok(());
    }

    let tasks = attachments.into_iter().enumerate().map(|(index, attachment)| {
        let msg = msg.clone();

        async move {
            let mut embed = CreateEmbed::default();

            if attachment.filename.ends_with(".log") {
                embed = handle_log_file(msg.clone(), index).await?;
            } else if let Some(content_type) = attachment.content_type {
                if content_type.contains("text/") {
                    embed = handle_text_file(ctx, msg.clone(), index).await?;
                }
            }

            Ok::<_, Error>(embed)
        }
    });

    let results: Vec<CreateEmbed> = futures::future::try_join_all(tasks).await?;

    ctx.send(CreateReply {
        embeds: results,
        ephemeral: Some(!display),
        ..CreateReply::default()
    })
        .await?;

    Ok(())
}

async fn handle_text_file(ctx: Context<'_>, msg: Message, index: usize) -> Result<CreateEmbed, Error> {
    let attachment = &msg.attachments[index];
    let content = fetch_attachment_content(attachment.url.clone()).await?;
    let human_readable_size = format_bytes(content.len());
    let string_content = String::from_utf8_lossy(&content);

    let link = upload_to_pastebook(
        string_content.to_string(),
        &format!("{} by {}", attachment.filename, ctx.author().display_name()),
    )
        .await?;

    Ok(create_upload_embed(
        &attachment.filename,
        &human_readable_size,
        &link,
        true,
    ))
}

async fn handle_log_file(msg: Message, index: usize) -> Result<CreateEmbed, Error> {
    let attachment = &msg.attachments[index];
    let content = fetch_attachment_content(attachment.url.clone()).await?;
    let human_readable_size = format_bytes(content.len());
    let string_content = String::from_utf8_lossy(&content);

    let link = upload_to_pastes_dev(string_content.to_string()).await?;

    Ok(create_upload_embed(
        &attachment.filename,
        &human_readable_size,
        &link,
        false))
}

fn create_upload_embed(file_name: &str, size: &str, link: &str, expires: bool) -> CreateEmbed {
    if expires {
        CreateEmbed::default()
            .title("File Successfully Uploaded")
            .field("File Name", file_name, true)
            .field("File Size", size, true)
            .field(
                "Expires",
                format!("<t:{}:R>", (chrono::Utc::now() + chrono::Duration::days(1)).timestamp()),
                true,
            )
            .description(link)
            .color(0x00FF00)
    } else {
        CreateEmbed::default()
            .title("File Successfully Uploaded")
            .field("File Name", file_name, true)
            .field("File Size", size, true)
            .description(link)
            .color(0x00FF00)
    }
}

async fn fetch_attachment_content(url: String) -> Result<Vec<u8>, Error> {
    let response = reqwest::get(url).await?;
    Ok(response.bytes().await?.to_vec())
}

async fn upload_to_pastebook(content: String, file_name: &str) -> Result<String, Error> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.pastebook.dev/upload")
        .header("title", file_name)
        .header("Content-Type", "text/plain")
        .body(content)
        .send()
        .await?;

    let id = response.text().await?;
    Ok(format!("https://pastebook.dev/p/{}", id))
}

async fn upload_to_pastes_dev(content: String) -> Result<String, Error> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.pastes.dev/post")
        .header("Content-Type", "text/log")
        .body(content)
        .send()
        .await?;

    let location = response
        .headers()
        .get("Location")
        .ok_or_else(|| Error::from("Missing Location header"))?
        .to_str()?;

    Ok(format!("https://pastes.dev/{}", location))
}

fn format_bytes(bytes: usize) -> String {
    match bytes {
        b if b >= 1 << 30 => format!("{:.2} GB", b as f64 / (1 << 30) as f64),
        b if b >= 1 << 20 => format!("{:.2} MB", b as f64 / (1 << 20) as f64),
        b if b >= 1 << 10 => format!("{:.2} KB", b as f64 / (1 << 10) as f64),
        _ => format!("{} bytes", bytes),
    }
}