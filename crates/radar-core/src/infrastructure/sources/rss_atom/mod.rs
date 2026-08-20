//! Bounded RSS 2.0 and Atom parser.

use std::fmt::Write as _;
use std::time::Instant;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use sha2::{Digest, Sha256};
use url::Url;

use crate::contracts::errors::{AppError, ErrorCode};
use crate::domain::sources::{RawSourceCandidate, SourceFieldWarning};
use crate::infrastructure::http::source_http_policy::MAX_RESPONSE_BYTES;

const MAX_ITEMS: usize = 1_000;
const MAX_TEXT_BYTES: usize = 256_000;
const MAX_XML_DEPTH: usize = 64;
const ATOM_NAMESPACE: &[u8] = b"http://www.w3.org/2005/Atom";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FeedKind {
    Rss,
    Atom,
}

#[derive(Default)]
struct CandidateBuilder {
    kind: Option<FeedKind>,
    namespace_prefix: Option<Vec<u8>>,
    entry_index: usize,
    link_priority: u8,
    stable_id: Option<String>,
    title: Option<String>,
    original_url: Option<String>,
    author: Option<String>,
    summary: Option<String>,
    published_at: Option<String>,
    updated_at: Option<String>,
}

/// Parses one bounded RSS 2.0 or Atom payload into internal source candidates.
///
/// # Errors
/// Returns a stable source-format error for malformed, unsupported, or oversized input.
pub fn parse_feed(bytes: &[u8]) -> Result<Vec<RawSourceCandidate>, AppError> {
    parse_feed_until(bytes, None)
}

#[allow(clippy::too_many_lines)] // Keeping the bounded XML state machine in one loop makes state transitions auditable.
pub(crate) fn parse_feed_until(
    bytes: &[u8],
    deadline: Option<Instant>,
) -> Result<Vec<RawSourceCandidate>, AppError> {
    if bytes.len() > MAX_RESPONSE_BYTES || has_unsupported_encoding(bytes) {
        return Err(format_error("rss-size-or-encoding"));
    }
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_end_names = true;
    let mut stack: Vec<Vec<u8>> = Vec::new();
    let mut current: Option<CandidateBuilder> = None;
    let mut output = Vec::new();
    let mut feed_kind = None;
    let mut feed_prefix: Option<Vec<u8>> = None;
    let mut encountered_entries = 0_usize;

    loop {
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(format_error("rss-deadline"));
        }
        match reader.read_event() {
            Ok(Event::Decl(declaration)) => validate_declaration_encoding(&declaration)?,
            Ok(Event::Start(event)) => {
                let raw_name = event.name().as_ref().to_vec();
                if stack.is_empty() {
                    let (kind, prefix) = validate_root(&event, reader.decoder())?;
                    feed_kind = Some(kind);
                    feed_prefix = prefix;
                } else {
                    reject_namespace_rebinding(&event)?;
                }
                if stack.len() >= MAX_XML_DEPTH {
                    return Err(format_error("rss-depth-limit"));
                }
                stack.push(raw_name.clone());
                let name = feed_kind
                    .and_then(|kind| feed_local_name(kind, feed_prefix.as_deref(), &raw_name));
                if matches!(name, Some(b"item" | b"entry")) {
                    if current.is_some() {
                        return Err(format_error("rss-nested-entry"));
                    }
                    let kind = feed_kind.ok_or_else(|| format_error("rss-root"))?;
                    if !is_valid_entry_path(kind, &stack) {
                        return Err(format_error("rss-entry-path"));
                    }
                    encountered_entries = encountered_entries.saturating_add(1);
                    if encountered_entries > MAX_ITEMS {
                        return Err(format_error("rss-item-limit"));
                    }
                    current = Some(CandidateBuilder {
                        kind: Some(kind),
                        namespace_prefix: feed_prefix.clone(),
                        entry_index: stack.len() - 1,
                        ..CandidateBuilder::default()
                    });
                }
                if name == Some(b"link") {
                    assign_link_attributes(&mut current, &event, reader.decoder())?;
                }
            }
            Ok(Event::Empty(event)) => {
                let raw_name = event.name().as_ref().to_vec();
                if stack.is_empty() {
                    let (kind, prefix) = validate_root(&event, reader.decoder())?;
                    if matches!(kind, FeedKind::Rss | FeedKind::Atom) {
                        feed_kind = Some(kind);
                        feed_prefix = prefix;
                    }
                } else {
                    reject_namespace_rebinding(&event)?;
                }
                if feed_kind
                    .and_then(|kind| feed_local_name(kind, feed_prefix.as_deref(), &raw_name))
                    == Some(b"link")
                {
                    assign_link_attributes(&mut current, &event, reader.decoder())?;
                }
            }
            Ok(Event::Text(event)) => {
                if let Some(builder) = current.as_mut() {
                    let text = event.decode().map_err(|_| format_error("rss-text"))?;
                    let text = quick_xml::escape::unescape(&text)
                        .map_err(|_| format_error("rss-entity"))?;
                    assign_text(builder, &stack, &text)?;
                }
            }
            Ok(Event::CData(event)) => {
                if let Some(builder) = current.as_mut() {
                    let text = event.decode().map_err(|_| format_error("rss-cdata"))?;
                    assign_text(builder, &stack, &text)?;
                }
            }
            Ok(Event::End(event)) => {
                let raw_name = event.name().as_ref().to_vec();
                let popped = stack.pop().ok_or_else(|| format_error("rss-stack"))?;
                if popped != raw_name {
                    return Err(format_error("rss-end"));
                }
                let name = feed_kind
                    .and_then(|kind| feed_local_name(kind, feed_prefix.as_deref(), &raw_name));
                if matches!(name, Some(b"item" | b"entry")) {
                    let builder = current.take().ok_or_else(|| format_error("rss-entry"))?;
                    output.push(finish(builder)?);
                }
            }
            Ok(Event::DocType(_)) => return Err(format_error("rss-doctype")),
            Ok(Event::Eof) => break,
            Err(_) => return Err(format_error("rss-xml")),
            _ => {}
        }
    }
    if current.is_some() || !stack.is_empty() || feed_kind.is_none() {
        return Err(format_error("rss-incomplete"));
    }
    Ok(output)
}

