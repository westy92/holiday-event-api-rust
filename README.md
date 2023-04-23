# The Official Holiday and Event API for Rust

[![Crates.io](https://img.shields.io/crates/v/holiday_event_api)](https://crates.io/crates/holiday_event_api)
[![Build Status](https://github.com/westy92/holiday-event-api-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/westy92/holiday-event-api-rust/actions)
[![docs.rs](https://img.shields.io/docsrs/holiday_event_api)](https://docs.rs/holiday_event_api)
[![Code Coverage](https://codecov.io/gh/westy92/holiday-event-api-rust/branch/main/graph/badge.svg)](https://codecov.io/gh/westy92/holiday-event-api-rust)
[![Funding Status](https://img.shields.io/github/sponsors/westy92)](https://github.com/sponsors/westy92)

Industry-leading Holiday and Event API for Rust. Over 5,000 holidays and thousands of descriptions. Trusted by the World’s leading companies. Built by developers for developers since 2011.

## Authentication

Access to the Holiday and Event API requires an API Key. You can get for one for FREE [here](https://apilayer.com/marketplace/checkiday-api#pricing), no credit card required! Note that free plans are limited. To access more data and have more requests, a paid plan is required.

## Installation

```console
cargo add holiday_event_api
```

## Example

```rust
use holiday_event_api::{HolidayEventApi, model::{GetEventsRequest, RateLimited, GetEventInfoRequest, SearchRequest}};

#[tokio::main]
async fn main() {
    // Get a FREE API key from https://apilayer.com/marketplace/checkiday-api#pricing
    let client = HolidayEventApi::new("<your API key>".into(), None);

    if client.is_err() {
        println!("{}", client.unwrap_err());
        return;
    }

    let client = client.unwrap();
    // Get Events for a given Date
    let events = client.get_events(GetEventsRequest {
        // These parameters are all optional. These are their defaults:
        date: Some("today".into()),
        adult: Some(false),
        timezone: Some("America/Chicago".into()),
    }).await;

    if events.is_err() {
        println!("{}", events.unwrap_err());
        return;
    }

    let events = events.unwrap();
    let event = events.events.get(0).unwrap();
    println!("Today is {}! Find more information at: {}.", event.name, event.url);
    let rate_limit = events.get_rate_limit();
    println!("Rate limit remaining: {}/{} (month).\n", rate_limit.remaining_month, rate_limit.limit_month);

    // Get Event Information
    let event_info = client.get_event_info(GetEventInfoRequest {
        id: event.id.to_string(),
        // These parameters can be specified to calculate the range of event_info.event.occurrences
        start: None, // Some(2020),
        end: None, // Some(2030),
    }).await;

    if event_info.is_err() {
        println!("{}", event_info.unwrap_err());
        return;
    }

    let event_info = event_info.unwrap();

    println!("The Event's hashtags are {:?}.", event_info.event.hashtags);

    // Search for Events
    let query = "pizza day";
    let search = client.search(SearchRequest {
        query: query.into(),
        // These parameters are the defaults but can be specified:
        adult: None, // Some(true),
    }).await;

    if search.is_err() {
        println!("{}", search.unwrap_err());
        return;
    }

    let search = search.unwrap();
    println!("Found {} events, including {}, that match the query \"{}\".", search.events.len(), search.events.first().unwrap().name, query)
}
```
