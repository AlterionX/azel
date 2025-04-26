use serenity::{all::CommandInteraction, builder::{CreateAllowedMentions, CreateInteractionResponse, CreateInteractionResponseMessage}, client::Context};

use crate::{cmd::RequestError, DatabaseConfiguration};

use tracing as trc;

pub struct ExecutionContext<'a> {
    pub db_cfg: &'a DatabaseConfiguration,
    pub cmd: &'a CommandInteraction,
    pub ctx: &'a Context,
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
    }
}

