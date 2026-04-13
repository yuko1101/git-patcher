use std::{fmt, str::FromStr, sync::LazyLock};

use anyhow::Context;
use git2::Signature;
use regex::Regex;

pub struct SignatureData {
    name: String,
    email: String,
    seconds: i64,
    offset_minutes: i32,
}

impl SignatureData {
    pub fn as_signature(&self) -> anyhow::Result<git2::Signature<'_>> {
        let timestamp = git2::Time::new(self.seconds, self.offset_minutes);
        Ok(git2::Signature::new(&self.name, &self.email, &timestamp)?)
    }
}

impl fmt::Display for SignatureData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} <{}> {} {}",
            self.name,
            self.email,
            self.seconds,
            format_timezone(self.offset_minutes)
        )
    }
}

const SIG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?<name>.+) <(?<email>.+)> (?<seconds>-?\d+) (?<tz>[+-]\d{4})$").unwrap()
});
impl FromStr for SignatureData {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let caps = SIG_REGEX
            .captures(s)
            .with_context(|| format!("Failed to parse signature: {}", s))?;
        let name = caps.name("name").unwrap().as_str().to_string();
        let email = caps.name("email").unwrap().as_str().to_string();
        let seconds: i64 = caps
            .name("seconds")
            .unwrap()
            .as_str()
            .parse()
            .with_context(|| format!("Failed to parse seconds in signature: {}", s))?;
        let tz_str = caps.name("tz").unwrap().as_str();
        let offset_minutes = parse_timezone(tz_str)
            .with_context(|| format!("Failed to parse timezone in signature: {}", s))?;

        Ok(Self {
            name,
            email,
            seconds,
            offset_minutes,
        })
    }
}

impl From<Signature<'_>> for SignatureData {
    fn from(sig: Signature<'_>) -> Self {
        Self {
            name: sig.name().unwrap_or("").to_string(),
            email: sig.email().unwrap_or("").to_string(),
            seconds: sig.when().seconds(),
            offset_minutes: sig.when().offset_minutes(),
        }
    }
}

fn format_timezone(offset_minutes: i32) -> String {
    let sign = if offset_minutes >= 0 { '+' } else { '-' };
    let abs_offset = offset_minutes.abs();
    let hours = abs_offset / 60;
    let minutes = abs_offset % 60;

    format!("{}{:02}{:02}", sign, hours, minutes)
}

/// WARN: this function cannot distinguish between "+0000" and "-0000", which are both valid timezone. this can lead to incorrect commit hashes.
fn parse_timezone(tz_str: &str) -> Option<i32> {
    if tz_str.len() != 5 {
        return None;
    }

    let sign = match tz_str.chars().next()? {
        '+' => 1,
        '-' => -1,
        _ => return None,
    };

    let hours: i32 = tz_str[1..3].parse().ok()?;
    let minutes: i32 = tz_str[3..5].parse().ok()?;

    Some(sign * (hours * 60 + minutes))
}
