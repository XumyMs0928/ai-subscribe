//! Shared outbound-source network policy.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use url::Url;

use crate::contracts::errors::{AppError, ErrorCode};
use crate::domain::sources::{FetchIncrementalResult, is_public_ip};
use crate::infrastructure::sources::rss_atom::parse_feed_until;

pub const MAX_RESPONSE_BYTES: usize = 10_000_000;
pub const MAX_REDIRECTS: usize = 5;
pub const REQUEST_DEADLINE_SECONDS: u64 = 30;
pub const MAX_RETRY_DELAY_MS: u64 = 30 * 24 * 60 * 60 * 1_000;

pub type SourceFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait SourceResolver: Send + Sync {
    fn resolve<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> SourceFuture<'a, Result<Vec<SocketAddr>, AppError>>;
}

pub trait SourceConnector: Send + Sync {
    fn send_pinned<'a>(
        &'a self,
        request: SourceHttpRequest,
        addresses: &'a [SocketAddr],
    ) -> SourceFuture<'a, Result<SourceHttpResponse, AppError>>;
}

pub trait SourceClock: Send + Sync {
    fn now_epoch_seconds(&self) -> u64;
    fn deadline_exceeded(&self) -> bool;
}

#[derive(Clone, Debug)]
pub struct SourceHttpRequest {
    pub url: Url,
    pub headers: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct SourceHttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

struct TokioResolver;
struct ReqwestConnector;
struct SystemClock {
    deadline: Instant,
}

/// Canonicalizes a first-phase public HTTPS endpoint without performing I/O.
///
/// # Errors
/// Returns a stable validation error for malformed or non-HTTPS endpoints.
pub fn canonicalize_public_https_url(value: &str) -> Result<Url, AppError> {
    if value.len() > 2_048 {
        return Err(source_error(
            ErrorCode::ValidationSource,
            "source-url-length",
        ));
    }
    let mut url = Url::parse(value)
        .map_err(|_| source_error(ErrorCode::ValidationSource, "source-url-parse"))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(source_error(
            ErrorCode::ValidationSource,
            "source-url-policy",
        ));
    }
    if url.port() == Some(443) {
        url.set_port(None)
            .map_err(|()| source_error(ErrorCode::ValidationSource, "source-url-port"))?;
    }
    url.set_fragment(None);
    Ok(url)
}

/// Requires every resolved address to be globally routable.
///
/// # Errors
/// Returns a stable validation error when the set is empty or includes a non-public address.
pub fn validate_public_ips(addresses: &[IpAddr]) -> Result<(), AppError> {
    if addresses.is_empty() || addresses.iter().any(|ip| !is_public_ip(*ip)) {
        return Err(source_error(
            ErrorCode::ValidationSource,
            "source-address-policy",
        ));
    }
    Ok(())
}

#[must_use]
pub fn retry_delay_ms(consecutive_failure: u32, retry_after_ms: Option<u64>) -> u64 {
    let exponent = consecutive_failure.saturating_sub(1).min(10);
    let fallback = 60_000_u64.saturating_mul(1_u64 << exponent);
    retry_after_ms
        .filter(|delay| *delay <= MAX_RETRY_DELAY_MS)
        .unwrap_or(0)
        .max(fallback)
        .min(MAX_RETRY_DELAY_MS)
}

/// Parses an RFC 9110 Retry-After delta-seconds or IMF-fixdate into a delay.
#[must_use]
pub fn parse_retry_after_ms(value: &str, now_epoch_seconds: u64) -> Option<u64> {
    if let Ok(seconds) = value.trim().parse::<u64>() {
        return seconds.checked_mul(1_000);
    }
    let target = parse_imf_fixdate_epoch_seconds(value)?;
    Some(
        target
            .saturating_sub(now_epoch_seconds)
            .saturating_mul(1_000),
    )
}

