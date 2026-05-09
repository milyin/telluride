use std::cmp::Ordering;
use std::convert::TryFrom;
use std::{fmt, ops::Deref, str::FromStr};

use anyhow::bail;
use chrono::{DateTime, Timelike, Utc};
use chrono_tz::Tz;

// ── Duration ─────────────────────────────────────────────────────────────────

/// Wraps `humantime::Duration` with compact `Display` formatting (`"1h30m"` instead of `"1h 30m"`).
#[derive(Clone)]
pub struct Duration(humantime::Duration);

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let total_mins = self.0.as_secs() as i64 / 60;
        let h = total_mins / 60;
        let m = total_mins % 60;
        match (h, m) {
            (0, m) => write!(f, "{}m", m),
            (h, 0) => write!(f, "{}h", h),
            (h, m) => write!(f, "{}h{}m", h, m),
        }
    }
}

impl FromStr for Duration {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        s.parse::<humantime::Duration>()
            .map(Duration)
            .map_err(|e| anyhow::anyhow!("{}", e))
    }
}

impl Deref for Duration {
    type Target = humantime::Duration;
    fn deref(&self) -> &humantime::Duration {
        &self.0
    }
}

impl From<std::time::Duration> for Duration {
    fn from(d: std::time::Duration) -> Self {
        Duration(humantime::Duration::from(d))
    }
}

// ── TelegramName ─────────────────────────────────────────────────────────────

/// A normalised Telegram username.
///
/// Stored with `'@'`, in lowercase (e.g. `"@alice"`).  Constructed via
/// [`TryFrom<&str>`], which strips a leading `'@'` if present, lower-cases
/// the result, enforces the following Telegram username rules, and then
/// prepends `'@'` before storing:
///
/// * 5–32 characters long (not counting the `'@'`).
/// * Only ASCII letters (`a-z`), digits (`0-9`), and underscores (`_`).
/// * Must start with a letter (not a digit or underscore).
/// * Cannot end with an underscore.
/// * No consecutive underscores.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, bitcode::Encode, bitcode::Decode)]
pub struct TelegramName(String);

impl FromStr for TelegramName {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let s = raw.trim_start_matches('@').to_lowercase();

        if s.len() < 5 || s.len() > 32 {
            bail!(
                "Telegram username {:?} must be 5–32 characters long (got {})",
                s,
                s.len()
            );
        }

        if !s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            bail!(
                "Telegram username {:?} contains invalid characters \
                 (only a-z, 0-9, _ allowed)",
                s
            );
        }

        let first = s.chars().next().unwrap();
        if !first.is_ascii_alphabetic() {
            bail!(
                "Telegram username {:?} must start with a letter, not {:?}",
                s,
                first
            );
        }

        if s.ends_with('_') {
            bail!("Telegram username {:?} must not end with an underscore", s);
        }

        if s.contains("__") {
            bail!(
                "Telegram username {:?} must not contain consecutive underscores",
                s
            );
        }

        Ok(TelegramName(format!("@{}", s)))
    }
}

impl fmt::Display for TelegramName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<&str> for TelegramName {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl TelegramName {
    /// Returns the username string including the leading `'@'` (e.g. `"@alice"`).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ── TimePeriod ───────────────────────────────────────────────────────────────

/// A contiguous time interval.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimePeriod {
    pub start: DateTime<Utc>,
    pub duration: chrono::Duration,
}

impl TimePeriod {
    pub fn new(start: DateTime<Utc>, duration: chrono::Duration) -> Self {
        Self { start, duration }
    }

    pub fn end(&self) -> DateTime<Utc> {
        self.start + self.duration
    }

    pub fn overlaps(&self, other: &TimePeriod) -> bool {
        self.start < other.end() && other.start < self.end()
    }

    pub fn contains(&self, other: &TimePeriod) -> bool {
        self.start <= other.start && other.end() <= self.end()
    }

    /// Remove `other` from `self`, yielding 0–2 remaining periods.
    pub fn subtract(&self, other: &TimePeriod) -> Vec<TimePeriod> {
        if !self.overlaps(other) {
            return vec![self.clone()];
        }
        let mut result = Vec::new();
        if self.start < other.start {
            result.push(TimePeriod::new(self.start, other.start - self.start));
        }
        if other.end() < self.end() {
            result.push(TimePeriod::new(other.end(), self.end() - other.end()));
        }
        result
    }

