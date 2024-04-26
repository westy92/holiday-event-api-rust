pub mod model;

use std::{collections::HashMap, time::Duration};

use reqwest::{
    header::{self, HeaderValue},
    Client, Url,
};

#[derive(Debug)]
pub struct HolidayEventApi {
    client: Client,
    base_url: Url,
}

static APP_USER_AGENT: &str = concat!("HolidayApiRust/", env!("CARGO_PKG_VERSION"));

impl HolidayEventApi {
    pub fn new(api_key: &str) -> Result<Self, String> {
        Self::new_internal(api_key, "https://api.apilayer.com/checkiday/")
    }

    pub(crate) fn new_internal(api_key: &str, base_url: &str) -> Result<Self, String> {
        let api_key_header = HeaderValue::try_from(api_key);
        if api_key.is_empty() || api_key_header.is_err() {
            return Err("Please provide a valid API key. Get one at https://apilayer.com/marketplace/checkiday-api#pricing.".into());
        }
        let mut headers = header::HeaderMap::new();
        headers.insert("apikey", api_key_header.unwrap());
        let rustc = rustc_version_runtime::version();
        headers.insert(
            "X-Platform-Version",
            HeaderValue::try_from(&rustc.to_string()).unwrap(),
        );

        let Ok(client) = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent(APP_USER_AGENT)
            .timeout(Duration::from_secs(10))
            .build()
        else {
            return Err("Error instantiating client.".into());
        };

        let Ok(base_url) = Url::parse(base_url) else {
            return Err("Invalid base_url.".into());
        };

        Ok(Self { client, base_url })
    }

    /// Gets the Events for the provided Date
    pub async fn get_events(
        &self,
        request: model::GetEventsRequest,
    ) -> Result<model::GetEventsResponse, String> {
        let mut params: HashMap<String, String> =
            HashMap::from([("adult".into(), request.adult.unwrap_or(false).to_string())]);

        if let Some(tz) = request.timezone {
            params.insert("timezone".into(), tz);
        }

        if let Some(date) = request.date {
            params.insert("date".into(), date);
        }

        self.request("events".into(), params).await
    }

    /// Gets the Event Info for the provided Event
    pub async fn get_event_info(
        &self,
        request: model::GetEventInfoRequest,
    ) -> Result<model::GetEventInfoResponse, String> {
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
    pub async fn search(
        &self,
        request: model::SearchRequest,
    ) -> Result<model::SearchResponse, String> {
        if request.query.is_empty() {
            return Err("Search query is required.".into());
        }

        let params: HashMap<String, String> = HashMap::from([
            ("query".into(), request.query),
            ("adult".into(), request.adult.unwrap_or(false).to_string()),
        ]);

        self.request("search".into(), params).await
    }

    async fn request<T>(&self, path: String, params: HashMap<String, String>) -> Result<T, String>
    where
        T: serde::de::DeserializeOwned + std::fmt::Debug + model::RateLimited,
    {
        let mut url = self.base_url.join(&path.to_string()).unwrap();
        url.query_pairs_mut().extend_pairs(params);

        let res = match self.client.get(url).send().await {
            Ok(ok) => ok,
            Err(e) => return Err(format!("Can't process request: {}", e)),
        };
        let status = res.status();
        if !status.is_success() {
            let json = res.json::<HashMap<String, String>>().await;
            return if json.is_err()
                || json
                    .as_ref()
                    .unwrap()
                    .get("error")
                    .unwrap_or(&"".into())
                    .is_empty()
            {
                Err(status.canonical_reason().unwrap_or(status.as_str()).into())
            } else {
                Err(json.unwrap().get("error").unwrap().to_owned())
            };
        }
        let headers = res.headers().to_owned();
        let json = match res.json::<T>().await {
            Ok(ok) => ok,
            Err(e) => return Err(format!("Can't parse response: {}", e)),
        };
        let rate_limit = model::RateLimit {
            limit_month: headers
                .get("x-ratelimit-limit-month")
                .and_then(|h| h.to_str().ok().and_then(|s| s.parse().ok()))
                .unwrap_or(0),
            remaining_month: headers
                .get("x-ratelimit-remaining-month")
                .and_then(|h| h.to_str().ok().and_then(|s| s.parse().ok()))
                .unwrap_or(0),
        };
        let mut result = json;
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
            let result = HolidayEventApi::new("");
            assert!(result.is_err());
            assert_eq!("Please provide a valid API key. Get one at https://apilayer.com/marketplace/checkiday-api#pricing.".to_string(), result.unwrap_err());
        }

        #[test]
        fn fails_with_invalid_base_url() {
            let result = HolidayEventApi::new_internal("abc123", "derp");
            assert!(result.is_err());
            assert_eq!("Invalid base_url.".to_string(), result.unwrap_err());
        }

        #[test]
        fn returns_a_new_client() {
            assert!(HolidayEventApi::new("abc123").is_ok());
        }
    }

