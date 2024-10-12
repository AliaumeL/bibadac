/// This file is used to "set-up" a working environment
/// from a bibtex file. This means downoalding as much
/// as possible the pdfs that are mentionned in the file.


/// TODO: change the way we handle downloads,
/// the "trait" is useless (we use one client per request)
/// -> we should be directly taking &str as input
/// -> we should be returning 
///     - the corresponding bibentry 
///     - the path of the pdf (if it makes sense)
///     - the sha256 of the pdf (if it makes sense)
/// -> this way we can provide a way nicer interface 
///     - list dois that failed to download
///     - list pdfs that failed to download

use colored::Colorize;
use crate::arxiv_identifiers::ArxivId;
use reqwest::Client;
use std::sync::OnceLock;
use std::collections::{HashMap,HashSet};
use crate::bibtex::BibFile;

// typical url
// type="application/pdf" src="//zero.sci-hub.se/407/de27ca7d3dc4c4fddd8bac961171940d/kirsten2002.pdf#
fn sci_hub_pdf_regex() -> &'static regex::Regex {
    static INIT: OnceLock<regex::Regex> = OnceLock::new();
    INIT.get_or_init(|| regex::Regex::new(r"(src=.)([\/A-Za-z0-9\.-]+)(\.pdf)").unwrap())
}



#[derive(Debug, Clone, Default)]
pub struct SetupConfig {
    // existing identifiers in the "database"
    pub existing_sha256: HashSet<String>,
    pub existing_arxiv: HashSet<String>,
    pub existing_doi: HashSet<String>,
    // exiting mappings in the "database"
    pub arxiv_to_sha256: HashMap<String, String>,
    pub doi_to_sha256: HashMap<String, String>,
    // option flags
    pub progress: bool,
    pub download_pdf: bool,
    pub dry_run: bool, 
    pub working_directory: std::path::PathBuf,
    pub polite_email: Option<String>,
}

#[derive(Debug)]
pub struct PdfResult {
    pub filepath : std::path::PathBuf,
    pub sha256   : String,
    pub entry    : String,
}

#[derive(Debug)]
pub struct SetupResult {
    pub pdfs   : Vec<(String,Option<PdfResult>)>,
    pub entries: Vec<(String,Option<String>)>,
}

impl SetupConfig {
    pub fn new() -> Self {
        SetupConfig::default()
    }

