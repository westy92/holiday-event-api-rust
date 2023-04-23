use serde::Deserialize;

/// The Request struct for calling get_events.
#[derive(Debug)]
pub struct GetEventsRequest {
    /// Date to get the events for. Defaults to today.
    pub date: Option<String>,
    /// Include events that may be unsafe for viewing at work or by children. Default is false.
    pub adult: Option<bool>,
    /// IANA Time Zone for calculating dates and times. Defaults to America/Chicago.
    pub timezone: Option<String>,
}

/// The Response struct returned by get_events
#[derive(Debug, Deserialize, PartialEq)]
pub struct GetEventsResponse {
    /// Whether Adult entries can be included
    pub adult: bool,
    /// The Date string
    pub date: String,
    /// The Timezone used to calculate the Date's Events
    pub timezone: String,
    /// The Date's Events
    pub events: Vec<EventSummary>,
    /// Multi-day Events that start on Date
    pub multiday_starting: Vec<EventSummary>,
    /// Multi-day Events that are continuing their observance on Date
    pub multiday_ongoing: Vec<EventSummary>,
    #[serde(skip_deserializing)]
    rate_limit: RateLimit,
}

/// The Request struct for calling get_event_info.
#[derive(Debug)]
pub struct GetEventInfoRequest {
    /// The ID of the requested Event.
    pub id: String,
    /// The starting range of returned occurrences. Optional, defaults to 2 years prior.
    pub start: Option<i32>,
    /// The ending range of returned occurrences. Optional, defaults to 3 years in the future.
    pub end: Option<i32>,
}

/// The Response struct returned by get_event_info
#[derive(Debug, Deserialize, PartialEq)]
pub struct GetEventInfoResponse {
    /// The Event Info
    pub event: EventInfo,
    #[serde(skip_deserializing)]
    rate_limit: RateLimit,
}

/// The Request struct for calling search.
#[derive(Debug)]
pub struct SearchRequest {
    /// The search query. Must be at least 3 characters long.
    pub query: String,
    /// Include events that may be unsafe for viewing at work or by children. Default is false.
    pub adult: Option<bool>,
}

/// The Response struct returned by get_events
#[derive(Debug, Deserialize, PartialEq)]
pub struct SearchResponse {
    /// The search query
    pub query: String,
    /// Whether Adult entries can be included
    pub adult: bool,
    /// The found Events
    pub events: Vec<EventSummary>,
    #[serde(skip_deserializing)]
    rate_limit: RateLimit,
}

/// Information about an Event
#[derive(Debug, Deserialize, PartialEq)]
pub struct EventInfo {
    /// The Event Id
    pub id: String,
    /// The Event name
    pub name: String,
    /// The Event URL
    pub url: String,
    /// Whether this Event is unsafe for children or viewing at work
    pub adult: bool,
    /// The Event's Alternate Names
    pub alternate_names: Vec<AlternateName>,
    /// The Event's hashtags
    pub hashtags: Vec<String>,
    /// The Event's images
    pub image: ImageInfo,
    /// The Event's sources
    pub sources: Vec<String>,
    /// The Event's description
    pub description: RichText,
    /// How to observe the Event
    pub how_to_observe: RichText,
    /// Patterns defining when the Event is observed
    pub patterns: Vec<Pattern>,
    /// The Event Occurrences (when it occurs)
    pub occurrences: Vec<Occurrence>,
    /// The Event's founders
    pub founders: Vec<FounderInfo>,
}

/// Information about an Event's Pattern
#[derive(Debug, Deserialize, PartialEq)]
pub struct Pattern {
    /// The first year this event is observed (None implies none or unknown)
    pub first_year: Option<i32>,
    /// The last year this event is observed (None implies none or unknown)
    pub last_year: Option<i32>,
    /// A description of how this event is observed (formatted as plain text)
    pub observed: String,
    /// A description of how this event is observed (formatted as HTML)
    pub observed_html: String,
    /// A description of how this event is observed (formatted as Markdown)
    pub observed_markdown: String,
    /// For how many days this event is celebrated
    pub length: i32,
}

