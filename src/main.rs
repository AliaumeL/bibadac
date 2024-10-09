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

use bibadac::bibdb::{BibDb, LocalBibDb};
use bibadac::bibtex::BibFile;
use bibadac::format::{write_bibfile, FormatOptions};
use bibadac::linter::{LintMessage, LinterState};

#[derive(Debug, Clone, Parser)]
#[command(name = "bibadac")]
#[command(about = "A tool to handle bibliographic data")]
struct Cli {
    #[command(subcommand)]
    command: SubCommand,
}

#[derive(Debug, Clone, Subcommand)]
enum SubCommand {
    #[command(about = "Check the validity of a BibTeX/BibLaTeX file", arg_required_else_help = true)]
    Check(CheckArgs),
    #[command(about = "Format a BibTeX/BibLaTeX file", arg_required_else_help = true)]
    Format(FormatArgs),
    #[command(about = "Download pdfs that are mentionned in the file")]
    Setup,
}

#[derive(Debug, Clone, Args)]
struct CheckArgs {
    #[arg(short, long)]
    bib: Vec<String>,
    #[arg(short, long)]
    stdin: bool,
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Clone, Args)]
struct FormatArgs {
    #[arg(short, long)]
    bib: Vec<String>,
    #[arg(short, long)]
    stdin: bool,
}

fn main() {
    let args = Cli::parse();

    match args.command {
        SubCommand::Check(cargs) => {
            for bib in cargs.bib {
                let path = std::path::Path::new(&bib);
                if !path.exists() {
                    eprintln!("File {} does not exist", bib);
                    continue;
                }
                let content = std::fs::read_to_string(path).unwrap();
                let bibtex = BibFile::new(&content);
                let linter = LinterState {
                    revoked_dois: Default::default(),
                    arxiv_latest: Default::default(),
                    doi_arxiv: Default::default(),
                    arxiv_doi: Default::default(),
                };
                let lints = linter.lint_file(&bibtex, bibtex.list_entries().collect());
                if lints.len() == 0 {
                    println!("{} {}", "[OK]".green(), path.file_name().unwrap().to_str().unwrap());
                } else {
                    let err = if lints.len() > 1 { "errors" } else { "error" };
                    println!("{} {} \t {} {}", 
                             "[KO]".red(),
                             path.file_name().unwrap().to_str().unwrap(),
                             lints.len(),
                             err);
                }
                if cargs.verbose {
                    for l in lints {
                        println!("<{:?}:L{}:C{}> {:?}", 
                                 path.file_name(), 
                                 l.loc[0].start_position().row + 1,
                                 l.loc[0].start_position().column + 1,
                                 l.msg);
                        println!("{}", bibtex.get_slice(l.loc[0]));
                        if let LintMessage::SyntaxError(_) = l.msg {
                            // print a bit before and a bit after 
                            // using colors to highlight the error
                            let start = l.loc[0].start_byte();
                            let end = l.loc[0].end_byte();
                            let before = &content[(start - 10).max(0)..start];
                            let after = &content[end..(end + 10).min(content.len())];
                            print!("{}", before);
                            print!("{}", &content[start..end].red());
                            print!("{}", after);
                        }
                        println!();
                    }
                }
            }

            if cargs.stdin {
                let mut content = String::new();
                std::io::stdin().read_to_string(&mut content).unwrap();
                let bibtex = BibFile::new(&content);
                let linter = LinterState {
                    revoked_dois: Default::default(),
                    arxiv_latest: Default::default(),
                    doi_arxiv: Default::default(),
                    arxiv_doi: Default::default(),
                };
                let lints = linter.lint_file(&bibtex, bibtex.list_entries().collect());
                if lints.len() == 0 {
                    println!("{} {}", "[OK]".green(), "stdin");
                } else {
                    let err = if lints.len() > 1 { "errors" } else { "error" };
                    println!("{} {} \t {} {}", 
                             "[KO]".red(),
                             "stdin",
                             lints.len(),
                             err);
                }
                if cargs.verbose {
                    for l in linter.lint_file(&bibtex, bibtex.list_entries().collect()) {
                        println!("<stdin:L{}:C{}> {:?}", 
                                 l.loc[0].start_position().row + 1,
                                 l.loc[0].start_position().column + 1,
                                 l.msg);
                        println!("{}", bibtex.get_slice(l.loc[0]));
                        println!();
                    }
                }
            }
        }
        SubCommand::Format(cargs) => {

            let start_bib = std::fs::read_to_string("/Users/aliaume/Documents/transducer-bib/polyregular.bib").unwrap();

            let mut db = LocalBibDb::new().import_bibtex(&start_bib);

            for bib in cargs.bib {
                let path = std::path::Path::new(&bib);
                if !path.exists() {
                    eprintln!("File {} does not exist", bib);
                    continue;
                }
                let content = std::fs::read_to_string(path).unwrap();
                let bibtex = BibFile::new(&content);
                let mut format_options : FormatOptions<&mut LocalBibDb> = FormatOptions::new(&mut db);

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
                format_options.sort_entries = true;

                let mut out = std::io::stdout();
                write_bibfile(&bibtex, &format_options, &mut out);
            }
        }
        SubCommand::Setup => {
            println!("Setup");
        }
    }
}
