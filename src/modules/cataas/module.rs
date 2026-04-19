use crate::framework::module::BotModule;
use crate::modules::cataas::cataas_api::fetch;
use async_trait::async_trait;
use serenity::all::*;

pub struct CatsModule;

#[async_trait]
impl BotModule for CatsModule {
    fn name(&self) -> &'static str {
        "cats"
    }

    fn slash_commands(&self) -> Vec<CreateCommand> {
        vec![CreateCommand::new(self.name()).description("Cats :)")]
    }

    async fn interaction_create(&self, ctx: &Context, interaction: &Interaction) {
        let cat_url = fetch().await.unwrap_or_else(
            |_| "https://upload.wikimedia.org/wikipedia/commons/5/50/2007-09-20_Linux_2.6_Kernel_Panic.png".to_string()
        );

        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(cat_url)
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new(format!("{}_more", self.name()))
                        .label("More!")
                        .style(ButtonStyle::Primary),
                ])]),
        );

        match interaction {
            Interaction::Command(cmd) => {
                if cmd.data.name == self.name() {
                    let _ = cmd.create_response(&ctx.http, response).await;
                }
            }
            Interaction::Component(comp) => {
                if comp.data.custom_id == format!("{}_more", self.name()) {
                    let _ = comp.create_response(&ctx.http, response).await;
                }
            }
            _ => {}
        }
    }
}
