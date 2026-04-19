use serenity::all::*;
use std::sync::Arc;

mod framework;
mod handler;
mod modules;

use framework::registry::ModuleRegistry;
use handler::Handler;
use modules::*;

pub fn get_discord_token() -> String {
    std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN should be set")
}

pub fn get_laozhang_token() -> String {
    std::env::var("LAOZHANG_TOKEN").expect("LAOZHANG_TOKEN should be set")
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let intents =
        GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILDS;

    let mut registry = ModuleRegistry::new();

    registry.register(cataas::module::CatsModule);
    registry.register(messages::module::MessageModule);

    let registry = Arc::new(registry);

    let handler = Handler {
        modules: registry.clone(),
    };

    let mut client = Client::builder(get_discord_token(), intents)
        .event_handler(handler)
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why);
    }
}
