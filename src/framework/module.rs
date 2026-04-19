use async_trait::async_trait;
use serenity::all::*;

#[async_trait]
pub trait BotModule: Send + Sync {
    fn name(&self) -> &'static str;

    fn slash_commands(&self) -> Vec<CreateCommand> {
        Vec::new()
    }

    async fn interaction_create(&self, _ctx: &Context, _interaction: &Interaction) {}

    async fn message(&self, _ctx: &Context, _msg: &Message) {}

    async fn ready(&self, _ctx: &Context, _ready: &Ready) {}
}