/// Parses one strict IMF-fixdate and returns its Unix timestamp.
#[must_use]
pub(crate) fn parse_imf_fixdate_epoch_seconds(value: &str) -> Option<u64> {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 6
        || parts[5] != "GMT"
        || parts[0].len() != 4
        || !parts[0].ends_with(',')
        || parts[1].len() != 2
        || parts[3].len() != 4
        || parts[4].len() != 8
    {
        return None;
    }
    let weekday = match &parts[0][..3] {
        "Sun" => 0,
        "Mon" => 1,
        "Tue" => 2,
        "Wed" => 3,
        "Thu" => 4,
        "Fri" => 5,
        "Sat" => 6,
        _ => return None,
    };
    let day = parts[1].parse::<u32>().ok()?;
    let month = match parts[2] {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let year = parts[3].parse::<i64>().ok()?;
    let time = parts[4]
        .split(':')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if time.len() != 3
        || time[0] > 23
        || time[1] > 59
        || time[2] > 59
        || day == 0
        || day > days_in_month(year, month)
    {
        return None;
    }
    let days = days_from_civil(year, month, day);
    if days < 0 {
        return None;
    }
    let actual_weekday = (days + 4).rem_euclid(7);
    if actual_weekday != weekday {
        return None;
    }
    let target = u64::try_from(days)
        .ok()?
        .checked_mul(86_400)?
        .checked_add(u64::from(time[0]) * 3_600 + u64::from(time[1]) * 60 + u64::from(time[2]))?;
    Some(target)
}

const fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = i64::from(month) + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

impl SourceResolver for TokioResolver {
    fn resolve<'a>(
        &'a self,
        host: &'a str,
        port: u16,
    ) -> SourceFuture<'a, Result<Vec<SocketAddr>, AppError>> {
        Box::pin(async move {
            tokio::net::lookup_host((host, port))
                .await
                .map(std::iter::Iterator::collect)
                .map_err(|_| source_error(ErrorCode::NetworkSource, "source-dns"))
        })
    }
}

impl SourceConnector for ReqwestConnector {
    fn send_pinned<'a>(
        &'a self,
        request: SourceHttpRequest,
        addresses: &'a [SocketAddr],
    ) -> SourceFuture<'a, Result<SourceHttpResponse, AppError>> {
        Box::pin(async move {
            let host = request
                .url
                .host_str()
                .ok_or_else(|| source_error(ErrorCode::ValidationSource, "source-host"))?;
            let client = reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .no_proxy()
                .connect_timeout(Duration::from_secs(10))
                .resolve_to_addrs(host, addresses)
                .build()
                .map_err(|_| source_error(ErrorCode::NetworkSource, "source-client"))?;
            let mut outgoing = client.get(request.url);
            for (name, value) in request.headers {
                outgoing = outgoing.header(name, value);
            }
            let mut response = outgoing
                .send()
                .await
                .map_err(|_| source_error(ErrorCode::NetworkSource, "source-connect-or-tls"))?;
            if response
                .content_length()
                .is_some_and(|length| length > MAX_RESPONSE_BYTES as u64)
            {
                return Err(source_error(
                    ErrorCode::NetworkSource,
                    "source-content-length",
                ));
            }
            validate_content_encoding(
                response
                    .headers()
                    .get(reqwest::header::CONTENT_ENCODING)
                    .map(reqwest::header::HeaderValue::as_bytes),
            )?;
            let status = response.status().as_u16();
            let headers = response
                .headers()
                .iter()
                .filter_map(|(name, value)| {
                    value
                        .to_str()
                        .ok()
                        .map(|value| (name.as_str().to_ascii_lowercase(), value.to_owned()))
                })
                .collect();
            let mut body = Vec::new();
            while let Some(chunk) = response
                .chunk()
                .await
                .map_err(|_| source_error(ErrorCode::NetworkSource, "source-body"))?
            {
                append_bounded_chunk(&mut body, &chunk)?;
            }
            Ok(SourceHttpResponse {
                status,
                headers,
                body,
            })
        })
    }
}