    pub fn already_present(&self, request: &DownloadRequest) -> bool {
        // Tries to see if the corresponding pdf is already present
        // 1. matches the request to a sha256
        // 2 TODO: checks if the sha256 *really exists*
        match request {
            DownloadRequest::Arxiv(id) => {
                if let Some(_) = self.arxiv_to_sha256.get(id.id) {
                    true
                } else {
                    false
                }
            }
            DownloadRequest::Doi(doi) => {
                if let Some(_) = self.doi_to_sha256.get(*doi) {
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    pub fn import_bibfile(&mut self, path: &std::path::PathBuf) {
        let start_bib = std::fs::read_to_string(path).expect("Could not read the output bibfile");
        let bibtex = BibFile::new(&start_bib);
        for entry in bibtex.list_entries() {
            let mut doi = None;
            let mut eprint = None;
            let mut sha256 = None;
            for field in entry.fields.iter() {
                let key = bibtex.get_slice(field.name).to_lowercase();
                let value = bibtex.get_braceless_slice(field.value);
                match key.as_str() {
                    "doi" => { doi = Some(value.to_string()); self.existing_doi.insert(value.to_string()); }
                    "eprint" => { eprint = Some(value.to_string()); self.existing_arxiv.insert(value.to_string()); }
                    "sha256" => { sha256 = Some(value.to_string()); self.existing_sha256.insert(value.to_string()); }
                    _ => {}
                }
            }
            if let (Some(doi), Some(sha256)) = (doi, &sha256) {
                self.doi_to_sha256.insert(doi, sha256.clone());
            }
            if let (Some(eprint), Some(sha256)) = (eprint, sha256) {
                self.arxiv_to_sha256.insert(eprint, sha256);
            }
        }
    }

    pub async fn run(&self, dois    : HashSet<String>,
                            eprints : HashSet<String>,
                            _sha256s: HashSet<String>) -> SetupResult {

        use futures::stream::{self, StreamExt};

        let doi_requests: Vec<_> = dois.iter()
            .filter(|d| !self.existing_doi.contains(*d))
            .map(|d| DownloadRequest::Doi(d)).collect();

        let arxiv_requests : Vec<_> = eprints.iter()
            .filter(|d| !self.existing_arxiv.contains(*d))
            .filter_map(|d| {
                Some(DownloadRequest::Arxiv(ArxivId::try_from(d.as_str()).ok()?))
            }).collect();

        let pdf_requests: Vec<_> = dois
            .iter()
            .map(|d| DownloadRequest::Doi(d))
            .chain(eprints.iter()
                .filter_map(|d| {
                Some(DownloadRequest::Arxiv(ArxivId::try_from(d.as_str()).ok()?))
            }))
            .filter(|r| !self.already_present(r))
            .collect();

        let doi_downloader = DxDoiDownloader::new(self.polite_email.clone());
        let epr_downloader = ArxivDownloader::new();
        let pdf_downloader = PdfDownloader::new(self.working_directory.clone());

        if self.progress {
            println!("{:<10}\t{} dois / {} eprints / {} pdfs", 
                     "[TOTAL]".blue(), 
                     doi_requests.len(),
                     arxiv_requests.len(),
                     pdf_requests.len());
        }

        let mut res = vec![];

        let res_doi  = doi_downloader.download(&doi_requests, |url| {
            if self.progress {
                println!("{:<10}\t{}", "[BIBTEX]".green(),  url);
            }
        }).await;

        let res_eprint = epr_downloader.download(&arxiv_requests, |url| {
            if self.progress {
                println!("{:<10}\t{}", "[BIBTEX]".green(),  url);
            }
        }).await;
        res.extend(res_doi);
        res.extend(res_eprint);

        let count = res.iter().filter(|r| r.is_some()).count();

        if self.progress {
            println!("{:<10}\t{} / {} entries retrieved", "[TOTAL BIB]".blue(), count, dois.len() + eprints.len());
        }

        if !self.download_pdf {
            return SetupResult { pdfs: vec![], 
                entries: res.into_iter()
                    .zip(dois.iter().chain(eprints.iter()))
                    .map(|(r, d)| (d.clone(), r))
                    .collect() };
        }
        
        // FIXME: avoid duplicate downloads
        // by checking if the pdfs already are present in the current 
        // working directory

        let pdfs : Vec<Option<PdfResult>> = stream::iter(pdf_requests.iter().map(|r| {
                if self.progress {
                    println!("{:<10}\t{}", "[PDF]".green(), r);
                }
                pdf_downloader.download_one_pdf(r)
            }))
            .buffer_unordered(5)
            .collect()
            .await;

        let pdf_count = pdfs.iter().filter(|r| r.is_some()).count();

        if self.progress {
            println!("{:<10}\t{} / {}", "[TOTAL PDF]".blue(), pdf_count, pdf_requests.len());
        }

         SetupResult { 
            pdfs: pdf_requests.into_iter()
                .zip(pdfs.into_iter())
                .map(|(r, p)| (format!("{}", r), p))
                .collect(),
            entries: res.into_iter()
                .zip(dois.iter().chain(eprints.iter()))
                .map(|(r, d)| (d.clone(), r))
                .collect()
         }
    }
}



#[derive(Debug)]
pub enum DownloadRequest<'a> {
    Arxiv(ArxivId<'a>),
    Doi(&'a str),
    Url(&'a str),
}

impl std::fmt::Display for DownloadRequest<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadRequest::Arxiv(id) => write!(f, "arxiv:{}", id),
            DownloadRequest::Doi(doi) => write!(f, "doi:{}", doi),
            DownloadRequest::Url(url) => write!(f, "url:{}", url),
        }
    }
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
    cwd: std::path::PathBuf,
}


impl Default for DxDoiDownloader {
    fn default() -> Self {
        DxDoiDownloader::new(None)
    }
}

impl DxDoiDownloader {
    pub fn new(polite_email : Option<String>) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            "Accept",
            reqwest::header::HeaderValue::from_static("application/x-bibtex"),
        );
        if let Some(email) = polite_email {
            headers.insert(
                "Mailto",
                reqwest::header::HeaderValue::from_str(&email).expect("Could not parse email"),
            );
        }

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

    async fn download_one<'a>(&self, request: &DownloadRequest<'a>) -> Option<String> {
        if let DownloadRequest::Doi(doi) = request {
            let url = format!("https://dx.doi.org/{}", doi);
            let response = self.client.get(url).send().await.ok()?;
            let text = response.text_with_charset("utf-8").await.ok()?;
            if text.starts_with(" @") {
                Some(text[1..].to_string())
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
            let _ = response.text_with_charset("utf-8").await.ok()?;
            // TODO: parse
            Some("<PDF DATA>".to_string())
        } else {
            None
        }
    }
}

impl PdfDownloader {
    pub fn new(working_directory : std::path::PathBuf) -> Self {
        PdfDownloader { client: Client::default(), cwd: working_directory }
    }

    async fn download_one_pdf<'a>(&self, request: &DownloadRequest<'a>) -> Option<PdfResult> {
        use sha2::Digest;
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
            format!("{}", request)
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

        let filename = self.cwd.join(filename);
        let mut file = std::fs::File::create(&filename).ok()?;
        file.write_all(&pdf_bytes).ok()?;
        let sha256 = format!("{:x}", sha2::Sha256::digest(&pdf_bytes));

        let short_sha = &sha256[..10];
        let display_file = filename.display();
        let identifier_value = match request {
            DownloadRequest::Arxiv(id) => id.to_string(),
            DownloadRequest::Doi(doi) => doi.to_string(),
            DownloadRequest::Url(url) => url.to_string(),
        };
        let identifier_mapping = format!("@mapping{{{short_sha}:{request}, sha256 = {{{sha256}}}, filename = {{{display_file}}}, {mode} = {{{identifier_value}}}}}",
            mode = match request {
                DownloadRequest::Arxiv(_) => "eprint",
                DownloadRequest::Doi(_) => "doi",
                DownloadRequest::Url(_) => "url",
            });

        Some(PdfResult { filepath: filename, sha256, entry: identifier_mapping })
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

        let filename = self.cwd.join(filename);
        let mut file = std::fs::File::create(&filename).ok()?;
        file.write_all(&pdf_bytes).ok()?;
        Some(filename.to_str()?.to_string())
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
            progress(&format!("{}", r));
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
            progress(&format!("{}", r));
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
            progress(&format!("{}", r));
            self.download_one(r)
        }))
        .buffer_unordered(5)
        .collect()
        .await;
        res
    }
}
