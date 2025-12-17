use std::{borrow::Cow, fmt::Debug, hash::Hash};
use tracing as trc;

use serenity::{all::{CommandInteraction, CommandOptionType, CommandType}, builder::{CreateCommand, CreateCommandOption}, model::Permissions};
use strum::{EnumCount, IntoEnumIterator};

use crate::discord::ExecutionContext;

pub trait DiscordCommandDescriptor: Debug + Clone + Copy + PartialEq + Eq + Hash + EnumCount + IntoEnumIterator + Send + Sync + 'static {
    type Args<'a>: DiscordCommandArgs + 'a;

    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn options(&self) -> Vec<RawCommandOptionEntry>;
    fn parse<'a>(cmd: &'a CommandInteraction) -> Result<Self::Args<'a>, RequestError>;
}

#[derive(Debug)]
pub enum RawCommandOptionEntry {
    Integer {
        name: &'static str,
        description: &'static str,
        required: bool,
    },
    Number {
        name: &'static str,
        description: &'static str,
        required: bool,
    },
    Boolean {
        name: &'static str,
        description: &'static str,
        required: bool,
    },
    String {
        name: &'static str,
        description: &'static str,
        required: bool,
    },
    User {
        name: &'static str,
        description: &'static str,
        required: bool,
    },
    Channel {
        name: &'static str,
        description: &'static str,
        required: bool,
    },
    Attachment {
        name: &'static str,
        description: &'static str,
        required: bool,
    },
    StringSelect {
        name: &'static str,
        description: &'static str,
        // (name, value)
        choices: Vec<(&'static str, &'static str)>,
        required: bool,
    },
}

impl RawCommandOptionEntry {
    fn kind(&self) -> CommandOptionType {
        match self {
            Self::Integer { .. } => CommandOptionType::Integer,
            Self::Number { .. } => CommandOptionType::Number,
            Self::Boolean { .. } => CommandOptionType::Boolean,
            Self::String { .. } => CommandOptionType::String,
            Self::User { .. } => CommandOptionType::User,
            Self::Channel { .. } => CommandOptionType::Channel,
            Self::Attachment { .. } => CommandOptionType::Attachment,
            Self::StringSelect { .. } => CommandOptionType::String,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Integer { name, .. } => name,
            Self::Number { name, .. } => name,
            Self::Boolean { name, .. } => name,
            Self::String { name, .. } => name,
            Self::User { name, .. } => name,
            Self::Channel { name, .. } => name,
            Self::Attachment { name, .. } => name,
            Self::StringSelect { name, .. } => name,
        }
    }

    fn required(&self) -> bool {
        *match self {
            Self::Integer { required, .. } => required,
            Self::Number { required, .. } => required,
            Self::Boolean { required, .. } => required,
            Self::String { required, .. } => required,
            Self::User { required, .. } => required,
            Self::Channel { required, .. } => required,
            Self::Attachment { required, .. } => required,
            Self::StringSelect { required, .. } => required,
        }
    }

    fn description(&self) -> &'static str {
        match self {
            Self::Integer { description, .. } => description,
            Self::Number { description, .. } => description,
            Self::Boolean { description, .. } => description,
            Self::String { description, .. } => description,
            Self::User { description, .. } => description,
            Self::Channel { description, .. } => description,
            Self::Attachment { description, .. } => description,
            Self::StringSelect { description, .. } => description,
        }
    }

    fn to_option(&self) -> CreateCommandOption {
        let mut builder = CreateCommandOption::new(self.kind(), self.name(), self.description());
        builder = builder.required(self.required());
        match self {
            Self::Integer { .. } => {},
            Self::Number { .. } => {},
            Self::Boolean { .. } => {},
            Self::String { .. } => {},
            Self::User { .. } => {},
            Self::Channel { .. } => {},
            Self::Attachment { .. } => {},
            Self::StringSelect { choices, .. } => {
                for (name, value) in choices {
                    builder = builder.add_string_choice(*name, *value);
                }
            },
        }
        builder
    }
}

#[derive(Clone)]
pub struct CommandTreeIntermediate<RequestKind> {
    pub name: &'static str,
    pub description: &'static str,
    pub children: Vec<RequestKind>,
}

#[derive(Clone)]
pub enum CommandTreeTop<RequestKind> {
    Complex {
        name: &'static str,
        description: &'static str,
        kind: CommandType,
        subcommand_groups: Vec<CommandTreeIntermediate<RequestKind>>,
        subcommands: Vec<RequestKind>,
        opt_default_perm: Option<Permissions>,
    },
    NakedChatInput(RequestKind, Option<Permissions>),
    NakedUser(RequestKind, Option<Permissions>),
    MessageContextMenu(RequestKind, Option<Permissions>),
    GlobalMessageContextMenu(RequestKind, Option<Permissions>),
}

