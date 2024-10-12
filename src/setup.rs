/// This file is used to "set-up" a working environment
/// from a bibtex file. This means downoalding as much
/// as possible the pdfs that are mentionned in the file.


/// TODO: 
/// download PDFs and create entries of the form 
///
/// @mapping{arxiv:sha:<arxivId>,
///   sha256 = {<sha256>},
///   eprint = {<arxivId>},
/// }
///
/// @mapping{doi:sha:<doi>,
///    sha256 = {<sha256>},
///    doi    = {<doi>},
/// }
///
/// These can then be used to autocomplete entries.
///
/// 1. Download the PDFs (using arxiv / scihub [compile flag])
/// 2. Compute the sha256 of the PDFs
/// 3. Create the entries
///

use crate::arxiv_identifiers::ArxivId;
use reqwest::Client;
use std::sync::OnceLock;

// typical url
// type="application/pdf" src="//zero.sci-hub.se/407/de27ca7d3dc4c4fddd8bac961171940d/kirsten2002.pdf#
fn sci_hub_pdf_regex() -> &'static regex::Regex {
    static INIT: OnceLock<regex::Regex> = OnceLock::new();
    INIT.get_or_init(|| regex::Regex::new(r"(src=.)([\/A-Za-z0-9\.-]+)(\.pdf)").unwrap())
}

#[derive(Debug)]
pub enum DownloadRequest<'a> {
    Arxiv(ArxivId<'a>),
    Doi(&'a str),
    Url(&'a str),
}

#[allow(async_fn_in_trait)]
pub trait DownloadHandler<T: Fn(&str) -> ()> {
    fn can_handle(&self, request: &DownloadRequest) -> bool;
    async fn download<'a>(
        &self,
        request: &[DownloadRequest<'a>],
        progress: T,
    ) -> Vec<Option<String>>;
}

#[derive(Default)]
pub struct ArxivDownloader {
    client: Client,
}

pub struct DxDoiDownloader {
    client: Client,
}

#[derive(Default)]
pub struct PdfDownloader {
    client: Client,
}

impl Default for DxDoiDownloader {
    fn default() -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Accept",
            reqwest::header::HeaderValue::from_static("application/x-bibtex"),
        );
        headers.insert(
            "Mailto",
            reqwest::header::HeaderValue::from_static("ad.lopez@uw.edu.pl"),
        );

        let client = reqwest::Client::builder()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .default_headers(headers)
            .build()
            .expect("Could not build http client");

        DxDoiDownloader { client }
    }
}

impl DxDoiDownloader {
    pub fn new() -> Self {
        DxDoiDownloader::default()
    }

    async fn download_one<'a>(&self, request: &DownloadRequest<'a>) -> Option<String> {
        if let DownloadRequest::Doi(doi) = request {
            let url = format!("https://dx.doi.org/{}", doi);
            let response = self.client.get(url).send().await.ok()?;
            let text = response.text_with_charset("utf-8").await.ok()?;
            if text.starts_with(" @") {
                Some(text)
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl ArxivDownloader {
    pub fn new() -> Self {
        ArxivDownloader::default()
    }

    // We download the direct feed from the arxiv API
    // -> we use an rss parser to extract a "bibtex entry"
    // -> we output the bibtex entry
    // @misc{<arxivId>,
    //  title = {<title>},
    //  author = {<author>},
    //  year = {<year>},
    //  abstract = {<abstract>},
    //  archivePrefix = {arXiv},
    //  eprint = {<arxivId>},
    //  primaryClass = {<primaryClass>},
    //  }
    async fn download_one<'a>(&self, request: &DownloadRequest<'a>) -> Option<String> {
        if let DownloadRequest::Arxiv(id) = request {
            let url = id.to_api_url();
            let response = self.client.get(url).send().await.ok()?;
            println!("{:?}", response);
            let _ = response.text_with_charset("utf-8").await.ok()?;
            // TODO: parse
            Some("<PDF DATA>".to_string())
        } else {
            None
        }
    }
}

impl PdfDownloader {
    pub fn new() -> Self {
        PdfDownloader::default()
    }

    async fn download_one<'a>(&self, request: &DownloadRequest<'a>) -> Option<String> {
        use std::io::Write;
        let pdf_url: String = match request {
            DownloadRequest::Arxiv(id) => id.to_pdf_url(),
            DownloadRequest::Doi(doi) => {
                // using scihub
                let url = format!("https://sci-hub.se/{}", doi);
                let page = self.client.get(url).send().await.ok()?.text().await.ok()?;
                let pdf_stub = sci_hub_pdf_regex().captures(&page)?.get(2)?.as_str();
                format!("https:{}.pdf", pdf_stub)
            }
            DownloadRequest::Url(url) => url.to_string(),
        };
        let response = self.client.get(pdf_url).send().await.ok()?;
        let pdf_bytes = response.bytes().await.ok()?;
        let filename = format!(
            "{}.pdf",
            format!("{:?}", request)
                .to_ascii_lowercase()
                .replace(" ", "_")
                .replace("(", "_")
                .replace(")", "_")
                .replace("/", "_")
                .replace(":", "_")
                .replace("?", "_")
                .replace("=", "_")
                .replace("&", "_")
                .replace("'", "_")
                .replace("{", "_")
                .replace("}", "_")
                .replace(",", "_")
                .replace("\"", "_")
                .replace(".", "_")
        );

        let mut file = std::fs::File::create(&filename).ok()?;
        file.write_all(&pdf_bytes).ok()?;
        Some(filename)
    }
}

impl<T> DownloadHandler<T> for PdfDownloader
where
    T: Fn(&str) -> (),
{
    fn can_handle(&self, _: &DownloadRequest) -> bool {
        true
    }

    async fn download<'a>(
        &self,
        request: &[DownloadRequest<'a>],
        progress: T,
    ) -> Vec<Option<String>> {
        use futures::stream::{self, StreamExt};
        let res = stream::iter(request.iter().map(|r| {
            progress(&format!("{:?}", r));
            self.download_one(r)
        }))
        .buffer_unordered(5)
        .collect()
        .await;
        res
    }
}

impl<T> DownloadHandler<T> for DxDoiDownloader
where
    T: Fn(&str) -> (),
{
    fn can_handle(&self, request: &DownloadRequest) -> bool {
        match request {
            DownloadRequest::Doi(_) => true,
            _ => false,
        }
    }

    async fn download<'a>(
        &self,
        request: &[DownloadRequest<'a>],
        progress: T,
    ) -> Vec<Option<String>> {
        use futures::stream::{self, StreamExt};
        let res = stream::iter(request.iter().map(|r| {
            progress(&format!("{:?}", r));
            self.download_one(r)
        }))
        .buffer_unordered(5)
        .collect()
        .await;

        res
    }
}

impl<T> DownloadHandler<T> for ArxivDownloader
where
    T: Fn(&str) -> (),
{
    fn can_handle(&self, request: &DownloadRequest) -> bool {
        match request {
            DownloadRequest::Arxiv(_) => true,
            _ => false,
        }
    }

    async fn download<'a>(
        &self,
        request: &[DownloadRequest<'a>],
        progress: T,
    ) -> Vec<Option<String>> {
        use futures::stream::{self, StreamExt};
        let res = stream::iter(request.iter().map(|r| {
            progress(&format!("{:?}", r));
            self.download_one(r)
        }))
        .buffer_unordered(5)
        .collect()
        .await;
        res
    }
}