fn assign_text(
    builder: &mut CandidateBuilder,
    stack: &[Vec<u8>],
    text: &str,
) -> Result<(), AppError> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(format_error("rss-text-limit"));
    }
    let relative = stack
        .get(builder.entry_index..)
        .ok_or_else(|| format_error("rss-entry-path"))?;
    let Some(kind) = builder.kind else {
        return Err(format_error("rss-entry-kind"));
    };
    let path = relative
        .iter()
        .map(|value| feed_local_name(kind, builder.namespace_prefix.as_deref(), value))
        .collect::<Option<Vec<_>>>();
    let Some(path) = path else {
        return Ok(());
    };
    match (builder.kind, path.as_slice()) {
        (Some(FeedKind::Rss), [b"item", b"guid"]) | (Some(FeedKind::Atom), [b"entry", b"id"]) => {
            append(&mut builder.stable_id, text)?;
        }
        (_, [b"item" | b"entry", b"title"]) => append(&mut builder.title, text)?,
        (Some(FeedKind::Rss), [b"item", b"link"]) => {
            append(&mut builder.original_url, text)?;
        }
        (Some(FeedKind::Rss), [b"item", b"author"])
        | (Some(FeedKind::Atom), [b"entry", b"author", b"name"]) => {
            append(&mut builder.author, text)?;
        }
        (Some(FeedKind::Rss), [b"item", b"description"])
        | (Some(FeedKind::Atom), [b"entry", b"summary" | b"content"]) => {
            append(&mut builder.summary, text)?;
        }
        (Some(FeedKind::Rss), [b"item", b"pubDate"])
        | (Some(FeedKind::Atom), [b"entry", b"published"]) => {
            append(&mut builder.published_at, text)?;
        }
        (Some(FeedKind::Atom), [b"entry", b"updated"]) => {
            append(&mut builder.updated_at, text)?;
        }
        _ => {}
    }
    Ok(())
}

fn append(target: &mut Option<String>, text: &str) -> Result<(), AppError> {
    let value = target.get_or_insert_with(String::new);
    if value.len().saturating_add(text.len()) > MAX_TEXT_BYTES {
        return Err(format_error("rss-text-limit"));
    }
    value.push_str(text);
    Ok(())
}