impl <R: DiscordCommandDescriptor> CommandTreeTop<R> {
    pub fn into_discord_command(self) -> CreateCommand {
        match self {
            Self::Complex { name, description, kind, subcommands, subcommand_groups, opt_default_perm } => {
                let mut top_level = CreateCommand::new(name).description(description).kind(kind);
                if let Some(perm) = opt_default_perm {
                    top_level = top_level.default_member_permissions(perm);
                }

                let subcommand_iter = subcommands.into_iter().map(|rk| {
                    let mut subcommand = CreateCommandOption::new(CommandOptionType::SubCommand, rk.name(), rk.description());
                    let options = rk.options();
                    for option in options {
                        subcommand = subcommand.add_sub_option(option.to_option());
                    }
                    subcommand
                });
                let subcommand_group_iter = subcommand_groups.into_iter().map(|cti| {
                    let mut subcommand_group = CreateCommandOption::new(CommandOptionType::SubCommandGroup, cti.name, cti.description);
                    for child in cti.children {
                        let mut subcommand = CreateCommandOption::new(CommandOptionType::SubCommand, child.name(), child.description());
                        let options = child.options();
                        for option in options {
                            subcommand = subcommand.add_sub_option(option.to_option());
                        }
                        subcommand_group = subcommand_group.add_sub_option(subcommand);
                    }

                    subcommand_group
                });

                let child_members: Vec<_> = subcommand_iter.chain(subcommand_group_iter).collect();
                if !child_members.is_empty() {
                    top_level = top_level.set_options(child_members);
                }

                top_level
            },
            Self::NakedChatInput(cmd, opt_default_perm) => {
                let mut builder = CreateCommand::new(cmd.name()).description(cmd.description()).kind(CommandType::ChatInput);
                if let Some(perm) = opt_default_perm {
                    builder = builder.default_member_permissions(perm);
                }
                let options = cmd.options();
                if !options.is_empty() {
                    builder = builder.set_options(options.into_iter().map(|rcoe| {
                        rcoe.to_option()
                    }).collect());
                }
                builder
            },
            Self::NakedUser(cmd, opt_default_perm) => {
                let mut builder = CreateCommand::new(cmd.name()).kind(CommandType::User);
                if let Some(perm) = opt_default_perm {
                    builder = builder.default_member_permissions(perm);
                }
                let options = cmd.options();
                if !options.is_empty() {
                    builder = builder.set_options(options.into_iter().map(|rcoe| {
                        rcoe.to_option()
                    }).collect());
                }
                builder
            },
            Self::MessageContextMenu(cmd, opt_default_perm) => {
                let mut builder = CreateCommand::new(cmd.name()).kind(CommandType::Message);
                if let Some(perm) = opt_default_perm {
                    builder = builder.default_member_permissions(perm);
                }
                let options = cmd.options();
                if !options.is_empty() {
                    builder = builder.set_options(options.into_iter().map(|rcoe| {
                        rcoe.to_option()
                    }).collect());
                }
                builder
            },
            Self::GlobalMessageContextMenu(cmd, opt_default_perm) => {
                let mut builder = CreateCommand::new(cmd.name()).kind(CommandType::Message);
                if let Some(perm) = opt_default_perm {
                    builder = builder.default_member_permissions(perm);
                }
                let options = cmd.options();
                if !options.is_empty() {
                    builder = builder.set_options(options.into_iter().map(|rcoe| {
                        rcoe.to_option()
                    }).collect());
                }
                builder
            },
        }
    }

    pub fn is_global(&self) -> bool {
        match self {
            Self::Complex { .. } | Self::NakedChatInput(..) | Self::NakedUser(..) | Self::MessageContextMenu(..) => {
                false
            },
            Self::GlobalMessageContextMenu(..) => true,
        }
    }
}

pub trait DiscordCommandArgs: Debug + Sized + Send {
    fn execute(self, ctx: &ExecutionContext<'_>) -> impl std::future::Future<Output = Result<(), RequestError>> + Send;
}

#[derive(Debug)]
pub struct Request<'a, RequestKind: DiscordCommandDescriptor> {
    pub args: RequestKind::Args<'a>,
}

