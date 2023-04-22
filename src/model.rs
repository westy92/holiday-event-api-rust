use serde::Deserialize;

// TODO docs

#[derive(Debug)]
pub struct GetEventsRequest {
    pub date: Option<String>,
    pub adult: Option<bool>,
    pub timezone: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct GetEventsResponse {
    pub adult: bool,
    pub date: String,
    pub timezone: String,
    #[serde(skip_deserializing)]
    rate_limit: RateLimit,
    // TODO
}

#[derive(Debug, Deserialize, PartialEq, Default)]
pub struct RateLimit {
    pub limit_month: i32,
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