fn finish(mut builder: CandidateBuilder) -> Result<RawSourceCandidate, AppError> {
    builder.stable_id = clean(builder.stable_id.as_deref());
    builder.original_url = canonicalize_item_url(builder.original_url.as_deref())?;
    let stable_external_id = if let Some(id) = builder.stable_id.take() {
        id
    } else if let Some(url) = builder.original_url.as_deref() {
        format!("url:{}", hex_digest(url.as_bytes()))
    } else {
        return Err(format_error("rss-missing-identity"));
    };
    if stable_external_id.len() > 512 {
        return Err(format_error("rss-id-limit"));
    }
    builder.title = clean(builder.title.as_deref());
    builder.author = clean(builder.author.as_deref());
    builder.summary = clean(builder.summary.as_deref());
    let (published_at, published_warning) =
        clean_valid_time("published_at", builder.published_at.as_deref());
    let (updated_at, updated_warning) =
        clean_valid_time("updated_at", builder.updated_at.as_deref());
    builder.published_at = published_at;
    builder.updated_at = updated_at;
    let warnings = [published_warning, updated_warning]
        .into_iter()
        .flatten()
        .collect();
    let hash_input = serde_json::to_vec(&(
        &builder.title,
        &builder.original_url,
        &builder.author,
        &builder.summary,
        &builder.published_at,
        &builder.updated_at,
    ))
    .map_err(|_| format_error("rss-hash-input"))?;
    Ok(RawSourceCandidate {
        stable_external_id,
        title: builder.title,
        original_url: builder.original_url,
        author: builder.author,
        summary: builder.summary,
        published_at: builder.published_at,
        updated_at: builder.updated_at,
        content_hash: hex_digest(&hash_input),
        warnings,
    })
}

fn clean(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn clean_valid_time(
    field: &str,
    value: Option<&str>,
) -> (Option<String>, Option<SourceFieldWarning>) {
    let Some(value) = clean(value) else {
        return (None, None);
    };
    if crate::contracts::effects::normalize_rfc3339_utc(&value).is_some()
        || crate::infrastructure::http::source_http_policy::parse_imf_fixdate_epoch_seconds(&value)
            .is_some()
    {
        (Some(value), None)
    } else {
        (
            None,
            Some(SourceFieldWarning {
                field: field.to_owned(),
                code: "source.invalid_optional_time".to_owned(),
            }),
        )
    }
}

fn canonicalize_item_url(value: Option<&str>) -> Result<Option<String>, AppError> {
    let Some(value) = clean(value) else {
        return Ok(None);
    };
    let mut url = Url::parse(&value).map_err(|_| format_error("rss-item-url"))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(format_error("rss-item-url-policy"));
    }
    if url.port() == Some(443) {
        url.set_port(None)
            .map_err(|()| format_error("rss-item-url-port"))?;
    }
    url.set_fragment(None);
    Ok(Some(url.to_string()))
}

fn validate_root(
    event: &BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
) -> Result<(FeedKind, Option<Vec<u8>>), AppError> {
    let qualified_name = event.name();
    let raw_name = qualified_name.as_ref();
    match raw_name {
        b"rss" => {
            let mut version_two = false;
            for attribute in event.attributes().with_checks(true) {
                let attribute = attribute.map_err(|_| format_error("rss-root-attribute"))?;
                if local_name(attribute.key.as_ref()) == b"version" {
                    let value = attribute
                        .decoded_and_normalized_value(XmlVersion::Implicit1_0, decoder)
                        .map_err(|_| format_error("rss-root-version"))?;
                    version_two = value.trim() == "2.0";
                }
            }
            version_two
                .then_some((FeedKind::Rss, None))
                .ok_or_else(|| format_error("rss-root-version"))
        }
        b"feed" => {
            let mut atom_namespace = false;
            for attribute in event.attributes().with_checks(true) {
                let attribute = attribute.map_err(|_| format_error("atom-root-attribute"))?;
                if attribute.key.as_ref() == b"xmlns" {
                    let value = attribute
                        .decoded_and_normalized_value(XmlVersion::Implicit1_0, decoder)
                        .map_err(|_| format_error("atom-root-namespace"))?;
                    atom_namespace |= value.as_bytes() == ATOM_NAMESPACE;
                }
            }
            atom_namespace
                .then_some((FeedKind::Atom, None))
                .ok_or_else(|| format_error("atom-root-namespace"))
        }
        _ => {
            let Some((prefix, local)) = split_prefix(raw_name) else {
                return Err(format_error("rss-root"));
            };
            if local != b"feed" {
                return Err(format_error("rss-root"));
            }
            let expected_attribute = [b"xmlns:".as_slice(), prefix].concat();
            let mut atom_namespace = false;
            for attribute in event.attributes().with_checks(true) {
                let attribute = attribute.map_err(|_| format_error("atom-root-attribute"))?;
                if attribute.key.as_ref() == expected_attribute {
                    let value = attribute
                        .decoded_and_normalized_value(XmlVersion::Implicit1_0, decoder)
                        .map_err(|_| format_error("atom-root-namespace"))?;
                    atom_namespace = value.as_bytes() == ATOM_NAMESPACE;
                }
            }
            atom_namespace
                .then_some((FeedKind::Atom, Some(prefix.to_vec())))
                .ok_or_else(|| format_error("atom-root-namespace"))
        }
    }
}

