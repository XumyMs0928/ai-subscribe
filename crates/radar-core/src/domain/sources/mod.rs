//! Bounded source adapter value objects.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use ipnet::IpNet;
use serde::{Deserialize, Serialize};

/// Returns whether an IP literal is globally routable under the shared source policy.
#[must_use]
pub fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => is_public_v4(ip),
        IpAddr::V6(ip) => ip
            .to_ipv4_mapped()
            .map_or_else(|| is_public_v6(ip), is_public_v4),
    }
}

fn in_net(ip: IpAddr, cidr: &str) -> bool {
    cidr.parse::<IpNet>()
        .is_ok_and(|network| network.contains(&ip))
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let value = IpAddr::V4(ip);
    ![
        "0.0.0.0/8",
        "10.0.0.0/8",
        "100.64.0.0/10",
        "127.0.0.0/8",
        "169.254.0.0/16",
        "172.16.0.0/12",
        "192.0.0.0/24",
        "192.0.2.0/24",
        "192.168.0.0/16",
        "198.18.0.0/15",
        "198.51.100.0/24",
        "203.0.113.0/24",
        "224.0.0.0/4",
        "240.0.0.0/4",
    ]
    .iter()
    .any(|network| in_net(value, network))
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    let value = IpAddr::V6(ip);
    ![
        "::/128",
        "::1/128",
        "64:ff9b::/96",
        "64:ff9b:1::/48",
        "100::/64",
        "fc00::/7",
        "fe80::/10",
        "fec0::/10",
        "ff00::/8",
        "2001::/23",
        "2001:db8::/32",
        "2002::/16",
        "3fff::/20",
        "5f00::/16",
    ]
    .iter()
    .any(|network| in_net(value, network))
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RawSourceCandidate {
    pub stable_external_id: String,
    pub title: Option<String>,
    pub original_url: Option<String>,
    pub author: Option<String>,
    pub summary: Option<String>,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub content_hash: String,
    pub warnings: Vec<SourceFieldWarning>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceFieldWarning {
    pub field: String,
    pub code: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchIncrementalResult {
    pub candidates: Vec<RawSourceCandidate>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub adapter_cursor: Option<String>,
    pub not_modified: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateDisposition {
    New,
    Changed,
    Unchanged,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CandidateApplyResult {
    pub stable_external_id: String,
    pub disposition: CandidateDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceFetchState {
    pub revision: u64,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub adapter_cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IncrementalFetchRequest {
    pub source_id: String,
    pub canonical_url: String,
    pub expected_revision: u64,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub adapter_cursor: Option<String>,
}
