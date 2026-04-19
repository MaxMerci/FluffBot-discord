use crate::framework::registry::ModuleRegistry;
use async_trait::async_trait;
use serenity::all::*;
use std::sync::Arc;

pub struct Handler {
    pub modules: Arc<ModuleRegistry>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Logged in as {}", ready.user.name);
        self.modules.ready(&ctx, &ready).await;
        self.modules.register_commands(&ctx).await;
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        self.modules.interaction_create(&ctx, &interaction).await;
    }

    async fn message(&self, ctx: Context, msg: Message) {
        self.modules.message(&ctx, &msg).await;
    }
}
