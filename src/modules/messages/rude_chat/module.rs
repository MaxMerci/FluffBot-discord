use crate::framework::module::BotModule;
use crate::modules::messages::rude_chat::prompts::*;
use crate::modules::messages::rude_chat::remote::*;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use serenity::all::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

pub static CHAT_SESSIONS: Lazy<Arc<Mutex<HashMap<UserId, chat::ChatSession>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

fn random_prompt() -> Option<&'static str> {
    match rand::random::<u8>() % 4 {
        0 => Some(GIRL_PROMPT),
        1 => Some(TROLL_PROMPT),
        2 => Some(CUTE_PROMPT),
        _ => Some(NORMAL_PROMPT),
    }
}

pub struct RudeChatModule;

#[async_trait]
impl BotModule for RudeChatModule {
    fn name(&self) -> &'static str {
        "chatbot"
    }

    fn slash_commands(&self) -> Vec<CreateCommand> {
        vec![
            CreateCommand::new(self.name())
                .description("Chatbot commands.")
                .add_option(CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "delete",
                    "Delete personal chat.",
                ))
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::SubCommand,
                        "request",
                        "Request to API without chat.",
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::String,
                            "content",
                            "message content",
                        )
                        .required(true),
                    ),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::SubCommand,
                        "style",
                        "Communication style.",
                    )
                    .add_sub_option(
                        CreateCommandOption::new(
                            CommandOptionType::String,
                            "mode",
                            "Choose communication style",
                        )
                        .add_string_choice("troll", "troll")
                        .add_string_choice("dumb girl", "dumb girl")
                        .add_string_choice("cute", "cute")
                        .add_string_choice("normal", "normal")
                        .required(true),
                    ),
                ),
        ]
    }

    async fn interaction_create(&self, ctx: &Context, interaction: &Interaction) {
        if let Interaction::Command(command) = interaction {
            if command.data.name != "chatbot" {
                return;
            }

            let subcommand = command.data.options.first().unwrap();

            if subcommand.name == "delete" {
                let user_id = command.user.id;

                let mut sessions = CHAT_SESSIONS.lock().await;
                sessions.remove(&user_id);

                let _ = command.create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(
                        CreateInteractionResponseMessage::new()
                            .content("Chat has been cleared.")
                            .ephemeral(true),
                    ),
                ).await;
            } else if subcommand.name == "request" {
                let _ = command.create_response(
                    &ctx.http,
                    CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
                ).await;

                let CommandDataOptionValue::SubCommand(options) = &subcommand.value else {
                    unreachable!("expected subcommand");
                };
                let content = options
                    .iter()
                    .find(|opt| opt.name == "content")
                    .and_then(|opt| opt.value.as_str())
                    .unwrap();

                let mut sessions = CHAT_SESSIONS.lock().await;
                let chat = sessions.entry(command.user.id).or_insert_with(|| {
                    let engine = engine::LlmEngine::new();
                    chat::ChatSession::new(engine, random_prompt())
                });

                match chat.send_message(content).await {
                    Ok(text) => {
                        let edit = EditInteractionResponse::new().content(text);
                        let _ = command.edit_response(&ctx.http, edit).await;
                    }
                    Err(e) => {
                        let edit = EditInteractionResponse::new().content(format!("Err: {}", e));
                        let _ = command.edit_response(&ctx.http, edit).await;
                    }
                }
            } else if subcommand.name == "style" {
                let CommandDataOptionValue::SubCommand(options) = &subcommand.value else {
                    unreachable!("expected subcommand");
                };
                let style = options
                    .iter()
                    .find(|opt| opt.name == "mode")
                    .and_then(|opt| opt.value.as_str())
                    .unwrap();

                let prompt = match style {
                    "troll" => Some(TROLL_PROMPT),
                    "dumb girl" => Some(GIRL_PROMPT),
                    "cute" => Some(CUTE_PROMPT),
                    "normal" => Some(NORMAL_PROMPT),
                    _ => Some(""),
                };

                let mut sessions = CHAT_SESSIONS.lock().await;
                let engine = engine::LlmEngine::new();
                sessions.insert(command.user.id, chat::ChatSession::new(engine, prompt));

                let _ = command
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content(format!("Communication style has been changed — `{}`", style))
                                .ephemeral(true),
                        ),
                    )
                    .await;
            }
        }
    }

    async fn message(&self, ctx: &Context, msg: &Message) {
        let bot_id = ctx.cache.current_user().id;
        let mentions_bot = msg.mentions_user_id(bot_id);
        let is_reference = msg
            .referenced_message
            .as_ref()
            .map_or(false, |m| m.author.id == bot_id);

        if mentions_bot || is_reference {
            let mut cleaned = msg.content.clone();
            if mentions_bot {
                cleaned = cleaned.replace(&format!("<@{}>", bot_id), "");
                cleaned = cleaned.replace(&format!("<@!{}>", bot_id), "");
            }
            let cleaned = cleaned.trim().to_string();

            if cleaned.is_empty() {
                return;
            }

            let http = ctx.http.clone();
            let channel_id = msg.channel_id;
            let (stop_typing_tx, mut stop_typing_rx) = tokio::sync::oneshot::channel::<()>();
            let typing_task = tokio::spawn(async move {
                loop {
                    let _ = channel_id.broadcast_typing(&http).await;
                    tokio::select! {
                        _ = sleep(Duration::from_secs(7)) => {}
                        _ = &mut stop_typing_rx => break,
                    }
                }
            });

            let response_result = {
                let mut sessions = CHAT_SESSIONS.lock().await;
                let session = sessions.entry(msg.author.id).or_insert_with(|| {
                    let engine = engine::LlmEngine::new();
                    chat::ChatSession::new(engine, random_prompt())
                });
                session.send_message(&cleaned).await
            };

            let _ = stop_typing_tx.send(());
            let _ = typing_task.await;

            match response_result {
                Ok(response_text) => {
                    if let Err(why) = msg.reply(&ctx.http, response_text).await {
                        eprintln!("Error replying: {:?}", why);
                    }
                }
                Err(e) => {
                    let _ = msg
                        .reply(&ctx.http, format!("Ошибка генерации: {:?}", e))
                        .await;
                    eprintln!("LLM Error: {:?}", e);
                }
            }
        }
    }
}