/// Information about an Event's Occurrence
#[derive(Debug, Deserialize, PartialEq)]
pub struct Occurrence {
    /// The date or timestamp the Event occurs
    pub date: OccurrenceDate,
    /// The length (in days) of the Event occurrence
    pub length: i32,
}

#[derive(Debug, PartialEq)]
pub enum OccurrenceDate {
    Date(String),
    Timestamp(i64),
}

impl<'de> Deserialize<'de> for OccurrenceDate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: serde::Deserializer<'de>
    {
        struct OccurrenceDateVisitor;

        impl<'de> serde::de::Visitor<'de> for OccurrenceDateVisitor {
            type Value = OccurrenceDate;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("OccurrenceDate as a number or string")
            }

            fn visit_i64<E>(self, date: i64) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                Ok(OccurrenceDate::Timestamp(date))
            }

            fn visit_u64<E>(self, date: u64) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                Ok(OccurrenceDate::Timestamp(date as i64))
            }

            fn visit_str<E>(self, date: &str) -> Result<Self::Value, E>
                where E: serde::de::Error
            {
                Ok(OccurrenceDate::Date(date.to_string()))
            }
        }

        deserializer.deserialize_any(OccurrenceDateVisitor)
    }
}

/// Information about an Event's Alternate Name
#[derive(Debug, Deserialize, PartialEq)]
pub struct AlternateName {
    /// An Event's Alternate Name
    pub name: String,
    /// The first year this Alternate Name was in effect (None implies none or unknown)
    pub first_year: Option<i32>,
    /// The last year this Alternate Name was in effect (None implies none or unknown)
    pub last_year: Option<i32>,
}

/// Formatted Text
#[derive(Debug, Deserialize, PartialEq)]
pub struct RichText {
    /// Formatted as plain text
    pub text: Option<String>,
    /// Formatted as HTML
    pub html: Option<String>,
    /// Formatted as Markdown
    pub markdown: Option<String>,
}

/// A summary of an Event
#[derive(Debug, Deserialize, PartialEq)]
pub struct EventSummary {
    /// The Event Id
    pub id: String,
    /// The Event name
    pub name: String,
    /// The Event URL
    pub url: String,
}

/// Information about an Event image
#[derive(Debug, Deserialize, PartialEq)]
pub struct ImageInfo {
    /// A small image
    pub small: String,
    /// A medium image
    pub medium: String,
    /// A large image
    pub large: String,
}

/// Information about an Event Founder
#[derive(Debug, Deserialize, PartialEq)]
pub struct FounderInfo {
    /// The Founder's name
    pub name: String,
    /// A link to the Founder
    pub url: Option<String>,
    /// The date the Event was founded
    pub date: Option<String>,
}

/// Your API plan's current Rate Limit and status. Upgrade to increase these limits.
#[derive(Debug, Deserialize, PartialEq, Default)]
pub struct RateLimit {
    /// The amount of requests allowed this month
    pub limit_month: i32,
    /// The amount of requests remaining this month
    pub remaining_month: i32,
}

pub trait RateLimited {
    fn get_rate_limit(&self) -> &RateLimit;
    fn set_rate_limit(&mut self, rate_limit: RateLimit);
}

impl RateLimited for GetEventsResponse {
    fn get_rate_limit(&self) -> &RateLimit {
        &self.rate_limit
    }
    fn set_rate_limit(&mut self, rate_limit: RateLimit) {
        self.rate_limit = rate_limit;
    }
}

impl RateLimited for GetEventInfoResponse {
    fn get_rate_limit(&self) -> &RateLimit {
        &self.rate_limit
    }
    fn set_rate_limit(&mut self, rate_limit: RateLimit) {
        self.rate_limit = rate_limit;
    }
}

impl RateLimited for SearchResponse {
    fn get_rate_limit(&self) -> &RateLimit {
        &self.rate_limit
    }
    fn set_rate_limit(&mut self, rate_limit: RateLimit) {
        self.rate_limit = rate_limit;
    }
}
