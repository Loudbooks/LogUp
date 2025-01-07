mod upload;

use std::env;
use poise::CreateReply;
use serenity::all::GatewayIntents;
use serenity::Client;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Debug)]
pub struct Data {}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                upload::upload(),
                upload::upload_display(),
            ],
            on_error: |why| {
                println!("Error: {:?}", why);
                
                Box::pin(async move {
                    match why.ctx() {
                        None => {}
                        Some(ctx) => {
                            ctx.send(
                                CreateReply::default()
                                    .content(why.to_string())
                                    .ephemeral(true),
                            ).await.unwrap();
                        }
                    }
                })
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                
                Ok(Data {})
            })
        })
        .build();

    let client = Client::builder(&token, intents)
            .framework(framework).await;

    client.unwrap().start().await.unwrap();
}
