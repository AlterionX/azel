pub mod discord;
pub mod db;

pub mod cmd;

use cmd::{CommandTreeTop, DiscordCommandDescriptor};
use config::ConfigError;
use serenity::{all::{ClientBuilder, CommandPermissions, Interaction}, async_trait, client::{Client, EventHandler}, http::CacheHttp, model::{channel::{Message, Reaction}, event::MessageUpdateEvent, gateway::Ready, id::{ChannelId, GuildId, MessageId}}, prelude::{Context as DiscordContext, GatewayIntents}};
use tracing::{self as trc, Instrument};

use discord::ExecutionContext;

pub struct Arguments {
    cfg_path: String,
}

#[derive(serde::Deserialize)]
pub struct Configuration {
    discord: DiscordConfiguration,
    home_guild: HomeGuildConfiguration,
    database: DatabaseConfiguration,
}

#[derive(serde::Deserialize)]
pub struct DatabaseConfiguration {
    pub url: String,
}

#[derive(serde::Deserialize)]
pub struct DiscordConfiguration {
    token: String,
    application: u64,
}

#[derive(serde::Deserialize)]
pub struct HomeGuildConfiguration {
    id: u64,
}

pub struct DiscordHandler<R> {
    pub home_guild_id: GuildId,
    pub db_cfg: DatabaseConfiguration,
    pub command_descriptions: Vec<CommandTreeTop<R>>
}

impl <R> DiscordHandler<R> {
    fn show_time<TZ: chrono::TimeZone>(ui: &str, source: &str, data: impl std::fmt::Display, start: chrono::DateTime<TZ>, end: chrono::DateTime<TZ>) {
        let diff = end - start;
        let diff_ns = diff.num_nanoseconds().unwrap_or(-1);
        if diff <= chrono::Duration::seconds(1) {
            let diff_human = diff.num_milliseconds();
            trc::info!("TIMING ui={ui} {source}={data} duration={diff_ns}ns human={diff_human}ms");
        } else {
            let diff_human = diff.num_seconds();
            trc::info!("TIMING ui={ui} {source}={data} duration={diff_ns}ns human={diff_human}s");
        }
    }
}

#[async_trait]
impl <R: DiscordCommandDescriptor> EventHandler for DiscordHandler<R> {
    async fn ready(&self, ctx: DiscordContext, data_about_bot: Ready) {
        trc::info!("CMD-SETUP");
        let commands: Vec<_> = self.command_descriptions.clone().into_iter().map(|ctt| ctt.into_discord_command()).collect();
        trc::info!("CMD-SETUP-GUILD {:?}", self.home_guild_id);
        for guild in data_about_bot.guilds {
            trc::info!("CMD-SETUP-GUILD {:?}", guild.id);
            guild.id.set_commands(
                ctx.http(),
                commands.clone()
            ).await.expect("commands should have updated appropriately");
        }
        trc::info!("CMD-SETUP-CMPL");
    }

    async fn command_permissions_update(
        &self,
        _ctx: DiscordContext,
        _permission: CommandPermissions,
    ) {
        // TODO
    }

    async fn reaction_add(&self, _ctx: DiscordContext, _add_reaction: Reaction) {
        // TODO
    }

    async fn reaction_remove(&self, _ctx: DiscordContext, _removed_reaction: Reaction) {
        // TODO
    }

    async fn reaction_remove_all(&self, _ctx: DiscordContext, _channel_id: ChannelId, _removed_from_message_id: MessageId) {
        // TODO
    }

    async fn message(
        &self,
        _ctx: DiscordContext,
        _new_message: Message,
    ) {
        // TODO
    }

    async fn message_delete(
        &self,
        _ctx: DiscordContext,
        _channel_id: ChannelId,
        _deleted_message_id: MessageId,
        _guild_id: Option<GuildId>,
    ) {
        // TODO Chain delete related messages.
    }

    async fn message_delete_bulk(
        &self,
        _ctx: DiscordContext,
        _channel_id: ChannelId,
        _deleted_message_id: Vec<MessageId>,
        _guild_id: Option<GuildId>,
    ) {
        // TODO Chain delete related messages.
    }

