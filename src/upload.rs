use crate::{content_type, Context, Error};
use poise::CreateReply;
use poise::serenity_prelude::futures;
use serenity::all::{CreateEmbed, Message};
use crate::upload_request::UploadRequest;
use crate::upload_response::UploadResponse;

static PASTES_DEV: &str = "https://api.pastes.dev/post";
static PASTEBOOK_DEV: &str = "https://api.pastebook.dev/upload";

static PASTES_DEV_EXPIRE_TIME: i64 = 60 * 60 * 24 * 90; // 90 days
static PASTEBOOK_DEV_EXPIRE_TIME: i64 = 60 * 60 * 24 * 30; // 30 days

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

    let tasks = attachments.into_iter().map(|attachment| {
        let msg = msg.clone();

        async move {
            let content = fetch_attachment_content(attachment.url.clone()).await?;
            let human_readable_size = format_bytes(content.len());
            let string_content = String::from_utf8_lossy(&content);

            let content_type = if attachment.filename.ends_with(".log") {
                content_type::ContentType::Log
            } else {
                content_type::ContentType::Text
            };
            
            let upload_request = UploadRequest {
                string_content: string_content.to_string(),
                filename: attachment.filename.clone(),
                content_type,
                human_readable_size: human_readable_size.clone(),
                author: msg.author.name.clone(),
            };
            
            let upload_response = match &upload_request.content_type {
                content_type::ContentType::Text => handle_text_file(&upload_request).await?,
                content_type::ContentType::Log => handle_log_file(&upload_request).await?,
            };
            
            let embed = create_upload_embed(upload_request, upload_response);
            
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

async fn handle_text_file(upload_request: &UploadRequest) -> Result<UploadResponse, Error> {
    let link = upload_to_pastebook(
        upload_request.string_content.clone(),
        &format!("{} by {}", upload_request.filename, upload_request.author),
    )
        .await?;
    
    Ok(
        UploadResponse {
            link,
            expires: (chrono::Utc::now() + chrono::Duration::seconds(PASTEBOOK_DEV_EXPIRE_TIME)).timestamp(),
        }
    )
}

async fn handle_log_file(upload_request: &UploadRequest) -> Result<UploadResponse, Error> {
    let link = upload_to_pastes_dev(upload_request.string_content.clone()).await?;
    
    Ok(UploadResponse {
        link,
        expires: (chrono::Utc::now() + chrono::Duration::seconds(PASTES_DEV_EXPIRE_TIME)).timestamp(),
    })
}

fn format_field(content: &str) -> String {
    format!("`{}`", content)
}

fn create_upload_embed(upload_request: UploadRequest, upload_response: UploadResponse) -> CreateEmbed {
    CreateEmbed::default()
        .title("File Successfully Uploaded")
        .field("File Name", format_field(&upload_request.filename), true)
        .field("File Size", format_field(&upload_request.human_readable_size), true)
        .field(
            "Expires",
            format!("<t:{}:R>", { upload_response.expires }),
            true,
        )
        .description(upload_response.link)
        .color(0x00FF00)
}

async fn fetch_attachment_content(url: String) -> Result<Vec<u8>, Error> {
    let response = reqwest::get(url).await?;
    Ok(response.bytes().await?.to_vec())
}

async fn upload_to_pastebook(content: String, file_name: &str) -> Result<String, Error> {
    let client = reqwest::Client::new();
    let response = client
        .post(PASTEBOOK_DEV)
        .header("title", file_name)
        .header("expires", (PASTEBOOK_DEV_EXPIRE_TIME * 1000).to_string())
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
        .post(PASTES_DEV)
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