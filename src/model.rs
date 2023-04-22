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