impl SourceClock for SystemClock {
    fn now_epoch_seconds(&self) -> u64 {
        now_epoch_seconds()
    }

    fn deadline_exceeded(&self) -> bool {
        Instant::now() >= self.deadline
    }
}

/// Performs the production RSS/Atom probe with manual redirect and DNS pinning.
///
/// The client is rebuilt for each authority so the actual connector uses the exact
/// address set that passed policy validation for that redirect generation.
///
/// # Errors
/// Returns a redacted validation, network, rate-limit, or source-format error.
pub async fn probe_rss_atom_source(
    value: &str,
    etag: Option<&str>,
    last_modified: Option<&str>,
) -> Result<FetchIncrementalResult, AppError> {
    let deadline = Instant::now() + Duration::from_secs(REQUEST_DEADLINE_SECONDS);
    tokio::time::timeout(
        Duration::from_secs(REQUEST_DEADLINE_SECONDS),
        probe_rss_atom_source_with(
            value,
            etag,
            last_modified,
            &TokioResolver,
            &ReqwestConnector,
            &SystemClock { deadline },
        ),
    )
    .await
    .map_err(|_| source_error(ErrorCode::NetworkSource, "source-total-timeout"))?
}

/// Executes the same policy using injected resolver, pinned connector, and clock.
///
/// # Errors
/// Returns a stable, redacted source error.
#[allow(clippy::too_many_lines)]
pub async fn probe_rss_atom_source_with(
    value: &str,
    etag: Option<&str>,
    last_modified: Option<&str>,
    resolver: &dyn SourceResolver,
    connector: &dyn SourceConnector,
    clock: &dyn SourceClock,
) -> Result<FetchIncrementalResult, AppError> {
    let mut url = canonicalize_public_https_url(value)?;
    let deadline = Instant::now() + Duration::from_secs(REQUEST_DEADLINE_SECONDS);
    let original_origin = url.origin().ascii_serialization();
    let mut visited = HashSet::new();
    for redirect_count in 0..=MAX_REDIRECTS {
        if clock.deadline_exceeded() {
            return Err(source_error(
                ErrorCode::NetworkSource,
                "source-total-timeout",
            ));
        }
        if !visited.insert(url.to_string()) {
            return Err(source_error(
                ErrorCode::NetworkSource,
                "source-redirect-loop",
            ));
        }
        let host = url
            .host_str()
            .ok_or_else(|| source_error(ErrorCode::ValidationSource, "source-host"))?;
        let port = url.port_or_known_default().unwrap_or(443);
        let addresses = resolver.resolve(host, port).await?;
        validate_public_ips(&addresses.iter().map(SocketAddr::ip).collect::<Vec<_>>())?;
        let mut headers = HashMap::from([(
            "accept".to_owned(),
            "application/rss+xml, application/atom+xml, application/xml, text/xml;q=0.9".to_owned(),
        )]);
        if url.origin().ascii_serialization() == original_origin {
            if let Some(value) = etag {
                headers.insert("if-none-match".to_owned(), value.to_owned());
            }
            if let Some(value) = last_modified {
                headers.insert("if-modified-since".to_owned(), value.to_owned());
            }
        }
        let response = connector
            .send_pinned(
                SourceHttpRequest {
                    url: url.clone(),
                    headers,
                },
                &addresses,
            )
            .await?;
        if response.status == 304 {
            if etag.is_none() && last_modified.is_none() {
                return Err(source_error(
                    ErrorCode::NetworkSource,
                    "source-unconditional-304",
                ));
            }
            return Ok(FetchIncrementalResult {
                candidates: Vec::new(),
                etag: response
                    .headers
                    .get("etag")
                    .cloned()
                    .or_else(|| etag.map(ToOwned::to_owned)),
                last_modified: response
                    .headers
                    .get("last-modified")
                    .cloned()
                    .or_else(|| last_modified.map(ToOwned::to_owned)),
                adapter_cursor: None,
                not_modified: true,
            });
        }
        if (300..400).contains(&response.status) {
            if redirect_count == MAX_REDIRECTS {
                return Err(source_error(
                    ErrorCode::NetworkSource,
                    "source-redirect-limit",
                ));
            }
            let location = response.headers.get("location").ok_or_else(|| {
                source_error(ErrorCode::NetworkSource, "source-redirect-location")
            })?;
            url = canonicalize_public_https_url(
                url.join(location)
                    .map_err(|_| source_error(ErrorCode::ValidationSource, "source-redirect-url"))?
                    .as_str(),
            )?;
            continue;
        }
        if response.status == 429 {
            let retry_after_ms = response
                .headers
                .get("retry-after")
                .and_then(|value| parse_retry_after_ms(value, clock.now_epoch_seconds()))
                .filter(|delay| *delay <= MAX_RETRY_DELAY_MS);
            let error = source_error(ErrorCode::RateLimitedSource, "source-rate-limited");
            return Err(
                retry_after_ms.map_or(error.clone(), |delay| error.with_retry_after_ms(delay))
            );
        }
        if (500..600).contains(&response.status) {
            return Err(source_error(
                ErrorCode::NetworkSource,
                "source-server-error",
            ));
        }
        if !(200..300).contains(&response.status) {
            let code = if matches!(
                response.status,
                400 | 401 | 403 | 404 | 405 | 406 | 410 | 415 | 422
            ) {
                ErrorCode::ValidationSource
            } else {
                ErrorCode::NetworkSource
            };
            return Err(source_error(code, "source-http-status"));
        }
        validate_content_encoding(
            response
                .headers
                .get("content-encoding")
                .map(String::as_bytes),
        )?;
        if response.body.len() > MAX_RESPONSE_BYTES {
            return Err(source_error(
                ErrorCode::NetworkSource,
                "source-response-budget",
            ));
        }
        if clock.deadline_exceeded() {
            return Err(source_error(
                ErrorCode::NetworkSource,
                "source-total-timeout",
            ));
        }
        return Ok(FetchIncrementalResult {
            candidates: parse_feed_until(&response.body, Some(deadline))?,
            etag: response.headers.get("etag").cloned(),
            last_modified: response.headers.get("last-modified").cloned(),
            adapter_cursor: Some(format!("rss-atom-v1:{:x}", Sha256::digest(&response.body))),
            not_modified: false,
        });
    }
    Err(source_error(
        ErrorCode::NetworkSource,
        "source-redirect-limit",
    ))
}

