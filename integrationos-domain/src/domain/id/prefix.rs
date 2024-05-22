use crate::{IntegrationOSError, InternalError};
use std::{convert::TryFrom, fmt::Display, fmt::Formatter};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "dummy", derive(fake::Dummy))]
pub enum IdPrefix {
    CommonModel,
    CommonEnum,
    Connection,
    ConnectionDefinition,
    ConnectionModelDefinition,
    ConnectionModelSchema,
    ConnectionOAuthDefinition,
    Cursor,
    EmbedToken,
    SessionId,
    Event,
    EventAccess,
    EventDependency,
    EventKey,
    Job,
    JobStage,
    LLMMessage,
    Link,
    LinkToken,
    Log,
    LogTracking,
    Pipeline,
    Platform,
    PlatformPage,
    Queue,
    Settings,
    Transaction,
    UnitTest,
}

impl Display for IdPrefix {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IdPrefix::CommonModel => write!(f, "cm"),
            IdPrefix::CommonEnum => write!(f, "ce"),
            IdPrefix::Connection => write!(f, "conn"),
            IdPrefix::ConnectionDefinition => write!(f, "conn_def"),
            IdPrefix::ConnectionModelDefinition => write!(f, "conn_mod_def"),
            IdPrefix::ConnectionModelSchema => write!(f, "conn_mod_sch"),
            IdPrefix::ConnectionOAuthDefinition => write!(f, "conn_oauth_def"),
            IdPrefix::Cursor => write!(f, "crs"),
            IdPrefix::EmbedToken => write!(f, "embed_tk"),
            IdPrefix::SessionId => write!(f, "session_id"),
            IdPrefix::Event => write!(f, "evt"),
            IdPrefix::EventAccess => write!(f, "evt_ac"),
            IdPrefix::EventDependency => write!(f, "evt_dep"),
            IdPrefix::EventKey => write!(f, "evt_k"),
            IdPrefix::Job => write!(f, "job"),
            IdPrefix::JobStage => write!(f, "job_stg"),
            IdPrefix::LLMMessage => write!(f, "llm_msg"),
            IdPrefix::Link => write!(f, "ln"),
            IdPrefix::LinkToken => write!(f, "ln_tk"),
            IdPrefix::Log => write!(f, "log"),
            IdPrefix::LogTracking => write!(f, "log_trk"),
            IdPrefix::Pipeline => write!(f, "pipe"),
            IdPrefix::Platform => write!(f, "plf"),
            IdPrefix::PlatformPage => write!(f, "plf_pg"),
            IdPrefix::Queue => write!(f, "q"),
            IdPrefix::Settings => write!(f, "st"),
            IdPrefix::Transaction => write!(f, "tx"),
            IdPrefix::UnitTest => write!(f, "ut"),
        }
    }
}

impl TryFrom<&str> for IdPrefix {
    type Error = IntegrationOSError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "cm" => Ok(IdPrefix::CommonModel),
            "ce" => Ok(IdPrefix::CommonEnum),
            "conn" => Ok(IdPrefix::Connection),
            "conn_def" => Ok(IdPrefix::ConnectionDefinition),
            "conn_mod_def" => Ok(IdPrefix::ConnectionModelDefinition),
            "conn_mod_sch" => Ok(IdPrefix::ConnectionModelSchema),
            "conn_oauth_def" => Ok(IdPrefix::ConnectionOAuthDefinition),
            "crs" => Ok(IdPrefix::Cursor),
            "embed_tk" => Ok(IdPrefix::EmbedToken),
            "session_id" => Ok(IdPrefix::SessionId),
            "evt" => Ok(IdPrefix::Event),
            "evt_ac" => Ok(IdPrefix::EventAccess),
            "evt_dep" => Ok(IdPrefix::EventDependency),
            "evt_k" => Ok(IdPrefix::EventKey),
            "job" => Ok(IdPrefix::Job),
            "job_stg" => Ok(IdPrefix::JobStage),
            "llm_msg" => Ok(IdPrefix::LLMMessage),
            "ln" => Ok(IdPrefix::Link),
            "ln_tk" => Ok(IdPrefix::LinkToken),
            "log" => Ok(IdPrefix::Log),
            "log_trk" => Ok(IdPrefix::LogTracking),
            "pipe" => Ok(IdPrefix::Pipeline),
            "plf" => Ok(IdPrefix::Platform),
            "plf_pg" => Ok(IdPrefix::PlatformPage),
            "q" => Ok(IdPrefix::Queue),
            "st" => Ok(IdPrefix::Settings),
            "tx" => Ok(IdPrefix::Transaction),
            "ut" => Ok(IdPrefix::UnitTest),
            _ => Err(InternalError::invalid_argument(
                &format!("Invalid ID prefix: {}", s),
                None,
            )),
        }
    }
}

