/// This is the `bibadac` program to handle bibliographic data
/// written using the BibTeX/BibLaTeX formats.
///
/// The program contains 3 subcommands:
/// - `check`: check the validity of a BibTeX/BibLaTeX file
/// - `format`: format a BibTeX/BibLaTeX file
/// - `setup`: download pdfs that are mentionned in the file
///
use clap::{Args, Parser, Subcommand};
use std::io::Read;

use colored::Colorize;

use bibadac::bibdb::LocalBibDb;
use bibadac::bibtex::BibFile;
use bibadac::format::{write_bibfile, FormatOptions};
use bibadac::linter::{LintMessage, LinterState};

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Parser)]
#[command(name = "bibadac")]
#[command(about = "A tool to handle bibliographic data")]
struct Cli {
    #[command(subcommand)]
    command: SubCommand,
    #[arg(short, long)]
    config: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Subcommand)]
enum SubCommand {
    #[command(about = "Check the validity of a BibTeX/BibLaTeX file", arg_required_else_help = true)]
    Check(CheckArgs),
    #[command(about = "Format a BibTeX/BibLaTeX file", arg_required_else_help = true)]
    Format(FormatArgs),
    #[command(about = "Download pdfs that are mentionned in the file")]
    Setup(SetupArgs),
}

#[derive(Debug, Clone, Args)]
struct FileArgs {
    #[arg(short, long)]
    bib: Vec<std::path::PathBuf>,
    #[arg(short, long)]
    stdin: bool,
}

#[derive(Debug, Clone, Args)]
struct CheckArgs {
    #[clap(flatten)]
    files: FileArgs,
    #[arg(short, long)]
    pedantic: bool,
    #[arg(short, long)]
    verbose: bool,
    #[arg(short, long)]
    to_json: bool,
}


#[derive(Debug, Clone, Args)]
struct FormatArgs {
    #[clap(flatten)]
    files: FileArgs, 
    #[arg(short, long)]
    in_place: bool,
    #[arg(short, long)]
    file_db: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Args)]
struct SetupArgs {
    #[clap(flatten)]
    files: FileArgs,
}

#[derive(Debug, Clone)]
struct InputFile {
    name: std::path::PathBuf,
    content: String,
}

trait InputFiles {
    fn list_files(&self) -> Vec<InputFile>;
}