    mod common_functionality {
        use super::*;

        #[test]
        fn passes_along_api_key() {
            let mut server = Server::new();

            let mock = server
                .mock("GET", "/events")
                .match_query(Matcher::Any)
                .match_header("apikey", "abc123")
                .with_body_from_file("testdata/getEvents-default.json")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            assert!(aw!(api.get_events(model::GetEventsRequest {
                date: None,
                adult: None,
                timezone: None,
            }))
            .is_ok());

            mock.assert();
        }

        #[test]
        fn passes_along_user_agent() {
            let mut server = Server::new();

            let app_version = env!("CARGO_PKG_VERSION");
            let mock = server
                .mock("GET", "/events")
                .match_query(Matcher::Any)
                .match_header(
                    "user-agent",
                    format!("HolidayApiRust/{app_version}").as_str(),
                )
                .with_body_from_file("testdata/getEvents-default.json")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            assert!(aw!(api.get_events(model::GetEventsRequest {
                date: None,
                adult: None,
                timezone: None,
            }))
            .is_ok());

            mock.assert();
        }

        #[test]
        fn passes_along_platform_version() {
            let mut server = Server::new();

            let app_version = rustc_version_runtime::version().to_string();
            let mock = server
                .mock("GET", "/events")
                .match_query(Matcher::Any)
                .match_header("x-platform-version", app_version.as_str())
                .with_body_from_file("testdata/getEvents-default.json")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            assert!(aw!(api.get_events(model::GetEventsRequest {
                date: None,
                adult: None,
                timezone: None,
            }))
            .is_ok());

            mock.assert();
        }

        #[test]
        fn passes_along_error() {
            let mut server = Server::new();

            let mock = server
                .mock("GET", "/events")
                .match_query(Matcher::Any)
                .with_status(401)
                .with_body("{\"error\":\"MyError!\"}")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest {
                date: None,
                adult: None,
                timezone: None,
            }));

            assert_eq!("MyError!", result.unwrap_err());

