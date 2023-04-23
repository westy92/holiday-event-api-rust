pub mod model;

use std::{collections::HashMap, time::Duration};

use reqwest::{header::{self, HeaderValue}, Client, Url};

#[derive(Debug)]
pub struct HolidayEventApi {
    client: Client,
    base_url: Url,
}

static APP_USER_AGENT: &str = concat!(
    "HolidayApiRust/",
    env!("CARGO_PKG_VERSION"),
);

impl HolidayEventApi {
    pub fn new(api_key: String, base_url: Option<String>) -> Result<Self, String> {
        if api_key.is_empty() {
            return Err("Please provide a valid API key. Get one at https://apilayer.com/marketplace/checkiday-api#pricing.".into());
        }
        // TODO expose and test more errors
        let mut headers = header::HeaderMap::new();
        headers.insert("apikey", header::HeaderValue::from_str(&api_key.as_str()).unwrap());
        let rustc = rustc_version_runtime::version();
        headers.insert("X-Platform-Version", header::HeaderValue::from_str(&rustc.to_string()).unwrap());

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent(APP_USER_AGENT)
            .timeout(Duration::from_secs(10))
            .build().unwrap();

        let base_url = Url::parse(base_url.unwrap_or("https://api.apilayer.com/checkiday/".to_string()).as_str()).unwrap();

        Ok(Self {
            client,
            base_url,
        })
    }

    /// Gets the Events for the provided Date
    pub async fn get_events(&self, request: model::GetEventsRequest) -> Result<model::GetEventsResponse, String> {
        let mut params: HashMap<String, String> = HashMap::from([
            ("adult".into(), request.adult.unwrap_or(false).to_string())]);

        if let Some(tz) = request.timezone {
            params.insert("timezone".into(), tz);
        }

        if let Some(date) = request.date {
            params.insert("date".into(), date);
        }

        self.request("events".into(), params).await
    }

    /// Gets the Event Info for the provided Event
    pub async fn get_event_info(&self, request: model::GetEventInfoRequest) -> Result<model::GetEventInfoResponse, String> {
        if request.id.is_empty() {
            return Err("Event id is required.".into());
        }

        let mut params: HashMap<String, String> = HashMap::from([("id".into(), request.id)]);

        if let Some(start) = request.start {
            params.insert("start".into(), start.to_string());
        }

        if let Some(end) = request.end {
            params.insert("end".into(), end.to_string());
        }

        self.request("event".into(), params).await
    }

    /// Searches for Events with the given criteria
    pub async fn search(&self, request: model::SearchRequest) -> Result<model::SearchResponse, String> {
        if request.query.is_empty() {
            return Err("Search query is required.".into());
        }

        let params: HashMap<String, String> = HashMap::from([
            ("query".into(), request.query),
            ("adult".into(), request.adult.unwrap_or(false).to_string()),
        ]);

        self.request("search".into(), params).await
    }

    async fn request<T>(&self, path: String, params: HashMap<String, String>) -> Result<T, String> where T: serde::de::DeserializeOwned + std::fmt::Debug + model::RateLimited {
        let mut url = self.base_url.join(&path.to_string()).unwrap();
        url.query_pairs_mut().extend_pairs(params);

        let res = self.client.get(url).send().await;
        if res.is_err() {
            let err = res.unwrap_err().to_string();
            return Err(format!("Can't process request: {err}").into());
        }
        let res = res.unwrap();
        let status = res.status();
        if !status.is_success() {
            let json = res.json::<HashMap<String, String>>().await;
            if json.is_err() || json.as_ref().unwrap().get("error").unwrap_or(&"".into()).is_empty() {
                return Err(status.canonical_reason().unwrap_or(status.as_str()).into());
            } else {
                return Err(json.unwrap().get("error").unwrap().to_owned());
            }
        }
        let headers = res.headers().to_owned();
        let json = res.json::<T>().await;
        if json.is_err() {
            let err = json.unwrap_err().to_string();
            return Err(format!("Can't parse response: {err}"));
        }
        let rate_limit = model::RateLimit {
            limit_month: headers.get("x-ratelimit-limit-month").unwrap_or(&HeaderValue::from_str("").unwrap()).to_str().unwrap_or("").parse::<i32>().unwrap_or(0),
            remaining_month: headers.get("x-ratelimit-remaining-month").unwrap_or(&HeaderValue::from_str("").unwrap()).to_str().unwrap_or("").parse::<i32>().unwrap_or(0),
        };
        let mut result = json.unwrap();
        result.set_rate_limit(rate_limit);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};

