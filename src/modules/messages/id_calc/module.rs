use std::cmp::{max, min};
use crate::framework::module::BotModule;
use async_trait::async_trait;
use serenity::all::*;
use crate::modules::messages::id_calc::ship_render;
use crate::modules::messages::id_calc::ship_render::{get_avatar_bytes, ship_image};

pub struct IDCalcModule;

impl IDCalcModule {
    fn get_iq(&self, id: u64) -> u16 {
        (300. * (id.wrapping_mul(0x517cc1b727220a95) as f64 / u64::MAX as f64).powi(6)) as u16
    }

    fn get_percent(&self, id: u64) -> u8 {
        (id.wrapping_mul(0x517cc1b727220a95) % 101) as u8
    }
}

#[async_trait]
impl BotModule for IDCalcModule {
    fn name(&self) -> &'static str {
        "id calc"
    }

    fn slash_commands(&self) -> Vec<CreateCommand> {
        vec![
            CreateCommand::new("iq")
                .description("Calculate IQ.")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::User,
                        "user",
                        "Whose IQ do you want to know?",
                    ).required(false),
                ),
            CreateCommand::new("ship")
                .description("Check how compatible you are with someone!")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::User,
                        "user2",
                        "The one you want to check.",
                    ).required(true),
                )
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::User,
                        "user1",
                        "The one you want to check.",
                    ).required(false),
                ),
            CreateCommand::new("gay_metter")
                .description("How gay are you?")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::User,
                        "user",
                        "The one you want to check",
                    ).required(false),
                ),
            CreateCommand::new("islamic_metter")
                .description("What percentage of your Islamic Shariah beliefs do you adhere to?")
                .add_option(
                    CreateCommandOption::new(
                        CommandOptionType::User,
                        "user",
                        "The one you want to check.",
                    ).required(false),
                ),
        ]
    }

    async fn interaction_create(&self, ctx: &Context, interaction: &Interaction) {
        if let Interaction::Command(command) = interaction {
            let locale = command.locale.as_str();
            let id = command
                .data
                .options
                .iter()
                .find(|opt| opt.name == "user")
                .and_then(|opt| opt.value.as_user_id())
                .unwrap_or(command.user.id)
                .get();

            if command.data.name == "iq" {
                let iq = self.get_iq(id);

                let text = match id {
                    id if id == command.user.id.get() => match locale {
                        "ru" => format!("Твой IQ — `{}`", iq),
                        _ => format!("Your IQ is `{}`", iq),
                    },
                    _ => match locale {
                        "ru" => format!("<@{}> IQ —`{}`", id, iq),
                        _ => format!("<@{}> IQ is `{}`", id, iq),
                    },
                };

                let _ = command.create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(text)),
                ).await;
            } else if command.data.name == "ship" {
                let _ = command.create_response(
                    &ctx.http,
                    CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
                ).await;

                let u1_id = command
                    .data
                    .options
                    .iter()
                    .find(|opt| opt.name == "user1")
                    .and_then(|opt| opt.value.as_user_id())
                    .unwrap_or(command.user.id);
                let u2_id = command
                    .data
                    .options
                    .iter()
                    .find(|opt| opt.name == "user2")
                    .and_then(|opt| opt.value.as_user_id())
                    .unwrap();

                let p1 = self.get_percent(u1_id.get());
                let p2 = self.get_percent(u2_id.get());
                let p = 100 - (max(p1, p2) - min(p1, p2));

                // ох уж эта локализация
                let text = match (locale, u1_id == command.user.id, p) {
                    ("ru", true, 0..25) => "**Рекомендация:** найди себе другого.",
                    ("ru", true, 25..50) => "**Рекомендация:** лучше поищи другого.",
                    ("ru", true, 50..75) => "**Рекомендация:** поддерживай дружбу, но можно найти и получше :)",
                    ("ru", true, _) => "Он твой тот самый.",
                    ("ru", false, 0..25) => "Они друг другу не подходят.",
                    ("ru", false, 25..50) => "Ему/ей стоит поискать другого.",
                    ("ru", false, 50..75) => "Они подходят друг другу, но могут ещё подумать :P",
                    ("ru", false, _) => "Идеальная парочка ❤",
                    (_, true, 0..25) => "**Recommended:** find yourself another one.",
                    (_, true, 25..50) => "**Recommended:** better look for someone else.",
                    (_, true, 50..75) => "**Recommended:** keep the friendship, but you can find someone better :)",
                    (_, true, _) => "He's the one for you.",
                    (_, false, 0..25) => "They are not suitable for each other.",
                    (_, false, 25..50) => "He/She should look elsewhere.",
                    (_, false, 50..75) => "They are good together, but they might need some more thought :P",
                    (_, false, _) => "The perfect couple ❤",
                };

                let a1_bytes = get_avatar_bytes(u1_id.get()).await.unwrap();
                let a2_bytes = get_avatar_bytes(u2_id.get()).await.unwrap();
                let image = ship_image(a1_bytes, a2_bytes, p).unwrap();

                let _ = command.edit_response(
                    &ctx.http,
                    EditInteractionResponse::new()
                        .content(text)
                        .attachments(EditAttachments::new().add(CreateAttachment::bytes(image, "ship.png"))),
                    ).await;
            } else if command.data.name == "gay_metter" {
                let p = 100 - self.get_percent(id);

                let text = match (id == command.user.id.get(), p) {
                    (true, 0..25) => format!("You are gay on `{p}%` 😭"),
                    (true, 25..50) => format!("You are gay on `{p}%` 💅"),
                    (true, 50..75) => format!("You are gay on `{p}%` 👨‍❤️‍💋‍👨"),
                    (true, _) => format!("You are gay on **{p}%** 🏳️‍🌈🏳️‍🌈🏳️‍🌈🏳️‍🌈🏳️‍🌈"),
                    (false, 0..25) => format!("<@{id}> gay on `{p}%` 😭"),
                    (false, 25..50) => format!("<@{id}> gay on `{p}%` 💅"),
                    (false, 50..75) => format!("<@{id}> gay on `{p}%` 👨‍❤️‍💋‍👨"),
                    (false, _) => format!("<@{id}> gay on **{p}%** 🏳️‍🌈🏳️‍🌈🏳️‍🌈🏳️‍🌈🏳️‍🌈"),
                };

                let _ = command.create_response(
                    &ctx.http,
                    CreateInteractionResponse::Message(CreateInteractionResponseMessage::new().content(text)),
                ).await;
            }
        }
    }

    async fn message(&self, _ctx: &Context, _msg: &Message) {
        // TODO
    }
}
