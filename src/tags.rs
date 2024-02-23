use crate::error::MessageParseError;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Eq)]
pub enum UserType {
    User,
    Admin,
    GlobalMod,
    Staff,
}

impl TryFrom<&str> for UserType {
    type Error = MessageParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(match value {
            "" => UserType::User,
            "admin" => UserType::Admin,
            "global_mod" => UserType::GlobalMod,
            "staff" => UserType::Staff,
            _ => return Err(MessageParseError::InvalidUserType(value.to_owned())),
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Emote {
    id: String,
    start_position: u32,
    end_position: u32,
}

#[derive(Debug)]
pub enum ParseEmoteError {
    MalformedEmoteString,
    InvalidRange,
}

impl TryFrom<&str> for Emote {
    type Error = ParseEmoteError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut parts = value.split(':');
        let id = parts
            .next()
            .ok_or(ParseEmoteError::MalformedEmoteString)?
            .to_string();
        let mut parts = parts
            .next()
            .ok_or(ParseEmoteError::MalformedEmoteString)?
            .split('-');
        let start_position = parts
            .next()
            .ok_or(ParseEmoteError::MalformedEmoteString)?
            .parse()
            .or(Err(ParseEmoteError::InvalidRange))?;
        let end_position = parts
            .next()
            .ok_or(ParseEmoteError::MalformedEmoteString)?
            .parse()
            .or(Err(ParseEmoteError::InvalidRange))?;

        Ok(Emote {
            id,
            start_position,
            end_position,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Badge {
    Admin(u32),
    Bits(u32),
    Broadcaster(u32),
    Moderator(u32),
    Subscriber(u32),
    Staff(u32),
    Turbo(u32),

    // Matches any other badge for now
    Other(u32),
}

impl TryFrom<&str> for Badge {
    type Error = MessageParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut parts = value.split('/');
        let name = parts
            .next()
            .ok_or(MessageParseError::InvalidBadge(value.to_owned()))?;
        let version = parts
            .next()
            .ok_or(MessageParseError::InvalidBadge(value.to_owned()))?
            .parse::<u32>()
            .map_err(|_| MessageParseError::InvalidBadgeVersion(value.to_owned()))?;

        Ok(match name {
            "admin" => Badge::Admin(version),
            "bits" => Badge::Bits(version),
            "broadcaster" => Badge::Broadcaster(version),
            "moderator" => Badge::Moderator(version),
            "subscriber" => Badge::Subscriber(version),
            "staff" => Badge::Staff(version),
            "turbo" => Badge::Turbo(version),
            _ => Badge::Other(version),
        })
    }
}

pub fn parse_tags(tags_component: &str) -> Result<HashMap<String, String>, MessageParseError> {
    tags_component
        .split(';')
        .map(|kv| {
            let mut parts = kv.split('=');

            Ok((
                parts
                    .next()
                    .ok_or(MessageParseError::InvalidTag(kv.to_owned()))?
                    .to_owned(),
                parts
                    .next()
                    .ok_or(MessageParseError::InvalidTag(kv.to_owned()))?
                    .to_owned(),
            ))
        })
        .collect::<Result<HashMap<_, _>, MessageParseError>>()
}

pub trait Tags {
    fn try_get_bool(&self, key: &str) -> Result<bool, MessageParseError>;
    fn try_get_int(&self, key: &str) -> Result<u32, MessageParseError>;
    fn try_get_vec_int(&self, key: &str) -> Result<Vec<u32>, MessageParseError>;

    fn try_get_badges(&self) -> Result<Vec<Badge>, MessageParseError>;
    fn try_get_emotes(&self) -> Result<Vec<Emote>, MessageParseError>;
    fn try_get_user_type(&self) -> Result<UserType, MessageParseError>;
}

impl Tags for HashMap<String, String> {
    fn try_get_bool(&self, key: &str) -> Result<bool, MessageParseError> {
        let value = self
            .get(key)
            .ok_or(MessageParseError::MissingTag(key.to_owned()))?;

        match value.as_str() {
            "1" => Ok(true),
            "0" => Ok(false),
            _ => Err(MessageParseError::InvalidBoolValue(
                key.to_owned(),
                value.to_owned(),
            )),
        }
    }

    fn try_get_int(&self, key: &str) -> Result<u32, MessageParseError> {
        let value = self
            .get(key)
            .ok_or(MessageParseError::MissingTag(key.to_owned()))?;

        value
            .parse()
            .map_err(|_| MessageParseError::InvalidIntValue(key.to_owned(), value.to_owned()))
    }

    fn try_get_vec_int(&self, key: &str) -> Result<Vec<u32>, MessageParseError> {
        let value = self
            .get(key)
            .ok_or(MessageParseError::MissingTag(key.to_owned()))?;

        value
            .split(',')
            .map(|v| v.parse::<u32>())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| MessageParseError::InvalidTag(value.to_owned()))
    }

    fn try_get_badges(&self) -> Result<Vec<Badge>, MessageParseError> {
        let value = self
            .get("badges")
            .ok_or(MessageParseError::MissingTag("badges".to_owned()))?;

        value
            .split(',')
            .filter(|v| !v.is_empty())
            .map(Badge::try_from)
            .collect::<Result<Vec<_>, _>>()
    }

    fn try_get_emotes(&self) -> Result<Vec<Emote>, MessageParseError> {
        let value = self
            .get("emotes")
            .ok_or(MessageParseError::MissingTag("emotes".to_owned()))?;

        value
            .split(',')
            .filter(|v| !v.is_empty())
            .map(Emote::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| MessageParseError::InvalidTag(value.to_owned()))
    }

    fn try_get_user_type(&self) -> Result<UserType, MessageParseError> {
        let value = self
            .get("user-type")
            .ok_or(MessageParseError::MissingTag("user-type".to_owned()))?;

        UserType::try_from(value.as_str())
            .map_err(|_| MessageParseError::InvalidTag(value.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tag() {
        let mut expected: HashMap<String, String> = HashMap::new();
        expected.insert("badge-info".to_owned(), "".to_owned());
        expected.insert("badges".to_owned(), "".to_owned());
        expected.insert("color".to_owned(), "".to_owned());
        expected.insert("display-name".to_owned(), "<user>".to_owned());
        expected.insert("emote-sets".to_owned(), "0,300374282".to_owned());
        expected.insert("user-id".to_owned(), "12345678".to_owned());
        expected.insert("user-type".to_owned(), "".to_owned());

        let actual = parse_tags(
            "badge-info=;badges=;color=;display-name=<user>;emote-sets=0,300374282;user-id=12345678;user-type="
        ).unwrap();

        for (key, value) in expected.iter() {
            assert_eq!(value, actual.get(key).unwrap())
        }
    }
}