    macro_rules! aw {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    mod new {
        use super::*;

        #[test]
        fn fails_with_missing_api_key() {
            let result = HolidayEventApi::new("".into(), None);
            assert_eq!(true, result.is_err());
            assert_eq!("Please provide a valid API key. Get one at https://apilayer.com/marketplace/checkiday-api#pricing.".to_string(), result.unwrap_err());
        }

        #[test]
        fn returns_a_new_client() {
            assert!(HolidayEventApi::new("abc123".into(), None).is_ok());
        }

    }

    mod common_functionality {
        use crate::model::RateLimited;

        use super::*;

        #[test]
        fn passes_along_api_key() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/events")
                .match_query(Matcher::Any)
                .match_header("apikey", "abc123")
                .with_body_from_file("testdata/getEvents-default.json")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            assert!(aw!(api.get_events(model::GetEventsRequest { date: None, adult: None, timezone: None })).is_ok());

            mock.assert();
        }

        #[test]
        fn passes_along_user_agent() {
            let mut server = Server::new();

            let app_version = env!("CARGO_PKG_VERSION");
            let mock = server.mock("GET", "/events")
                .match_query(Matcher::Any)
                .match_header("user-agent", format!("HolidayApiRust/{app_version}").as_str())
                .with_body_from_file("testdata/getEvents-default.json")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            assert!(aw!(api.get_events(model::GetEventsRequest { date: None, adult: None, timezone: None })).is_ok());

            mock.assert();
        }

        #[test]
        fn passes_along_platform_version() {
            let mut server = Server::new();

            let app_version = rustc_version_runtime::version().to_string();
            let mock = server.mock("GET", "/events")
                .match_query(Matcher::Any)
                .match_header("x-platform-version", app_version.as_str())
                .with_body_from_file("testdata/getEvents-default.json")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            assert!(aw!(api.get_events(model::GetEventsRequest { date: None, adult: None, timezone: None })).is_ok());

            mock.assert();
        }

        #[test]
        fn passes_along_error() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/events")
                .match_query(Matcher::Any)
                .with_status(401)
                .with_body("{\"error\":\"MyError!\"}")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest { date: None, adult: None, timezone: None }));

            assert_eq!("MyError!", result.unwrap_err());

            mock.assert();
        }

        #[test]
        fn server_error_500() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/events")
                .match_query(Matcher::Any)
                .with_status(500)
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest { date: None, adult: None, timezone: None }));

            assert_eq!("Internal Server Error", result.unwrap_err());

            mock.assert();
        }

        #[test]
        fn server_error_unknown() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/events")
                .match_query(Matcher::Any)
                .with_status(599)
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest { date: None, adult: None, timezone: None }));

            assert_eq!("599", result.unwrap_err());

            mock.assert();
        }

        #[test]
        fn server_error_other() {
            let fake_url = "http://localhost".to_string();
            let api = HolidayEventApi::new("abc123".into(), Some(fake_url)).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest { date: None, adult: None, timezone: None }));

            if cfg!(target_os = "macos") {
                assert_eq!("Can't process request: error sending request for url (http://localhost/events?adult=false): error trying to connect: tcp connect error: Connection refused (os error 61)", result.unwrap_err());
            } else if cfg!(target_os = "linux") {
                assert_eq!("Can't process request: error sending request for url (http://localhost/events?adult=false): error trying to connect: tcp connect error: Connection refused (os error 111)", result.unwrap_err());
            } else {
                assert_eq!("Not Found", result.unwrap_err());
            }
        }

        #[test]
        fn server_error_malformed_response() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/events")
                .match_query(Matcher::Any)
                .with_body("{")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest { date: None, adult: None, timezone: None }));

            assert_eq!("Can't parse response: error decoding response body: EOF while parsing an object at line 1 column 1", result.unwrap_err());

            mock.assert();
        }

        #[test]
        fn follows_redirects() {
            let mut server = Server::new();

            let url = server.url();
            let mock = server.mock("GET", "/events")
                .match_query(Matcher::Any)
                .with_status(302)
                .with_header("Location", format!("{url}/redirected").as_str())
                .create();

            let mock2 = server.mock("GET", "/redirected")
                .match_query(Matcher::Any)
                .with_body_from_file("testdata/getEvents-default.json")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            assert!(aw!(api.get_events(model::GetEventsRequest { date: None, adult: None, timezone: None })).is_ok());

            mock.assert();
            mock2.assert();
        }

        #[test]
        fn reports_rate_limits() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/events")
                .match_query(Matcher::Any)
                .with_header("X-RateLimit-Limit-Month", "100")
                .with_header("x-ratelimit-remaining-month", "88")
                .with_body_from_file("testdata/getEvents-default.json")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest { date: None, adult: None, timezone: None }));

            assert!(result.is_ok());
            let result = result.unwrap();
            assert_eq!(100, result.get_rate_limit().limit_month);
            assert_eq!(88, result.get_rate_limit().remaining_month);

            mock.assert();
        }
    }

    mod get_events {
        use super::*;

        #[test]
        fn fetches_with_default_parameters() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/events")
                .match_query(Matcher::Any)
                .with_body_from_file("testdata/getEvents-default.json")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest { date: None, adult: None, timezone: None }));

            assert!(result.is_ok());
            let result = result.unwrap();
            assert_eq!(false, result.adult);
            assert_eq!("America/Chicago", result.timezone);
            assert_eq!(2, result.events.len());
            assert_eq!(1, result.multiday_starting.len());
            assert_eq!(2, result.multiday_ongoing.len());
            assert_eq!(&model::EventSummary {
                id: "b80630ae75c35f34c0526173dd999cfc".into(),
                name: "Cinco de Mayo".into(),
                url: "https://www.checkiday.com/b80630ae75c35f34c0526173dd999cfc/cinco-de-mayo".into(),
            }, result.events.get(0).unwrap());

            mock.assert();
        }

        #[test]
        fn fetches_with_set_parameters() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/events")
                .match_query(Matcher::AllOf(vec![
                    Matcher::UrlEncoded("adult".into(), "true".into()),
                    Matcher::UrlEncoded("timezone".into(), "America/New_York".into()),
                    Matcher::UrlEncoded("date".into(), "7/16/1992".into()),
                ]))
                .with_body_from_file("testdata/getEvents-parameters.json")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest {
                date: Some("7/16/1992".into()), adult: Some(true), timezone: Some("America/New_York".into())
            }));

            assert!(result.is_ok());
            let result = result.unwrap();
            assert_eq!(true, result.adult);
            assert_eq!("America/New_York", result.timezone);
            assert_eq!(2, result.events.len());
            assert_eq!(0, result.multiday_starting.len());
            assert_eq!(1, result.multiday_ongoing.len());
            assert_eq!(&model::EventSummary {
                id: "6ebb6fd5e483de2fde33969a6c398472".into(),
                name: "Get to Know Your Customers Day".into(),
                url: "https://www.checkiday.com/6ebb6fd5e483de2fde33969a6c398472/get-to-know-your-customers-day".into(),
            }, result.events.get(0).unwrap());

            mock.assert();
        }
    }

    mod get_event_info {
        use super::*;

        #[test]
        fn fetches_with_default_parameters() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/event")
                .match_query(Matcher::UrlEncoded("id".into(), "f90b893ea04939d7456f30c54f68d7b4".into()))
                .with_body_from_file("testdata/getEventInfo-default.json")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            let result = aw!(api.get_event_info(model::GetEventInfoRequest { id: "f90b893ea04939d7456f30c54f68d7b4".into(), start: None, end: None }));

            assert!(result.is_ok());
            let result = result.unwrap();
            assert_eq!("f90b893ea04939d7456f30c54f68d7b4", result.event.id);
            assert_eq!(2, result.event.hashtags.len());

            mock.assert();
        }

        #[test]
        fn fetches_with_set_parameters() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/event")
                .match_query(Matcher::AllOf(vec![
                    Matcher::UrlEncoded("id".into(), "f90b893ea04939d7456f30c54f68d7b4".into()),
                    Matcher::UrlEncoded("start".into(), "2002".into()),
                    Matcher::UrlEncoded("end".into(), "2003".into()),
                ]))
                .with_body_from_file("testdata/getEventInfo-parameters.json")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            let result = aw!(api.get_event_info(model::GetEventInfoRequest {
                id: "f90b893ea04939d7456f30c54f68d7b4".into(), start: Some(2002), end: Some(2003)
            }));

            assert!(result.is_ok());
            let result = result.unwrap();
            assert_eq!(3, result.event.occurrences.len());
            assert_eq!(&model::Occurrence {
                date: model::OccurrenceDate::Date("08/08/2002".into()),
                length: 1,
            }, result.event.occurrences.get(0).unwrap());
            assert_eq!(&model::Occurrence {
                date: model::OccurrenceDate::Timestamp(1734772794),
                length: 1,
            }, result.event.occurrences.get(1).unwrap());
            assert_eq!(&model::Occurrence {
                date: model::OccurrenceDate::Timestamp(-12345),
                length: 7,
            }, result.event.occurrences.get(2).unwrap());

            mock.assert();
        }

        #[test]
        fn invalid_event() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/event")
                .match_query(Matcher::AllOf(vec![
                    Matcher::UrlEncoded("id".into(), "hi".into()),
                ]))
                .with_status(404)
                .with_body("{\"error\":\"Event not found.\"}")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            let result = aw!(api.get_event_info(model::GetEventInfoRequest {
                id: "hi".into(), start: None, end: None,
            }));

            assert!(result.is_err());
            assert_eq!("Event not found.", result.unwrap_err());

            mock.assert();
        }

        #[test]
        fn missing_id() {
            let api = HolidayEventApi::new("abc123".into(), None).unwrap();
            let result = aw!(api.get_event_info(model::GetEventInfoRequest {
                id: "".into(), start: None, end: None,
            }));

            assert!(result.is_err());
            assert_eq!("Event id is required.", result.unwrap_err());
        }
    }

    mod search {
        use super::*;

        #[test]
        fn fetches_with_default_parameters() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/search")
                .match_query(Matcher::UrlEncoded("query".into(), "zucchini".into()))
                .with_body_from_file("testdata/search-default.json")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            let result = aw!(api.search(model::SearchRequest { query: "zucchini".into(), adult: None }));

            assert!(result.is_ok());
            let result = result.unwrap();
            assert_eq!(false, result.adult);
            assert_eq!("zucchini", result.query);
            assert_eq!(3, result.events.len());
            assert_eq!(&model::EventSummary {
                id: "cc81cbd8730098456f85f69798cbc867".into(),
                name: "National Zucchini Bread Day".into(),
                url: "https://www.checkiday.com/cc81cbd8730098456f85f69798cbc867/national-zucchini-bread-day".into(),
            }, result.events.get(0).unwrap());

            mock.assert();
        }

        #[test]
        fn fetches_with_set_parameters() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/search")
                .match_query(Matcher::UrlEncoded("query".into(), "porch day".into()))
                .match_query(Matcher::UrlEncoded("adult".into(), "true".into()))
                .with_body_from_file("testdata/search-parameters.json")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            let result = aw!(api.search(model::SearchRequest { query: "porch day".into(), adult: Some(true) }));

            assert!(result.is_ok());
            let result = result.unwrap();
            assert_eq!(true, result.adult);
            assert_eq!("porch day", result.query);
            assert_eq!(1, result.events.len());
            assert_eq!(&model::EventSummary {
                id: "61363236f06e4eb8e4e14e5925c2503d".into(),
                name: "Sneak Some Zucchini Onto Your Neighbor's Porch Day".into(),
                url: "https://www.checkiday.com/61363236f06e4eb8e4e14e5925c2503d/sneak-some-zucchini-onto-your-neighbors-porch-day".into(),
            }, result.events.get(0).unwrap());

            mock.assert();
        }

        #[test]
        fn query_too_short() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/search")
                .match_query(Matcher::UrlEncoded("query".into(), "a".into()))
                .with_status(400)
                .with_body("{\"error\":\"Please enter a longer search term.\"}")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            let result = aw!(api.search(model::SearchRequest { query: "a".into(), adult: None }));

            assert!(result.is_err());
            assert_eq!("Please enter a longer search term.", result.unwrap_err());

            mock.assert();
        }

        #[test]
        fn too_many_results() {
            let mut server = Server::new();

            let mock = server.mock("GET", "/search")
                .match_query(Matcher::UrlEncoded("query".into(), "day".into()))
                .with_status(400)
                .with_body("{\"error\":\"Too many results returned. Please refine your query.\"}")
                .create();

            let api = HolidayEventApi::new("abc123".into(), Some(server.url())).unwrap();
            let result = aw!(api.search(model::SearchRequest { query: "day".into(), adult: None }));

            assert!(result.is_err());
            assert_eq!("Too many results returned. Please refine your query.", result.unwrap_err());

            mock.assert();
        }

        #[test]
        fn missing_parameters() {
            let api = HolidayEventApi::new("abc123".into(), None).unwrap();
            let result = aw!(api.search(model::SearchRequest { query: "".into(), adult: None }));

            assert!(result.is_err());
            assert_eq!("Search query is required.", result.unwrap_err());
        }
    }
}
