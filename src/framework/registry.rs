use crate::framework::module::BotModule;
use serenity::all::*;
use std::sync::Arc;

pub struct ModuleRegistry {
    modules: Vec<Arc<dyn BotModule>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
        }
    }

    pub fn register<M: BotModule + 'static>(&mut self, module: M) {
        self.modules.push(Arc::new(module));
    }

    pub async fn register_commands(&self, ctx: &Context) {
        let mut commands = Vec::new();

        for module in &self.modules {
            commands.extend(module.slash_commands());
        }

        Command::set_global_commands(&ctx.http, commands)
            .await
            .expect("Failed to overwrite global commands");
    }

    pub async fn interaction_create(&self, ctx: &Context, interaction: &Interaction) {
        for module in &self.modules {
            module.interaction_create(ctx, interaction).await;
        }
    }

    pub async fn message(&self, ctx: &Context, msg: &Message) {
        for module in &self.modules {
            module.message(ctx, msg).await;
        }
    }

    pub async fn ready(&self, ctx: &Context, ready: &Ready) {
        for module in &self.modules {
            module.ready(ctx, ready).await;
        }
    }
}