fn is_valid_entry_path(kind: FeedKind, stack: &[Vec<u8>]) -> bool {
    let prefix = stack
        .first()
        .and_then(|name| split_prefix(name).map(|(prefix, _)| prefix));
    let path = stack
        .iter()
        .map(|value| feed_local_name(kind, prefix, value))
        .collect::<Option<Vec<_>>>();
    matches!(
        (kind, path.as_deref()),
        (FeedKind::Rss, Some([b"rss", b"channel", b"item"]))
            | (FeedKind::Atom, Some([b"feed", b"entry"]))
    )
}

fn reject_namespace_rebinding(event: &BytesStart<'_>) -> Result<(), AppError> {
    for attribute in event.attributes().with_checks(true) {
        let attribute = attribute.map_err(|_| format_error("rss-namespace-attribute"))?;
        if attribute.key.as_ref() == b"xmlns" || attribute.key.as_ref().starts_with(b"xmlns:") {
            return Err(format_error("rss-namespace-rebinding"));
        }
    }
    Ok(())
}

fn feed_local_name<'a>(
    kind: FeedKind,
    expected_prefix: Option<&[u8]>,
    name: &'a [u8],
) -> Option<&'a [u8]> {
    match (kind, expected_prefix, split_prefix(name)) {
        (FeedKind::Rss | FeedKind::Atom, None, None) => Some(name),
        (FeedKind::Atom, Some(expected), Some((actual, local))) if actual == expected => {
            Some(local)
        }
        _ => None,
    }
}

fn split_prefix(name: &[u8]) -> Option<(&[u8], &[u8])> {
    let separator = name.iter().position(|byte| *byte == b':')?;
    Some((&name[..separator], &name[separator + 1..]))
}

fn assign_link_attributes(
    current: &mut Option<CandidateBuilder>,
    event: &BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
) -> Result<(), AppError> {
    let Some(builder) = current
        .as_mut()
        .filter(|builder| builder.kind == Some(FeedKind::Atom))
    else {
        return Ok(());
    };
    let mut href = None;
    let mut rel = None;
    for attribute in event.attributes().with_checks(true) {
        let attribute = attribute.map_err(|_| format_error("rss-attribute"))?;
        let value = attribute
            .decoded_and_normalized_value(XmlVersion::Implicit1_0, decoder)
            .map_err(|_| format_error("rss-link"))?;
        match local_name(attribute.key.as_ref()) {
            b"href" => href = clean(Some(&value)),
            b"rel" => rel = clean(Some(&value)),
            _ => {}
        }
    }
    let priority = match rel.as_deref() {
        None | Some("alternate") => 2,
        _ => 0,
    };
    if priority > builder.link_priority && href.is_some() {
        builder.original_url = href;
        builder.link_priority = priority;
    }
    Ok(())
}

fn validate_declaration_encoding(
    declaration: &quick_xml::events::BytesDecl<'_>,
) -> Result<(), AppError> {
    let Some(encoding) = declaration.encoding() else {
        return Ok(());
    };
    let encoding = encoding.map_err(|_| format_error("rss-encoding-declaration"))?;
    if encoding.eq_ignore_ascii_case(b"utf-8") || encoding.eq_ignore_ascii_case(b"us-ascii") {
        Ok(())
    } else {
        Err(format_error("rss-encoding"))
    }
}

fn local_name(name: &[u8]) -> &[u8] {
    name.rsplit(|byte| *byte == b':').next().unwrap_or(name)
}

fn has_unsupported_encoding(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff])
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::with_capacity(64), |mut output, byte| {
            write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
            output
        })
}

fn format_error(boundary: &'static str) -> AppError {
    AppError::from_code(ErrorCode::SourceFormatRssAtom, boundary)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::parse_feed_until;

    #[test]
    fn expired_deadline_aborts_before_parsing() {
        let deadline = Instant::now()
            .checked_sub(Duration::from_millis(1))
            .expect("representable expired deadline");
        let error = parse_feed_until(b"<rss version=\"2.0\"><channel/></rss>", Some(deadline))
            .expect_err("expired parsing budget must fail closed");
        assert_eq!(error.code(), "source_format.rss_atom");
        assert!(error.details_allowlisted().is_empty());
    }
}