impl InputFiles for FileArgs {
    fn list_files(&self) -> Vec<InputFile> {
        self.bib
            .iter()
            .filter_map(|name| {
                if !name.exists() {
                    eprintln!("File {:?} does not exist", name);
                    return None;
                }
                let content = std::fs::read_to_string(name).unwrap();
                Some(InputFile {
                    name: name.clone(),
                    content,
                })
            }).chain(if self.stdin {
                let mut content = String::new();
                std::io::stdin().read_to_string(&mut content).unwrap();
                vec![InputFile {
                    name: "stdin".into(),
                    content,
                }]
            } else {
                vec![]
            }).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonReportEntry {
    file: String,
    errors: Vec<JsonReportLint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonReportLoc {
    line: usize,
    column: usize,
    start_byte: usize,
    end_byte: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonReportLint {
    msg: LintMessage,
    loc: Vec<JsonReportLoc>,
}


fn main() {
    let args = Cli::parse();

    match args.command {
        SubCommand::Check(cargs) => {
            let linter = LinterState {
                revoked_dois: Default::default(),
                arxiv_latest: Default::default(),
                doi_arxiv: Default::default(),
                arxiv_doi: Default::default(),
            };
            let files = cargs.files.list_files();
            let inputs = files.iter().map(|f| {
                let bibtex = BibFile::new(&f.content);
                (f, bibtex)
            }).collect::<Vec<_>>();
            let mut lints = vec![];
            for (bib, tex) in inputs.iter() {
                if cargs.pedantic {
                    lints.push((bib, tex, linter.lint_file(&tex, tex.list_entries().collect())));
                } else {
                    lints.push((bib, tex, 
                                linter.lint_file(&tex, tex.list_entries().collect())
                                      .into_iter()
                                      .filter(|l| l.msg.is_crucial()).collect()));
                }
            }

            if cargs.to_json {
                let mut out = std::io::stdout();
                let json_report = lints.iter()
                    .map(|(bib, _, lints)| {
                        JsonReportEntry {
                            file: bib.name.to_string_lossy().to_string(),
                            errors: lints.iter().map(|l| {
                                JsonReportLint {
                                    msg: l.msg.clone(),
                                    loc: l.loc.iter().map(|n| {
                                        JsonReportLoc {
                                            line: n.start_position().row + 1,
                                            column: n.start_position().column + 1,
                                            start_byte: n.start_byte(),
                                            end_byte: n.end_byte(),
                                        }
                                    }).collect()
                                }
                            }).collect()
                        }
                    }).collect::<Vec<_>>();
                serde_json::to_writer_pretty(&mut out, &json_report).unwrap();
                return;
            }

            // 1. print the number of errors for every input
            for (bib, _, lints) in lints.iter() {
                if lints.len() == 0 {
                    println!("{} {:?}", "[OK]".green(), bib.name);
                } else {
                    let err = if lints.len() > 1 { "errors" } else { "error" };
                    println!("{} {:?} \t {} {}", 
                             "[KO]".red(),
                             bib.name,
                             lints.len(),
                             err);
                }
            }
            // 2. print the errors for each file if verbose
            if !cargs.verbose {
                return;
            }

            for (bib, bibtex, lints) in lints.iter() {
                for l in lints {
                    println!("{}\n<{:?}:L{}:C{}>\n{:?}", 
                             "Error".red(),
                             bib.name, 
                             l.loc[0].start_position().row    + 1,
                             l.loc[0].start_position().column + 1,
                             l.msg);
                    println!("{}",
                                l.loc.iter()
                                    .map(|n| {
                                        let s = bibtex.get_slice(*n);
                                        s.lines()
                                         .take(3)
                                         .zip(1..)
                                         .map(|(l, i)| format!("{:>4}| {}", i + n.start_position().row, l))
                                         .collect::<Vec<_>>()
                                         .join("\n")
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n...\n")
                                    .blue());
                    if let LintMessage::SyntaxError(_) = l.msg {
                        // print a bit before and a bit after 
                        // using colors to highlight the error
                        let start = l.loc[0].start_byte();
                        let end = l.loc[0].end_byte();
                        let before = &bib.content[(start - 10).max(0)..start];
                        let after = &bib.content[end..(end + 10).min(bib.content.len())];
                        print!("{}", before);
                        print!("{}", &bib.content[start..end].red());
                        print!("{}", after);
                    }
                    println!();
                }
            }
        }
        SubCommand::Format(cargs) => {
            let start_bib = std::fs::read_to_string("/Users/aliaume/Documents/transducer-bib/polyregular.bib").unwrap();
            let inputs = cargs.files.list_files();
            let mut db = LocalBibDb::new().import_bibtex(&start_bib);

            let mut format_options = FormatOptions::new(&mut db);
            format_options.sort_entries = true;

            let mut out = std::io::stdout();
            
            for bib in inputs {
                let bibtex = BibFile::new(&bib.content);
                let max_field_length = bibtex
                    .list_entries()
                    .map(|entry| {
                        entry
                            .fields
                            .iter()
                            .map(|field| bibtex.get_slice(field.name).len())
                            .max()
                            .unwrap_or(0)
                    }).max().unwrap_or(0);
                format_options.min_field_length = Some(max_field_length);
                write_bibfile(&bibtex, &format_options, &mut out);
            }
        }
        SubCommand::Setup(cargs) => {
            use bibadac::setup::{DownloadHandler, DownloadRequest, ArxivDownloader};
            let files = cargs.files.list_files();
            let downloader = ArxivDownloader {};
            for bib in files {
                let bibtex = BibFile::new(&bib.content);
                let eprints = bibtex.list_entries().filter_map(|entry| {
                    entry.fields.iter()
                         .find(|f| bibtex.get_slice(f.name) == "eprint")
                         .map(|f| bibtex.get_slice(f.value))
                         .map(|s| &s[1..s.len()-1])
                         .and_then(|f| {
                             Some(DownloadRequest::Arxiv(bibadac::arxiv_identifiers::ArxivId::try_from(f).ok()?))
                         })
                }).collect::<Vec<_>>();
                downloader.download(&eprints);
            }
        }
    }
}