impl From<IdPrefix> for String {
    fn from(id: IdPrefix) -> Self {
        match id {
            IdPrefix::CommonModel => "cm".to_string(),
            IdPrefix::CommonEnum => "ce".to_string(),
            IdPrefix::Connection => "conn".to_string(),
            IdPrefix::ConnectionDefinition => "conn_def".to_string(),
            IdPrefix::ConnectionModelDefinition => "conn_mod_def".to_string(),
            IdPrefix::ConnectionModelSchema => "conn_mod_sch".to_string(),
            IdPrefix::ConnectionOAuthDefinition => "conn_oauth_def".to_string(),
            IdPrefix::Cursor => "crs".to_string(),
            IdPrefix::EmbedToken => "embed_tk".to_string(),
            IdPrefix::SessionId => "session_id".to_string(),
            IdPrefix::Event => "evt".to_string(),
            IdPrefix::EventAccess => "evt_ac".to_string(),
            IdPrefix::EventDependency => "evt_dep".to_string(),
            IdPrefix::EventKey => "evt_k".to_string(),
            IdPrefix::Job => "job".to_string(),
            IdPrefix::JobStage => "job_stg".to_string(),
            IdPrefix::LLMMessage => "llm_msg".to_string(),
            IdPrefix::Link => "ln".to_string(),
            IdPrefix::LinkToken => "ln_tk".to_string(),
            IdPrefix::Log => "log".to_string(),
            IdPrefix::LogTracking => "log_trk".to_string(),
            IdPrefix::Pipeline => "pipe".to_string(),
            IdPrefix::Platform => "plf".to_string(),
            IdPrefix::PlatformPage => "plf_pg".to_string(),
            IdPrefix::Queue => "q".to_string(),
            IdPrefix::Settings => "st".to_string(),
            IdPrefix::Transaction => "tx".to_string(),
            IdPrefix::UnitTest => "ut".to_string(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_id_prefix_try_from() {
        assert!(IdPrefix::try_from("invalid").is_err());
        assert_eq!(
            IdPrefix::try_from("conn_def").unwrap(),
            IdPrefix::ConnectionDefinition
        );
        assert_eq!(
            IdPrefix::try_from("conn_mod_def").unwrap(),
            IdPrefix::ConnectionModelDefinition
        );
        assert_eq!(
            IdPrefix::try_from("conn_mod_sch").unwrap(),
            IdPrefix::ConnectionModelSchema
        );
        assert_eq!(
            IdPrefix::try_from("conn_oauth_def").unwrap(),
            IdPrefix::ConnectionOAuthDefinition
        );
        assert_eq!(
            IdPrefix::try_from("evt_dep").unwrap(),
            IdPrefix::EventDependency
        );
        assert_eq!(
            IdPrefix::try_from("log_trk").unwrap(),
            IdPrefix::LogTracking
        );
        assert_eq!(
            IdPrefix::try_from("plf_pg").unwrap(),
            IdPrefix::PlatformPage
        );
        assert_eq!(IdPrefix::try_from("cm").unwrap(), IdPrefix::CommonModel);
        assert_eq!(IdPrefix::try_from("ce").unwrap(), IdPrefix::CommonEnum);
        assert_eq!(IdPrefix::try_from("conn").unwrap(), IdPrefix::Connection);
        assert_eq!(IdPrefix::try_from("crs").unwrap(), IdPrefix::Cursor);
        assert_eq!(IdPrefix::try_from("evt").unwrap(), IdPrefix::Event);
        assert_eq!(
            IdPrefix::try_from("embed_tk").unwrap(),
            IdPrefix::EmbedToken
        );
        assert_eq!(
            IdPrefix::try_from("session_id").unwrap(),
            IdPrefix::SessionId
        );
        assert_eq!(IdPrefix::try_from("evt_ac").unwrap(), IdPrefix::EventAccess);
        assert_eq!(IdPrefix::try_from("evt_k").unwrap(), IdPrefix::EventKey);
        assert_eq!(IdPrefix::try_from("job").unwrap(), IdPrefix::Job);
        assert_eq!(IdPrefix::try_from("job_stg").unwrap(), IdPrefix::JobStage);
        assert_eq!(IdPrefix::try_from("llm_msg").unwrap(), IdPrefix::LLMMessage);
        assert_eq!(IdPrefix::try_from("ln").unwrap(), IdPrefix::Link);
        assert_eq!(IdPrefix::try_from("ln_tk").unwrap(), IdPrefix::LinkToken);
        assert_eq!(IdPrefix::try_from("log").unwrap(), IdPrefix::Log);
        assert_eq!(IdPrefix::try_from("pipe").unwrap(), IdPrefix::Pipeline);
        assert_eq!(IdPrefix::try_from("plf").unwrap(), IdPrefix::Platform);
        assert_eq!(IdPrefix::try_from("q").unwrap(), IdPrefix::Queue);
        assert_eq!(IdPrefix::try_from("st").unwrap(), IdPrefix::Settings);
        assert_eq!(IdPrefix::try_from("tx").unwrap(), IdPrefix::Transaction);
        assert_eq!(IdPrefix::try_from("ut").unwrap(), IdPrefix::UnitTest);
    }

    #[test]
    fn test_id_prefix_display() {
        assert_eq!(
            format!("{}", IdPrefix::ConnectionModelDefinition),
            "conn_mod_def"
        );
        assert_eq!(
            format!("{}", IdPrefix::ConnectionModelSchema),
            "conn_mod_sch"
        );
        assert_eq!(
            format!("{}", IdPrefix::ConnectionOAuthDefinition),
            "conn_oauth_def"
        );
        assert_eq!(format!("{}", IdPrefix::CommonModel), "cm");
        assert_eq!(format!("{}", IdPrefix::CommonEnum), "ce");
        assert_eq!(format!("{}", IdPrefix::Connection), "conn");
        assert_eq!(format!("{}", IdPrefix::ConnectionDefinition), "conn_def");
        assert_eq!(format!("{}", IdPrefix::Cursor), "crs");
        assert_eq!(format!("{}", IdPrefix::Event), "evt");
        assert_eq!(format!("{}", IdPrefix::EmbedToken), "embed_tk");
        assert_eq!(format!("{}", IdPrefix::SessionId), "session_id");
        assert_eq!(format!("{}", IdPrefix::EventAccess), "evt_ac");
        assert_eq!(format!("{}", IdPrefix::EventDependency), "evt_dep");
        assert_eq!(format!("{}", IdPrefix::EventKey), "evt_k");
        assert_eq!(format!("{}", IdPrefix::Job), "job");
        assert_eq!(format!("{}", IdPrefix::JobStage), "job_stg");
        assert_eq!(format!("{}", IdPrefix::LLMMessage), "llm_msg");
        assert_eq!(format!("{}", IdPrefix::Link), "ln");
        assert_eq!(format!("{}", IdPrefix::LinkToken), "ln_tk");
        assert_eq!(format!("{}", IdPrefix::Log), "log");
        assert_eq!(format!("{}", IdPrefix::LogTracking), "log_trk");
        assert_eq!(format!("{}", IdPrefix::Pipeline), "pipe");
        assert_eq!(format!("{}", IdPrefix::Platform), "plf");
        assert_eq!(format!("{}", IdPrefix::PlatformPage), "plf_pg");
        assert_eq!(format!("{}", IdPrefix::Queue), "q");
        assert_eq!(format!("{}", IdPrefix::Settings), "st");
        assert_eq!(format!("{}", IdPrefix::Transaction), "tx");
        assert_eq!(format!("{}", IdPrefix::UnitTest), "ut");
    }
}
