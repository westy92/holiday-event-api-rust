use holiday_event_api::{
    model::{GetEventInfoRequest, GetEventsRequest, SearchRequest},
    HolidayEventApi,
};

#[tokio::main]
async fn main() {
    // Get a FREE API key from https://apilayer.com/marketplace/checkiday-api#pricing
    let client = HolidayEventApi::new("<your API key>");

    if client.is_err() {
        println!("{}", client.unwrap_err());
        return;
    }

    let client = client.unwrap();
    // Get Events for a given Date
    let events = client
        .get_events(GetEventsRequest {
            // These parameters are all optional. These are their defaults:
            date: Some("today".into()),
            adult: Some(false),
            timezone: Some("America/Chicago".into()),
        })
        .await;

    if events.is_err() {
        println!("{}", events.unwrap_err());
        return;
    }

    let events = events.unwrap();
    let event = events.events.get(0).unwrap();
    println!(
        "Today is {}! Find more information at: {}.",
        event.name, event.url
    );
    println!(
        "Rate limit remaining: {}/{} (month).",
        events.rate_limit.remaining_month, events.rate_limit.limit_month
    );

    // Get Event Information
    let event_info = client
        .get_event_info(GetEventInfoRequest {
            id: event.id.to_string(),
            // These parameters can be specified to calculate the range of event_info.event.occurrences
            start: None, // Some(2020),
            end: None,   // Some(2030),
        })
        .await;

    if event_info.is_err() {
        println!("{}", event_info.unwrap_err());
        return;
    }

    let event_info = event_info.unwrap();

    println!("The Event's hashtags are {:?}.", event_info.event.hashtags);

    // Search for Events
    let query = "pizza day";
    let search = client
        .search(SearchRequest {
            query: query.into(),
            // These parameters are the defaults but can be specified:
            adult: None, // Some(true),
        })
        .await;

    if search.is_err() {
        println!("{}", search.unwrap_err());
        return;
    }

    let search = search.unwrap();
    println!(
        "Found {} events, including {}, that match the query \"{}\".",
        search.events.len(),
        search.events.first().unwrap().name,
        query
    )
}
