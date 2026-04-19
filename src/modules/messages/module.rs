use crate::framework::module::BotModule;
use crate::modules::messages::*;
use async_trait::async_trait;
use futures::future::join_all;
use once_cell::sync::Lazy;
use serenity::all::*;
use std::collections::HashMap;

static MODULES: &[&dyn BotModule] = &[
    &rude_chat::module::RudeChatModule,
    &id_calc::module::IDCalcModule,
];

pub struct MessageModule;

#[async_trait]
impl BotModule for MessageModule {
    fn name(&self) -> &'static str {
        "messages"
    }

    fn slash_commands(&self) -> Vec<CreateCommand> {
        let mut commands = Vec::new();

        for module in MODULES.iter() {
            commands.extend(module.slash_commands());
        }

        commands
    }

    async fn interaction_create(&self, ctx: &Context, interaction: &Interaction) {
        let futures = MODULES
            .iter()
            .map(|module| module.interaction_create(ctx, interaction));

        join_all(futures).await;
    }

    /* main */
    async fn message(&self, ctx: &Context, msg: &Message) {
        if msg.author.bot {
            return;
        }
        let bot_id = ctx.cache.current_user().id;
        let mentions_bot = msg.mentions_user_id(bot_id);

        let mut c = msg.content.clone();
        if mentions_bot {
            c = c.replace(&format!("<@{}>", bot_id), "");
            c = c.replace(&format!("<@!{}>", bot_id), "");
        }
        c = c.to_lowercase();
        let c = c.trim();

        match c {
            c if ["quote", "цитата"].contains(&c) || c.is_empty() => {
                // TODO
            }
            c if ["iq", "айкью"].contains(&c) => {
                // TODO
            }
            _ => MODULES.get(0).unwrap().message(ctx, msg).await, // rude chat
        }
    }
}