            mock.assert();
        }

        #[test]
        fn server_error_500() {
            let mut server = Server::new();

            let mock = server
                .mock("GET", "/events")
                .match_query(Matcher::Any)
                .with_status(500)
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest {
                date: None,
                adult: None,
                timezone: None,
            }));

            assert_eq!("Internal Server Error", result.unwrap_err());

            mock.assert();
        }

        #[test]
        fn server_error_unknown() {
            let mut server = Server::new();

            let mock = server
                .mock("GET", "/events")
                .match_query(Matcher::Any)
                .with_status(599)
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest {
                date: None,
                adult: None,
                timezone: None,
            }));

            assert_eq!("599", result.unwrap_err());

            mock.assert();
        }

        #[test]
        fn server_error_other() {
            let fake_url = "http://localhost";
            let api = HolidayEventApi::new_internal("abc123", fake_url).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest {
                date: None,
                adult: None,
                timezone: None,
            }));

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

            let mock = server
                .mock("GET", "/events")
                .match_query(Matcher::Any)
                .with_body("{")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest {
                date: None,
                adult: None,
                timezone: None,
            }));

            assert_eq!("Can't parse response: error decoding response body: EOF while parsing an object at line 1 column 1", result.unwrap_err());

            mock.assert();
        }

        #[test]
        fn follows_redirects() {
            let mut server = Server::new();

            let url = server.url();
            let mock = server
                .mock("GET", "/events")
                .match_query(Matcher::Any)
                .with_status(302)
                .with_header("Location", format!("{url}/redirected").as_str())
                .create();

            let mock2 = server
                .mock("GET", "/redirected")
                .match_query(Matcher::Any)
                .with_body_from_file("testdata/getEvents-default.json")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            assert!(aw!(api.get_events(model::GetEventsRequest {
                date: None,
                adult: None,
                timezone: None,
            }))
            .is_ok());

            mock.assert();
            mock2.assert();
        }

        #[test]
        fn reports_rate_limits() {
            let mut server = Server::new();

            let mock = server
                .mock("GET", "/events")
                .match_query(Matcher::Any)
                .with_header("X-RateLimit-Limit-Month", "100")
                .with_header("x-ratelimit-remaining-month", "88")
                .with_body_from_file("testdata/getEvents-default.json")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest {
                date: None,
                adult: None,
                timezone: None,
            }));

            assert!(result.is_ok());
            assert_eq!(
                model::RateLimit {
                    limit_month: 100,
                    remaining_month: 88,
                },
                result.unwrap().rate_limit
            );

            mock.assert();
        }
    }

    mod get_events {
        use super::*;

        #[test]
        fn fetches_with_default_parameters() {
            let mut server = Server::new();

            let mock = server
                .mock("GET", "/events")
                .match_query(Matcher::Any)
                .with_body_from_file("testdata/getEvents-default.json")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest {
                date: None,
                adult: None,
                timezone: None,
            }));

            assert!(result.is_ok());
            assert_eq!(model::GetEventsResponse {
                adult: false,
                date: model::DateOrTimestamp::Date("05/05/2025".into()),
                timezone: "America/Chicago".into(),
                events: vec![
                    model::EventSummary {
                        id: "b80630ae75c35f34c0526173dd999cfc".into(),
                        name: "Cinco de Mayo".into(),
                        url: "https://www.checkiday.com/b80630ae75c35f34c0526173dd999cfc/cinco-de-mayo"
                            .into(),
                    },
                    model::EventSummary {
                        id: "50bd02adb1a5fb297657a46a1b6b1082".into(),
                        name: "Great Lakes Awareness Day".into(),
                        url: "https://www.checkiday.com/50bd02adb1a5fb297657a46a1b6b1082/great-lakes-awareness-day"
                            .into(),
                    },
                ],
                multiday_starting: vec![
                    model::EventSummary {
                        id: "b9321bf3ce70e98fb385cb03d2f0cac4".into(),
                        name: "Teacher Appreciation Week".into(),
                        url: "https://www.checkiday.com/b9321bf3ce70e98fb385cb03d2f0cac4/teacher-appreciation-week"
                            .into(),
                    },
                ],
                multiday_ongoing: vec![
                    model::EventSummary {
                        id: "676cd91e31adcacd0a505117d2c4a842".into(),
                        name: "Be Kind to Animals Week".into(),
                        url: "https://www.checkiday.com/676cd91e31adcacd0a505117d2c4a842/be-kind-to-animals-week"
                            .into(),
                    },
                    model::EventSummary {
                        id: "decc6d9d46ac1e40bf345d963fe2a7a2".into(),
                        name: "National Children's Mental Health Awareness Week".into(),
                        url: "https://www.checkiday.com/decc6d9d46ac1e40bf345d963fe2a7a2/national-childrens-mental-health-awareness-week"
                            .into(),
                    },
                ],
                rate_limit: model::RateLimit { limit_month: 0, remaining_month: 0 },
            }, result.unwrap());

            mock.assert();
        }

        #[test]
        fn fetches_with_set_parameters() {
            let mut server = Server::new();

            let mock = server
                .mock("GET", "/events")
                .match_query(Matcher::AllOf(vec![
                    Matcher::UrlEncoded("adult".into(), "true".into()),
                    Matcher::UrlEncoded("timezone".into(), "America/New_York".into()),
                    Matcher::UrlEncoded("date".into(), "now".into()),
                ]))
                .with_body_from_file("testdata/getEvents-parameters.json")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.get_events(model::GetEventsRequest {
                date: Some("now".into()),
                adult: Some(true),
                timezone: Some("America/New_York".into()),
            }));

            assert!(result.is_ok());
            assert_eq!(model::GetEventsResponse {
                timezone: "America/New_York".into(),
                date: model::DateOrTimestamp::Timestamp(1682652947),
                adult: true,
                events: vec![
                    model::EventSummary {
                        id: "6ebb6fd5e483de2fde33969a6c398472".into(),
                        name: "Get to Know Your Customers Day".into(),
                        url: "https://www.checkiday.com/6ebb6fd5e483de2fde33969a6c398472/get-to-know-your-customers-day".into(),
                    },
                    model::EventSummary {
                        id: "b99556564fabc2f39e1b97c9a40e1e15".into(),
                        name: "National Atomic Veterans Day".into(),
                        url: "https://www.checkiday.com/b99556564fabc2f39e1b97c9a40e1e15/national-atomic-veterans-day".into(),
                    },
                ],
                multiday_starting: vec![],
                multiday_ongoing: vec![
                    model::EventSummary {
                        id: "9c64b0803f77735dc76c0cc0b6a1ccf0".into(),
                        name: "Hitchhiking Month".into(),
                        url: "https://www.checkiday.com/9c64b0803f77735dc76c0cc0b6a1ccf0/hitchhiking-month".into(),
                    },
                ],
                rate_limit: model::RateLimit { limit_month: 0, remaining_month: 0, }
            }, result.unwrap());

            mock.assert();
        }
    }

    mod get_event_info {
        use super::*;

        #[test]
        fn fetches_with_default_parameters() {
            let mut server = Server::new();

            let mock = server
                .mock("GET", "/event")
                .match_query(Matcher::UrlEncoded(
                    "id".into(),
                    "f90b893ea04939d7456f30c54f68d7b4".into(),
                ))
                .with_body_from_file("testdata/getEventInfo-default.json")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.get_event_info(model::GetEventInfoRequest {
                id: "f90b893ea04939d7456f30c54f68d7b4".into(),
                start: None,
                end: None,
            }));

            assert!(result.is_ok());
            assert_eq!(model::GetEventInfoResponse {
                event: model::EventInfo {
                    id: "f90b893ea04939d7456f30c54f68d7b4".into(),
                    name: "International Cat Day".into(),
                    url: "https://www.checkiday.com/f90b893ea04939d7456f30c54f68d7b4/international-cat-day".into(),
                    alternate_names: vec![model::AlternateName {
                        first_year: Some(2005),
                        last_year: None,
                        name: "TEST".into(),
                    }],
                    adult: false,
                    hashtags: Some(vec!["InternationalCatDay".into(), "CatDay".into()]),
                    image: Some(model::ImageInfo {
                        small: "https://static.checkiday.com/img/300/kittens-555822.jpg".into(),
                        medium: "https://static.checkiday.com/img/600/kittens-555822.jpg".into(),
                        large: "https://static.checkiday.com/img/1200/kittens-555822.jpg".into(),
                     }),
                    sources: Some(vec![
                        "https://www.source.com/1".into(),
                        "https://www.source.org/2".into(),
                    ]),
                    description: Some(model::RichText {
                        text: Some("International Cat Day celebrates love for cats...".into()),
                        html: Some("<p>International Cat Day <a href=\"https://www.google.com\">celebrates</a> love for cats...</p>".into()),
                        markdown: Some("International Cat Day [celebrates](https://www.google.com) love for cats...".into()),
                    }),
                    how_to_observe: Some(model::RichText {
                        text: Some("Spend the day playing with your cat...".into()),
                        html: Some("<p>Spend the day <a href=\"https://www.bing.com\">playing</a> with your cat...</p>".into()),
                        markdown: Some("Spend the day [playing](https://www.bing.com) with your cat...".into()),
                    }),
                    patterns: Some(vec![
                        model::Pattern{
                            first_year: Some(2002),
                            last_year: None,
                            observed: "annually on August 8th".into(),
                            observed_html: "annually on <a href=\"https://www.checkiday.com/8/8\">August 8th</a>".into(),
                            observed_markdown: "annually on [August 8th](https://www.checkiday.com/8/8)".into(),
                            length: 1,
                        }
                    ]),
                    founders: Some(vec![
                        model::FounderInfo {
                            name: "International Fund For Animal Welfare".into(),
                            date: Some("2002".into()),
                            url: Some("https://www.ifaw.org/".into()),
                        }
                    ]),
                    occurrences: Some(vec![
                        model::Occurrence {
                            date: model::DateOrTimestamp::Date("08/08/2020".into()),
                            length: 1,
                        },
                        model::Occurrence {
                            date: model::DateOrTimestamp::Date("08/08/2021".into()),
                            length: 1,
                        },
                        model::Occurrence {
                            date: model::DateOrTimestamp::Date("08/08/2022".into()),
                            length: 1,
                        },
                        model::Occurrence {
                            date: model::DateOrTimestamp::Date("08/08/2023".into()),
                            length: 1,
                        },
                        model::Occurrence {
                            date: model::DateOrTimestamp::Date("08/08/2024".into()),
                            length: 1,
                        },
                        model::Occurrence {
                            date: model::DateOrTimestamp::Timestamp(1734772794),
                            length: 1,
                        },
                        model::Occurrence {
                            date: model::DateOrTimestamp::Timestamp(-12345),
                            length: 7,
                        },
                    ]),
                    analytics: Some(model::Analytics { overall_rank: 12, social_rank: 34, social_shares: 56, popularity: "★★★☆☆".into() }),
                    tags: Some(vec![model::Tag{name: "A".into()}, model::Tag{name: "B".into()}]),
                },
                rate_limit: model::RateLimit { limit_month: 0, remaining_month: 0, }
            }, result.unwrap());

            mock.assert();
        }

        #[test]
        fn fetches_with_set_parameters() {
            let mut server = Server::new();

            let mock = server
                .mock("GET", "/event")
                .match_query(Matcher::AllOf(vec![
                    Matcher::UrlEncoded("id".into(), "f90b893ea04939d7456f30c54f68d7b4".into()),
                    Matcher::UrlEncoded("start".into(), "2002".into()),
                    Matcher::UrlEncoded("end".into(), "2003".into()),
                ]))
                .with_body_from_file("testdata/getEventInfo-parameters.json")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.get_event_info(model::GetEventInfoRequest {
                id: "f90b893ea04939d7456f30c54f68d7b4".into(),
                start: Some(2002),
                end: Some(2003),
            }));

            assert!(result.is_ok());
            assert_eq!(model::GetEventInfoResponse {
                event: model::EventInfo {
                    id: "f90b893ea04939d7456f30c54f68d7b4".into(),
                    name: "International Cat Day".into(),
                    url: "https://www.checkiday.com/f90b893ea04939d7456f30c54f68d7b4/international-cat-day".into(),
                    alternate_names: vec![model::AlternateName {
                        first_year: Some(2005),
                        last_year: None,
                        name: "TEST".into(),
                    }],
                    adult: false,
                    hashtags: Some(vec!["InternationalCatDay".into(), "CatDay".into()]),
                    image: Some(model::ImageInfo {
                        small: "https://static.checkiday.com/img/300/kittens-555822.jpg".into(),
                        medium: "https://static.checkiday.com/img/600/kittens-555822.jpg".into(),
                        large: "https://static.checkiday.com/img/1200/kittens-555822.jpg".into(),
                     }),
                    sources: Some(vec![
                        "https://www.source.com/1".into(),
                        "https://www.source.org/2".into(),
                    ]),
                    description: Some(model::RichText {
                        text: Some("International Cat Day celebrates love for cats...".into()),
                        html: Some("<p>International Cat Day <a href=\"https://www.google.com\">celebrates</a> love for cats...</p>".into()),
                        markdown: Some("International Cat Day [celebrates](https://www.google.com) love for cats...".into()),
                    }),
                    how_to_observe: Some(model::RichText {
                        text: Some("Spend the day playing with your cat...".into()),
                        html: Some("<p>Spend the day <a href=\"https://www.bing.com\">playing</a> with your cat...</p>".into()),
                        markdown: Some("Spend the day [playing](https://www.bing.com) with your cat...".into()),
                    }),
                    patterns: Some(vec![
                        model::Pattern{
                            first_year: Some(2002),
                            last_year: None,
                            observed: "annually on August 8th".into(),
                            observed_html: "annually on <a href=\"https://www.checkiday.com/8/8\">August 8th</a>".into(),
                            observed_markdown: "annually on [August 8th](https://www.checkiday.com/8/8)".into(),
                            length: 1,
                        }
                    ]),
                    founders: Some(vec![
                        model::FounderInfo {
                            name: "International Fund For Animal Welfare".into(),
                            date: Some("2002".into()),
                            url: Some("https://www.ifaw.org/".into()),
                        }
                    ]),
                    occurrences: Some(vec![
                        model::Occurrence {
                            date: model::DateOrTimestamp::Date("08/08/2002".into()),
                            length: 1,
                        },
                        model::Occurrence {
                            date: model::DateOrTimestamp::Timestamp(1734772794),
                            length: 1,
                        },
                        model::Occurrence {
                            date: model::DateOrTimestamp::Timestamp(-12345),
                            length: 7,
                        },
                    ]),
                    analytics: Some(model::Analytics { overall_rank: 12, social_rank: 34, social_shares: 56, popularity: "★★★☆☆".into() }),
                    tags: Some(vec![model::Tag{name: "A".into()}, model::Tag{name: "B".into()}]),
                },
                rate_limit: model::RateLimit { limit_month: 0, remaining_month: 0, }
            }, result.unwrap());

            mock.assert();
        }

        #[test]
        fn fetches_with_starter_plan() {
            let mut server = Server::new();

            let mock = server
                .mock("GET", "/event")
                .match_query(Matcher::UrlEncoded(
                    "id".into(),
                    "1a85c01ea2a6e3f921667c59391aa7ee".into(),
                ))
                .with_body_from_file("testdata/getEventInfo-starter.json")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.get_event_info(model::GetEventInfoRequest {
                id: "1a85c01ea2a6e3f921667c59391aa7ee".into(),
                start: None,
                end: None,
            }));

            assert!(result.is_ok());
            assert_eq!(model::GetEventInfoResponse {
                event: model::EventInfo {
                    id: "1a85c01ea2a6e3f921667c59391aa7ee".into(),
                    name: "International Pay it Forward Day".into(),
                    url: "https://www.checkiday.com/1a85c01ea2a6e3f921667c59391aa7ee/international-pay-it-forward-day".into(),
                    alternate_names: vec![model::AlternateName {
                        first_year: None,
                        last_year: None,
                        name: "Pay it Forward Day".into(),
                    }],
                    adult: false,
                    hashtags: None,
                    image: None,
                    sources: None,
                    description: None,
                    patterns: None,
                    how_to_observe: None,
                    founders: None,
                    occurrences: None,
                    analytics: None,
                    tags: None,
                },
                rate_limit: model::RateLimit { limit_month: 0, remaining_month: 0, }
            }, result.unwrap());

            mock.assert();
        }

        #[test]
        fn invalid_event() {
            let mut server = Server::new();

            let mock = server
                .mock("GET", "/event")
                .match_query(Matcher::AllOf(vec![Matcher::UrlEncoded(
                    "id".into(),
                    "hi".into(),
                )]))
                .with_status(404)
                .with_body("{\"error\":\"Event not found.\"}")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.get_event_info(model::GetEventInfoRequest {
                id: "hi".into(),
                start: None,
                end: None,
            }));

            assert!(result.is_err());
            assert_eq!("Event not found.", result.unwrap_err());

            mock.assert();
        }

        #[test]
        fn missing_id() {
            let api = HolidayEventApi::new("abc123").unwrap();
            let result = aw!(api.get_event_info(model::GetEventInfoRequest {
                id: "".into(),
                start: None,
                end: None,
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

            let mock = server
                .mock("GET", "/search")
                .match_query(Matcher::UrlEncoded("query".into(), "zucchini".into()))
                .with_body_from_file("testdata/search-default.json")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.search(model::SearchRequest {
                query: "zucchini".into(),
                adult: None,
            }));

            assert!(result.is_ok());
            assert_eq!(model::SearchResponse {
                query: "zucchini".into(),
                adult: false,
                events: vec![
                    model::EventSummary {
                        id: "cc81cbd8730098456f85f69798cbc867".into(),
                        name: "National Zucchini Bread Day".into(),
                        url: "https://www.checkiday.com/cc81cbd8730098456f85f69798cbc867/national-zucchini-bread-day".into(),
                    },
                    model::EventSummary {
                        id: "778e08321fc0ca4ec38fbf507c0e6c26".into(),
                        name: "National Zucchini Day".into(),
                        url: "https://www.checkiday.com/778e08321fc0ca4ec38fbf507c0e6c26/national-zucchini-day".into(),
                    },
                ],
                rate_limit: model::RateLimit { limit_month: 0, remaining_month: 0 },
            }, result.unwrap());

            mock.assert();
        }

        #[test]
        fn fetches_with_set_parameters() {
            let mut server = Server::new();

            let mock = server
                .mock("GET", "/search")
                .match_query(Matcher::UrlEncoded("query".into(), "porch day".into()))
                .match_query(Matcher::UrlEncoded("adult".into(), "true".into()))
                .with_body_from_file("testdata/search-parameters.json")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.search(model::SearchRequest {
                query: "porch day".into(),
                adult: Some(true),
            }));

            assert!(result.is_ok());
            assert_eq!(model::SearchResponse {
                query: "porch day".into(),
                adult: true,
                events: vec![
                    model::EventSummary {
                        id: "61363236f06e4eb8e4e14e5925c2503d".into(),
                        name: "Sneak Some Zucchini Onto Your Neighbor's Porch Day".into(),
                        url: "https://www.checkiday.com/61363236f06e4eb8e4e14e5925c2503d/sneak-some-zucchini-onto-your-neighbors-porch-day".into(),
                    },
                ],
                rate_limit: model::RateLimit { limit_month: 0, remaining_month: 0 },
            }, result.unwrap());

            mock.assert();
        }

        #[test]
        fn query_too_short() {
            let mut server = Server::new();

            let mock = server
                .mock("GET", "/search")
                .match_query(Matcher::UrlEncoded("query".into(), "a".into()))
                .with_status(400)
                .with_body("{\"error\":\"Please enter a longer search term.\"}")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.search(model::SearchRequest {
                query: "a".into(),
                adult: None,
            }));

            assert!(result.is_err());
            assert_eq!("Please enter a longer search term.", result.unwrap_err());

            mock.assert();
        }

        #[test]
        fn too_many_results() {
            let mut server = Server::new();

            let mock = server
                .mock("GET", "/search")
                .match_query(Matcher::UrlEncoded("query".into(), "day".into()))
                .with_status(400)
                .with_body("{\"error\":\"Too many results returned. Please refine your query.\"}")
                .create();

            let api = HolidayEventApi::new_internal("abc123", &server.url()).unwrap();
            let result = aw!(api.search(model::SearchRequest {
                query: "day".into(),
                adult: None,
            }));

            assert!(result.is_err());
            assert_eq!(
                "Too many results returned. Please refine your query.",
                result.unwrap_err()
            );

            mock.assert();
        }

        #[test]
        fn missing_parameters() {
            let api = HolidayEventApi::new("abc123").unwrap();
            let result = aw!(api.search(model::SearchRequest {
                query: "".into(),
                adult: None,
            }));

            assert!(result.is_err());
            assert_eq!("Search query is required.", result.unwrap_err());
        }
    }
}