fn validate_content_encoding(value: Option<&[u8]>) -> Result<(), AppError> {
    // V1 deliberately accepts no compressed representation. This makes raw and decoded size
    // identical and rejects compression bombs before allocating or reading their body.
    if value.is_none_or(|value| value.eq_ignore_ascii_case(b"identity")) {
        Ok(())
    } else {
        Err(source_error(
            ErrorCode::SourceFormatRssAtom,
            "source-content-encoding",
        ))
    }
}

fn append_bounded_chunk(body: &mut Vec<u8>, chunk: &[u8]) -> Result<(), AppError> {
    if body.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
        return Err(source_error(
            ErrorCode::NetworkSource,
            "source-response-budget",
        ));
    }
    body.extend_from_slice(chunk);
    Ok(())
}

fn now_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn source_error(code: ErrorCode, boundary: &'static str) -> AppError {
    AppError::from_code(code, boundary)
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, VecDeque};
    use std::future::Future;
    use std::net::SocketAddr;
    use std::pin::pin;
    use std::sync::Mutex;
    use std::task::{Context, Poll, Waker};

    use super::{
        AppError, ErrorCode, MAX_RESPONSE_BYTES, SourceClock, SourceConnector, SourceFuture,
        SourceHttpRequest, SourceHttpResponse, SourceResolver, append_bounded_chunk,
        probe_rss_atom_source_with, validate_content_encoding,
    };

    const RSS: &[u8] = include_bytes!("../../../../../contracts/fixtures/rss-atom/rss2-v1.xml");

    fn block_on<F: Future>(future: F) -> F::Output {
        let mut context = Context::from_waker(Waker::noop());
        let mut future = pin!(future);
        loop {
            match future.as_mut().poll(&mut context) {
                Poll::Ready(output) => return output,
                Poll::Pending => std::thread::yield_now(),
            }
        }
    }

    struct FakeResolver {
        answers: Mutex<VecDeque<Vec<SocketAddr>>>,
    }

    impl SourceResolver for FakeResolver {
        fn resolve<'a>(
            &'a self,
            _host: &'a str,
            _port: u16,
        ) -> SourceFuture<'a, Result<Vec<SocketAddr>, AppError>> {
            Box::pin(async move {
                self.answers
                    .lock()
                    .expect("resolver lock")
                    .pop_front()
                    .ok_or_else(|| AppError::from_code(ErrorCode::NetworkSource, "fake-dns"))
            })
        }
    }

    struct FakeConnector {
        responses: Mutex<VecDeque<SourceHttpResponse>>,
        requests: Mutex<Vec<(SourceHttpRequest, Vec<SocketAddr>)>>,
    }

    impl SourceConnector for FakeConnector {
        fn send_pinned<'a>(
            &'a self,
            request: SourceHttpRequest,
            addresses: &'a [SocketAddr],
        ) -> SourceFuture<'a, Result<SourceHttpResponse, AppError>> {
            Box::pin(async move {
                self.requests
                    .lock()
                    .expect("request lock")
                    .push((request, addresses.to_vec()));
                self.responses
                    .lock()
                    .expect("response lock")
                    .pop_front()
                    .ok_or_else(|| AppError::from_code(ErrorCode::NetworkSource, "fake-response"))
            })
        }
    }

    struct FailingConnector;

    impl SourceConnector for FailingConnector {
        fn send_pinned<'a>(
            &'a self,
            _request: SourceHttpRequest,
            _addresses: &'a [SocketAddr],
        ) -> SourceFuture<'a, Result<SourceHttpResponse, AppError>> {
            Box::pin(async {
                Err(AppError::from_code(
                    ErrorCode::NetworkSource,
                    "fixture-tls-failure",
                ))
            })
        }
    }

    struct FixedClock(u64);
    impl SourceClock for FixedClock {
        fn now_epoch_seconds(&self) -> u64 {
            self.0
        }

        fn deadline_exceeded(&self) -> bool {
            false
        }
    }

    struct ExpiredClock;
    impl SourceClock for ExpiredClock {
        fn now_epoch_seconds(&self) -> u64 {
            0
        }

        fn deadline_exceeded(&self) -> bool {
            true
        }
    }

    fn response(status: u16, headers: &[(&str, &str)], body: &[u8]) -> SourceHttpResponse {
        SourceHttpResponse {
            status,
            headers: headers
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect::<HashMap<_, _>>(),
            body: body.to_vec(),
        }
    }

    #[test]
    fn raw_stream_budget_accepts_exact_limit_and_rejects_next_byte() {
        let mut body = Vec::new();
        append_bounded_chunk(&mut body, &vec![0_u8; MAX_RESPONSE_BYTES]).expect("exact raw budget");
        assert_eq!(body.len(), MAX_RESPONSE_BYTES);
        assert!(append_bounded_chunk(&mut body, &[0]).is_err());
    }

    #[test]
    fn compressed_representations_are_rejected_before_body_processing() {
        assert!(validate_content_encoding(None).is_ok());
        assert!(validate_content_encoding(Some(b"IDENTITY")).is_ok());
        for encoding in [b"gzip".as_slice(), b"br", b"deflate", b"identity, gzip"] {
            assert!(validate_content_encoding(Some(encoding)).is_err());
        }
    }

    #[test]
    fn injected_policy_pins_public_resolution_and_sends_conditionals() {
        let address: SocketAddr = "8.8.8.8:443".parse().expect("public address");
        let resolver = FakeResolver {
            answers: Mutex::new(VecDeque::from([vec![address]])),
        };
        let connector = FakeConnector {
            responses: Mutex::new(VecDeque::from([response(304, &[("etag", "v2")], &[])])),
            requests: Mutex::new(Vec::new()),
        };
        let result = block_on(probe_rss_atom_source_with(
            "https://feeds.example.test/rss.xml",
            Some("v1"),
            None,
            &resolver,
            &connector,
            &FixedClock(1_787_047_200),
        ))
        .expect("304");
        assert!(result.not_modified);
        assert_eq!(result.etag.as_deref(), Some("v2"));
        let requests = connector.requests.lock().expect("requests");
        assert_eq!(requests[0].1, vec![address]);
        assert_eq!(
            requests[0].0.headers.get("if-none-match"),
            Some(&"v1".to_owned())
        );
    }

    #[test]
    fn injected_policy_revalidates_redirect_and_rejects_private_rebinding() {
        let resolver = FakeResolver {
            answers: Mutex::new(VecDeque::from([
                vec!["8.8.8.8:443".parse().unwrap()],
                vec!["127.0.0.1:443".parse().unwrap()],
            ])),
        };
        let connector = FakeConnector {
            responses: Mutex::new(VecDeque::from([response(
                302,
                &[("location", "https://private.example.test/feed.xml")],
                &[],
            )])),
            requests: Mutex::new(Vec::new()),
        };
        let error = block_on(probe_rss_atom_source_with(
            "https://feeds.example.test/start",
            None,
            None,
            &resolver,
            &connector,
            &FixedClock(0),
        ))
        .expect_err("private redirect");
        assert_eq!(error.code(), ErrorCode::ValidationSource.as_str());
        assert_eq!(connector.requests.lock().unwrap().len(), 1);
    }

    #[test]
    fn injected_policy_revalidates_public_redirect_and_drops_origin_conditionals() {
        let first: SocketAddr = "8.8.8.8:443".parse().unwrap();
        let second: SocketAddr = "1.1.1.1:443".parse().unwrap();
        let resolver = FakeResolver {
            answers: Mutex::new(VecDeque::from([vec![first], vec![second]])),
        };
        let connector = FakeConnector {
            responses: Mutex::new(VecDeque::from([
                response(
                    302,
                    &[("location", "https://cdn.example.test/final.xml")],
                    &[],
                ),
                response(200, &[], RSS),
            ])),
            requests: Mutex::new(Vec::new()),
        };
        let result = block_on(probe_rss_atom_source_with(
            "https://feeds.example.test/start",
            Some("v1"),
            Some("Sat, 01 Aug 2026 08:00:00 GMT"),
            &resolver,
            &connector,
            &FixedClock(0),
        ))
        .expect("public redirect");
        assert_eq!(result.candidates.len(), 2);
        let requests = connector.requests.lock().unwrap();
        assert_eq!(requests[0].1, vec![first]);
        assert_eq!(requests[1].1, vec![second]);
        assert!(requests[0].0.headers.contains_key("if-none-match"));
        assert!(!requests[1].0.headers.contains_key("if-none-match"));
        assert!(!requests[1].0.headers.contains_key("if-modified-since"));
    }

    #[test]
    fn injected_connector_failure_is_stably_redacted_as_network_source() {
        let resolver = FakeResolver {
            answers: Mutex::new(VecDeque::from([vec!["8.8.8.8:443".parse().unwrap()]])),
        };
        let error = block_on(probe_rss_atom_source_with(
            "https://feeds.example.test/rss.xml",
            None,
            None,
            &resolver,
            &FailingConnector,
            &FixedClock(0),
        ))
        .expect_err("TLS/connector failure");
        assert_eq!(error.code(), ErrorCode::NetworkSource.as_str());
        assert!(!format!("{error:?}").contains("feeds.example.test"));
    }

    #[test]
    fn injected_clock_controls_retry_after_without_sleeping() {
        let resolver = FakeResolver {
            answers: Mutex::new(VecDeque::from([vec!["8.8.8.8:443".parse().unwrap()]])),
        };
        let connector = FakeConnector {
            responses: Mutex::new(VecDeque::from([response(
                429,
                &[("retry-after", "Tue, 18 Aug 2026 10:02:00 GMT")],
                &[],
            )])),
            requests: Mutex::new(Vec::new()),
        };
        let error = block_on(probe_rss_atom_source_with(
            "https://feeds.example.test/rss.xml",
            None,
            None,
            &resolver,
            &connector,
            &FixedClock(1_787_047_200),
        ))
        .expect_err("rate limit");
        assert_eq!(error.retry_after_ms(), Some(120_000));
    }

    #[test]
    fn injected_transport_executes_the_real_parser_and_rejects_compression() {
        let address = "8.8.8.8:443".parse().unwrap();
        let resolver = FakeResolver {
            answers: Mutex::new(VecDeque::from([vec![address], vec![address]])),
        };
        let connector = FakeConnector {
            responses: Mutex::new(VecDeque::from([
                response(200, &[("etag", "v1")], RSS),
                response(200, &[("content-encoding", "GZip")], RSS),
            ])),
            requests: Mutex::new(Vec::new()),
        };
        let result = block_on(probe_rss_atom_source_with(
            "https://feeds.example.test/rss.xml",
            None,
            None,
            &resolver,
            &connector,
            &FixedClock(0),
        ))
        .expect("RSS");
        assert_eq!(result.candidates.len(), 2);
        let error = block_on(probe_rss_atom_source_with(
            "https://feeds.example.test/rss.xml",
            None,
            None,
            &resolver,
            &connector,
            &FixedClock(0),
        ))
        .expect_err("compressed response");
        assert_eq!(error.code(), ErrorCode::SourceFormatRssAtom.as_str());
    }

    #[test]
    fn injected_transport_rejects_body_larger_than_the_raw_stream_budget() {
        let resolver = FakeResolver {
            answers: Mutex::new(VecDeque::from([vec!["8.8.8.8:443".parse().unwrap()]])),
        };
        let connector = FakeConnector {
            responses: Mutex::new(VecDeque::from([response(
                200,
                &[],
                &vec![0_u8; MAX_RESPONSE_BYTES + 1],
            )])),
            requests: Mutex::new(Vec::new()),
        };
        let error = block_on(probe_rss_atom_source_with(
            "https://feeds.example.test/large.xml",
            None,
            None,
            &resolver,
            &connector,
            &FixedClock(0),
        ))
        .expect_err("oversized body");
        assert_eq!(error.code(), ErrorCode::NetworkSource.as_str());
    }

    #[test]
    fn injected_deadline_cancels_without_resolving_or_connecting() {
        let resolver = FakeResolver {
            answers: Mutex::new(VecDeque::from([vec!["8.8.8.8:443".parse().unwrap()]])),
        };
        let connector = FakeConnector {
            responses: Mutex::new(VecDeque::from([response(200, &[], RSS)])),
            requests: Mutex::new(Vec::new()),
        };
        let error = block_on(probe_rss_atom_source_with(
            "https://feeds.example.test/rss.xml",
            None,
            None,
            &resolver,
            &connector,
            &ExpiredClock,
        ))
        .expect_err("expired deadline");
        assert_eq!(error.code(), ErrorCode::NetworkSource.as_str());
        assert!(connector.requests.lock().unwrap().is_empty());
    }
}
