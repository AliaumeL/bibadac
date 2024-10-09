/// Manages arxiv identifiers
/// and their associated usage.
///
use std::fmt::{self, Display, Formatter};

#[derive(Debug, PartialEq, Eq)]
pub struct ArxivId<'a> {
    pub id: &'a str,
    pub version: Option<usize>,
}

impl PartialOrd for ArxivId<'_> {
    // ids should be equal AND versions should be comparable if they
    // exist (None > everything else)
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.id.cmp(other.id) {
            std::cmp::Ordering::Equal => match (self.version, other.version) {
                (Some(a), Some(b)) => Some(a.cmp(&b)),
                (Some(_), None) => Some(std::cmp::Ordering::Greater),
                (None, Some(_)) => Some(std::cmp::Ordering::Less),
                (None, None) => Some(std::cmp::Ordering::Equal),
            },
            _ => None,
        }
    }
}

fn parse_arxiv_id<'a>(s: &'a str) -> Option<ArxivId<'a>> {
    let last_v = s.rfind("v");
    let (id, version) = match last_v {
        Some(v) => {
            let (id, version) = s.split_at(v);
            (id, version[1..].parse().ok().map(|v| Some(v)))
        }
        None => (s, Some(None)),
    };
    Some(ArxivId {
        id,
        version: version?,
    })
}

impl<'a> TryFrom<&'a str> for ArxivId<'a> {
    type Error = ();
    fn try_from(s: &'a str) -> Result<Self, ()> {
        parse_arxiv_id(s).ok_or(())
    }
}

impl<'a> Display for ArxivId<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.version {
            Some(v) => write!(f, "{}v{}", self.id, v),
            None => write!(f, "{}", self.id),
        }
    }
}

impl ArxivId<'_> {
    pub fn to_string(&self) -> String {
        match self.version {
            Some(v) => format!("{}v{}", self.id, v),
            None => self.id.to_string(),
        }
    }

    pub fn to_abstract_url(&self) -> String {
        format!("https://arxiv.org/abs/{}", self.id)
    }

    pub fn to_pdf_url(&self) -> String {
        format!("https://arxiv.org/pdf/{}", self.id)
    }
}