    /// Union of `self` and `other` if they overlap or are adjacent; `None` if disjoint.
    pub fn join(&self, other: &TimePeriod) -> Option<TimePeriod> {
        if self.start > other.end() || other.start > self.end() {
            return None;
        }
        let start = self.start.min(other.start);
        let end = self.end().max(other.end());
        Some(TimePeriod::new(start, end - start))
    }

    /// Returns the start instants of all full-hour-aligned slots of `duration` that fit entirely
    /// within this period.
    pub fn hour_slots(&self, duration: chrono::Duration) -> Vec<DateTime<Utc>> {
        let first_h = if self.start.minute() == 0 && self.start.second() == 0 && self.start.nanosecond() == 0 {
            self.start.hour()
        } else {
            self.start.hour() + 1
        };
        let mut slots = Vec::new();
        let mut h = first_h;
        loop {
            let Some(slot_naive) = self.start.date_naive().and_hms_opt(h, 0, 0) else {
                break;
            };
            let slot_start = slot_naive.and_utc();
            let slot_end = slot_start + duration;
            if slot_end > self.end() {
                break;
            }
            slots.push(slot_start);
            h += 1;
        }
        slots
    }
}

// ── Timezone ──────────────────────────────────────────────────────────────────

/// A validated IANA timezone.
///
/// Accepts IANA names (`Europe/Berlin`) or numeric UTC offsets (`+3`, `-5`, `0`).
/// Displays as the IANA name. Use [`to_param`](Self::to_param) for command-line serialization.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Timezone(pub Tz);

impl fmt::Display for Timezone {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.name())
    }
}

impl Timezone {
    /// Returns the form suitable for command-line parameters:
    /// numeric offset (`+3`, `-10`, `+0`) for `Etc/` zones, IANA name for all others.
    pub fn to_param(&self) -> String {
        if self.0.name().starts_with("Etc/") {
            use chrono::{Offset as _, TimeZone as _, Utc};
            let hours = Utc.timestamp_opt(0, 0).unwrap()
                .with_timezone(&self.0)
                .offset()
                .fix()
                .local_minus_utc()
                / 3600;
            format!("{:+}", hours)
        } else {
            self.0.name().to_string()
        }
    }
}

impl FromStr for Timezone {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        let s = s.trim();
        if let Ok(tz) = Tz::from_str(s) {
            return Ok(Timezone(tz));
        }
        let hours: f64 = s.parse().map_err(|_| {
            anyhow::anyhow!("unknown timezone '{}': not an IANA name or numeric offset", s)
        })?;
        if !hours.is_finite() {
            return Err(anyhow::anyhow!("invalid timezone offset '{s}'"));
        }
        let h = hours.round() as i32;
        if !(-14..=12).contains(&h) {
            return Err(anyhow::anyhow!("timezone offset {h}h is out of range [-14, +12]"));
        }
        let name = match h.cmp(&0) {
            Ordering::Equal => "Etc/GMT".to_string(),
            Ordering::Greater => format!("Etc/GMT-{h}"),
            Ordering::Less => format!("Etc/GMT+{}", -h),
        };
        Tz::from_str(&name)
            .map(Timezone)
            .map_err(|_| anyhow::anyhow!("cannot resolve offset {h}h to an Etc/GMT zone"))
    }
}

impl Deref for Timezone {
    type Target = Tz;
    fn deref(&self) -> &Tz {
        &self.0
    }
}

impl TryFrom<&str> for Timezone {
    type Error = anyhow::Error;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

// ── Currency ──────────────────────────────────────────────────────────────────

/// A validated ISO 4217 currency code.
///
/// Constructed via [`FromStr`], which validates against the `rusty_money` ISO registry.
/// Displays as the 3-letter ISO alpha code (`USD`, `EUR`, …).
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Currency(pub &'static rusty_money::iso::Currency);

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.iso_alpha_code)
    }
}

impl FromStr for Currency {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        let code = s.trim().to_uppercase();
        rusty_money::iso::find(&code)
            .map(Currency)
            .ok_or_else(|| anyhow::anyhow!("unknown ISO 4217 currency code '{s}'"))
    }
}

impl Deref for Currency {
    type Target = rusty_money::iso::Currency;
    fn deref(&self) -> &rusty_money::iso::Currency {
        self.0
    }
}

impl TryFrom<&str> for Currency {
    type Error = anyhow::Error;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

