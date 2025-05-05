use serenity::{all::{ChannelType, CommandInteraction, CreateInteractionResponseFollowup, GuildChannel, GuildId}, builder::{CreateAllowedMentions, CreateInteractionResponse, CreateInteractionResponseMessage}, client::Context, futures::lock::Mutex};

use crate::{cmd::RequestError, DatabaseConfiguration};

use tracing as trc;

pub struct ExecutionContext<'a> {
    pub db_cfg: &'a DatabaseConfiguration,
    pub cmd: &'a CommandInteraction,
    pub ctx: &'a Context,
    pub is_first_response: Mutex<bool>,
}

pub enum MessageContent {
    Simple(String),
    SimpleRestrictedMention(String),
}

impl<'a> ExecutionContext<'a> {
    pub async fn reply(&self, content: String) -> Result<(), RequestError> {
        self.send_reply(MessageContent::Simple(content)).await
    }

    pub async fn reply_restricted(&self, content: String) -> Result<(), RequestError> {
        self.send_reply(MessageContent::SimpleRestrictedMention(content)).await
    }

    pub async fn send_reply(&self, content: MessageContent) -> Result<(), RequestError> {
        let mut is_first_response = self.is_first_response.lock().await;
        if *is_first_response {
            *is_first_response = false;

            let mut builder = CreateInteractionResponseMessage::new();

            builder = match content {
                MessageContent::Simple(s) => {
                    builder.content(s)
                },
                MessageContent::SimpleRestrictedMention(s) => {
                    builder.content(s).allowed_mentions(CreateAllowedMentions::new().users([self.cmd.user.id]))
                },
            };

            match self.cmd.create_response(&self.ctx, CreateInteractionResponse::Message(builder)).await {
                Ok(()) => Ok(()),
                Err(e) => {
                    trc::error!("SEND-FAILED err={e:?}");
                    Err(RequestError::Internal("Message failed to send.".into()))
                },
            }
        } else {
            let mut builder = CreateInteractionResponseFollowup::new();

            builder = match content {
                MessageContent::Simple(s) => {
                    builder.content(s)
                },
                MessageContent::SimpleRestrictedMention(s) => {
                    builder.content(s).allowed_mentions(CreateAllowedMentions::new().users([self.cmd.user.id]))
                },
            };

            match self.cmd.create_followup(&self.ctx, builder).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    trc::error!("SEND-FAILED err={e:?}");
                    Err(RequestError::Internal("Message failed to send.".into()))
                },
            }
        }
    }

    pub async fn defer(&self) -> Result<(), RequestError> {
        let mut is_first_response = self.is_first_response.lock().await;
        if *is_first_response {
            *is_first_response = false;
            match self.cmd.defer(&self.ctx).await {
                Ok(_) => Ok(()),
                Err(e) => {
                    trc::error!("SEND-FAILED err={e:?}");
                    Err(RequestError::Internal("Message failed to send.".into()))
                },
            }
        } else {
            Ok(())
        }
    }
}


impl ExecutionContext<'_> {
    pub async fn find_interactor_voice_channel(&self, guild_id: GuildId) -> Result<GuildChannel, RequestError> {
        let voice_channels = guild_id.channels(self.ctx).await.map_err(|_e| RequestError::Internal("channels failed to load".into()))?.into_values().filter(|ch| ch.kind == ChannelType::Voice);
        let mut located_channel = None;
        for channel in voice_channels {
            let joined_members = channel.members(self.ctx).map_err(|_e| RequestError::Internal("channel members failed to load".into()))?;
            if joined_members.iter().any(|j| j.user.id == self.cmd.user.id) {
                located_channel = Some(channel);
                break;
            }
        }
        located_channel.ok_or_else(|| RequestError::User("if no provided channel, caller must be in a voice channel".into()))
    }
}