#[derive(Debug)]
pub enum RequestError {
    User(Cow<'static, str>),
    Internal(Cow<'static, str>),
}

impl RequestError {
    pub async fn report(self, ctx: &ExecutionContext<'_>) -> Result<(), RequestError> {
        match self {
            Self::User(reason) => {
                trc::warn!("REQ-ERR-USER reason={}", reason);
                ctx.reply_restricted(reason.to_string()).await
            },
            Self::Internal(reason) => {
                trc::error!("REQ-ERR-INTERNAL reason={}", reason);
                ctx.reply_restricted("Something broke! Please contact a mod for help.".to_owned()).await
            },
        }
    }
}

impl <'a, RequestKind: DiscordCommandDescriptor> Request<'a, RequestKind> {
    pub fn parse(cmd: &'a CommandInteraction) -> Result<Self, RequestError> {
        Ok(Request {
            args: RequestKind::parse(cmd)?,
        })
    }

    pub async fn execute(self, ctx: &ExecutionContext<'_>) -> Result<(), RequestError> {
        self.args.execute(ctx).await
    }
}

#[cfg(any(feature = "test-utils", test))]
pub mod test_utils {
    use strum::IntoEnumIterator;

    use crate::cmd::DiscordCommandDescriptor;

    pub fn test_command_description_lengths<RK: IntoEnumIterator + DiscordCommandDescriptor>() {
        for c in RK::iter() {
            assert!(c.description().len() < 100, "{c:?}");
            for o in c.options() {
                assert!(o.description().len() < 100, "{c:?}, {o:?}")
            }
        }
    }

}

// TODO convert this into an importable test suite
#[cfg(test)]
pub mod test {
    use std::collections::HashSet;

    use strum::{EnumCount, EnumIter};
    use tracing as trc;

    use crate::cmd::test_utils::test_command_description_lengths;

    use super::{CommandTreeTop, RawCommandOptionEntry};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumCount, EnumIter)]
    pub enum TestRequestKind {
        Ping,
    }

    impl super::DiscordCommandDescriptor for TestRequestKind {
        type Args<'a> = TestRequestArgs;

        fn name(&self) -> &'static str {
            match self {
                TestRequestKind::Ping => {
                    "ping"
                },
            }
        }

        fn description(&self) -> &'static str {
            match self {
                TestRequestKind::Ping => {
                    "Ping!"
                },
            }
        }

        fn options(&self) -> Vec<RawCommandOptionEntry> {
            match self {
                TestRequestKind::Ping => {
                    vec![]
                },
            }
        }

        fn parse<'a>(cmd: &'a serenity::all::CommandInteraction) -> Result<Self::Args<'a>, super::RequestError> {
            match cmd.data.name.as_str() {
                "ping" => {
                    Ok(TestRequestArgs::Ping)
                },
                _ => {
                    trc::error!("Unknown command {:?} received", cmd);
                    Err(super::RequestError::Internal("Unknown command.".into()))
                },
            }
        }
    }

    #[derive(Debug)]
    pub enum TestRequestArgs {
        Ping,
    }

    impl super::DiscordCommandArgs for TestRequestArgs {
        async fn execute(self, _ctx: &crate::discord::ExecutionContext<'_>) -> Result<(), super::RequestError> {
            Ok(())
        }
    }

    pub fn generate_command_descriptions() -> Vec<CommandTreeTop<TestRequestKind>> {
        vec![
            CommandTreeTop::NakedChatInput(TestRequestKind::Ping, None),
        ]
    }

    fn iter_tree(tree: &CommandTreeTop<TestRequestKind>, set: &mut HashSet<TestRequestKind>) {
        match tree {
            CommandTreeTop::Complex { subcommand_groups, subcommands, .. } => {
                for c in subcommand_groups.iter().flat_map(|g| g.children.iter()).chain(subcommands.iter()) {
                    assert!(!set.contains(c));
                    set.insert(*c);
                }
            },
            CommandTreeTop::GlobalMessageContextMenu(cmd, _) | CommandTreeTop::NakedUser(cmd, _) | CommandTreeTop::NakedChatInput(cmd, _) | CommandTreeTop::MessageContextMenu(cmd, _) => {
                assert!(!set.contains(cmd));
                set.insert(*cmd);
            },
        }
    }

    #[test]
    fn all_commands_accounted_for() {
        let mut found_commands = HashSet::new();
        let command_tree = generate_command_descriptions();
        command_tree.iter().for_each(|ctt| iter_tree(ctt, &mut found_commands));
        assert_eq!(found_commands.len(), TestRequestKind::COUNT);
    }

    #[test]
    fn description_not_too_long() {
        test_command_description_lengths::<TestRequestKind>();
    }
}
