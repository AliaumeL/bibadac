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
use reqwest::blocking::Client;

pub enum DownloadRequest<'a> {
    Arxiv(ArxivId<'a>),
    Doi(&'a str),
    Url(&'a str),
}

pub trait DownloadHandler {
    fn can_handle(&self, request: &DownloadRequest) -> bool;
    fn download(&self, request: &[DownloadRequest]) -> Vec<Option<String>>;
}

pub struct ArxivDownloader {}

impl DownloadHandler for ArxivDownloader {
    fn can_handle(&self, request: &DownloadRequest) -> bool {
        match request {
            DownloadRequest::Arxiv(_) => true,
            _ => false,
        }
    }

    fn download(&self, request: &[DownloadRequest]) -> Vec<Option<String>> {
        let client = Client::new();
        request
            .iter()
            .map(|r| match r {
                DownloadRequest::Arxiv(id) => {
                    eprintln!("Downloading arxiv paper {}", id);
                    let url = id.to_pdf_url();
                    eprintln!("URL: {}", url);
                    let response = client.get(url).send();
                    eprintln!("Response: {:?}", response);
                    match response {
                        Ok(mut response) => {
                            let mut buf = Vec::new();
                            response.copy_to(&mut buf).ok()?;
                            Some(String::from_utf8(buf).ok()?)
                        }
                        Err(_) => None,
                    }
                }
                _ => None,
            })
            .collect()
    }
}
