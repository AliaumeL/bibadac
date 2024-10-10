/// This file is used to "set-up" a working environment
/// from a bibtex file. This means downoalding as much
/// as possible the pdfs that are mentionned in the file.
// 1 - How to store a PDF (hash)
//
// 2 - Download an arxiv paper (batch mode possible)
//
// 3 - Download a DOI paper (batch mode not possible)
//
// 4 - Use a caching mechanism to avoid downloading the same file
// twice.
//
use crate::arxiv_identifiers::ArxivId;
use reqwest::Client;

#[derive(Debug)]
pub enum DownloadRequest<'a> {
    Arxiv(ArxivId<'a>),
    Doi(&'a str),
    Url(&'a str),
}

pub trait DownloadHandler<T : Fn(&str) -> ()> {
    fn can_handle(&self, request: &DownloadRequest) -> bool;
    async fn download<'a>(&self, request: &[DownloadRequest<'a>], progress: T) -> Vec<Option<String>>;
}

#[derive(Default)]
pub struct ArxivDownloader  {
    client: Client
}

pub struct DxDoiDownloader  {
    client: Client
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
            reqwest::header::HeaderValue::from_static("ad.lopez@uw.edu.pl")
            );

        let client = reqwest::Client::builder()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .default_headers(headers)
            .build().expect("Could not build http client");

        DxDoiDownloader { client }
    }
}


#[derive(Default)]
pub struct ScihubDownloader {
    client: Client
}


impl DxDoiDownloader {
    pub fn new() -> Self {
        DxDoiDownloader::default()
    }

    async fn download_one<'a>(&self, request: &DownloadRequest<'a>) -> Option<String> {
        if let DownloadRequest::Doi(doi) = request {
            let url = format!("https://dx.doi.org/{}", doi);
            let response = self.client.get(url).send().await.ok()?;
            let text     = response.text_with_charset("utf-8").await.ok()?;
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
    fn new() -> Self {
        ArxivDownloader::default()
    }

    async fn download_one<'a>(&self, request: &DownloadRequest<'a>) -> Option<String> {
        if let DownloadRequest::Arxiv(id) = request {
            eprintln!("Downloading arxiv paper {}", id);
            let url = id.to_pdf_url();
            eprintln!("URL: {}", url);
            let response = self.client.get(url).send().await.ok()?;
            eprintln!("Response: {:?}", response);
            let _ = response.bytes().await.ok()?;
            Some("<PDF DATA>".to_string())
        } else {
            None
        }
    }
}

impl<T> DownloadHandler<T> for DxDoiDownloader
where T : Fn(&str) -> ()
{
    fn can_handle(&self, request: &DownloadRequest) -> bool {
        match request {
            DownloadRequest::Doi(_) => true,
            _ => false,
        }
    }

    async fn download<'a>(&self, request: &[DownloadRequest<'a>], progress: T) -> Vec<Option<String>> {
        use futures::stream::{self, StreamExt};
        let res = stream::iter(request
            .iter()
            .map(|r| { progress(&format!("{:?}", r)); self.download_one(r) }))
            .buffer_unordered(5)
            .collect()
            .await;

        res
    }

}

impl<T> DownloadHandler<T> for ArxivDownloader
where T : Fn(&str) -> ()
{
    fn can_handle(&self, request: &DownloadRequest) -> bool {
        match request {
            DownloadRequest::Arxiv(_) => true,
            _ => false,
        }
    }

    async fn download<'a>(&self, request: &[DownloadRequest<'a>], progress: T) -> Vec<Option<String>> {
        use futures::stream::{self, StreamExt};
        let res = stream::iter(request
            .iter()
            .map(|r| { progress(&format!("{:?}", r)); self.download_one(r) }))
            .buffer_unordered(5)
            .collect()
            .await;
        res
    }
}