    async fn message_update(
        &self,
        _ctx: DiscordContext,
        _old_if_available: Option<Message>,
        _new: Option<Message>,
        _event: MessageUpdateEvent,
    ) {
        // TODO Update informational messages.
    }

    async fn interaction_create(
        &self,
        dctx: DiscordContext,
        interaction: Interaction,
    ) {
        let interaction_id = interaction.id();
        let start = chrono::Utc::now();

        async move {
            let ui = match interaction {
                Interaction::Ping(_) => {
                    "discord_ping"
                },
                Interaction::Modal(_) => {
                    "discord_modal"
                },
                Interaction::Autocomplete(_autocomplete) => {
                    "discord_autocomp"
                },
                Interaction::Component(_component) => {
                    "discord_component"
                },
                Interaction::Command(command) => {
                    let ctx = ExecutionContext {
                        ctx: &dctx,
                        cmd: &command,
                        db_cfg: &self.db_cfg,
                        is_first_response: true.into(),
                    };
                    match cmd::Request::<R>::parse(&command) {
                        Ok(req) => {
                            trc::info!("REQ-EXEC req={req:?}");
                            // TODO Execute request.
                            match req.execute(&ctx).await {
                                Ok(_) => {
                                    trc::info!("REQ-CMP");
                                },
                                Err(err) => {
                                    trc::warn!("REQ-FAIL");
                                    if let Err(e) = err.report(&ctx).await {
                                        trc::error!("REQ-EXEC-ERR-REPORT-FAIL err={:?}", e);
                                    }
                                }
                            }
                        },
                        Err(err) => {
                            trc::warn!("REQ-FAIL cmd={:?}", ctx.cmd);
                            if let Err(e) = err.report(&ctx).await {
                                trc::error!("REQ-PARSE-ERR-REPORT-FAIL err={:?}", e);
                            }
                        },
                    }
                    "discord_command"
                },
                _s => {
                    panic!("wtf library");
                },
            };
            let end = chrono::Utc::now();
            Self::show_time(ui, "interaction", interaction_id, start, end);
        }.instrument(trc::info_span!("interaction", primary_id = u64::from(interaction_id))).await;
    }
}

pub struct Discord(pub Client);

pub async fn build_client<R: DiscordCommandDescriptor>(
    cfg: Configuration,
    command_descriptions: Vec<CommandTreeTop<R>>,
    builder_config_fn: impl FnOnce(ClientBuilder) -> ClientBuilder,
) -> serenity::Result<Discord> {
    let token = cfg.discord.token.as_str();
    let application_id = cfg.discord.application.into();
    let handler = DiscordHandler {
        home_guild_id: cfg.home_guild.id.into(),
        db_cfg: cfg.database,
        command_descriptions,
    };

    let intents = GatewayIntents::non_privileged();

    let builder = Client::builder(token, intents)
        .application_id(application_id)
        .event_handler(handler);
    let builder = builder_config_fn(builder);

    let client = builder.await?;

    Ok(Discord(client))
}

pub fn load_configuration(cfg_path: &str) -> Result<Configuration, ConfigError> {
    config::Config::builder()
        .add_source(config::File::with_name(cfg_path))
        .build()?
        .try_deserialize()
}

pub fn setup_default_log_and_load_configuration() -> Result<Configuration, ConfigError> {
    tracing_subscriber::fmt::init();

    trc::info!("LOG-CMPL");

    let Arguments { cfg_path } = {
        let mut args: Vec<String> = std::env::args().collect();
        if args.len() != 2 {
            trc::error!("BAD-ARGS args={:?}", args);
            panic!("bad args");
        }
        let cfg_path = args.remove(1);
        Arguments {
            cfg_path,
        }
    };


    load_configuration(cfg_path.as_str())
}

pub async fn easy_setup_and_run<R: DiscordCommandDescriptor>(command_descriptions: Vec<CommandTreeTop<R>>) {
    let cfg = setup_default_log_and_load_configuration().unwrap();

    let mut discord = build_client(cfg, command_descriptions, |b| b).await.expect("client to be built");

    trc::info!("BOOT-CMPL");

    discord.0.start().await.expect("no error");
}
