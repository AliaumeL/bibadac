/// This is the `bibadac` program to handle bibliographic data
/// written using the BibTeX/BibLaTeX formats.
///
/// The program contains 3 subcommands:
/// - `check`: check the validity of a BibTeX/BibLaTeX file
/// - `format`: format a BibTeX/BibLaTeX file
/// - `setup`: download pdfs that are mentionned in the file
///
use clap::{Args, Parser, Subcommand, ValueEnum};

use bibadac::bibtex::BibFile;
use bibadac::format::{write_bibfile, FormatOptions};
use bibadac::linter::LinterState;

#[derive(Debug, Clone, Parser)]
#[command(name = "bibadac")]
#[command(about = "A tool to handle bibliographic data")]
struct Cli {
    #[command(subcommand)]
    command: SubCommand,
}

#[derive(Debug, Clone, Subcommand)]
enum SubCommand {
    #[command(about = "Check the validity of a BibTeX/BibLaTeX file")]
    Check,
    #[command(about = "Format a BibTeX/BibLaTeX file")]
    Format,
    #[command(about = "Download pdfs that are mentionned in the file")]
    Setup,
}

fn main() {
    let args = Cli::parse();

    match args.command {
        SubCommand::Check => {
            println!("Check");
        }
        SubCommand::Format => {
            println!("Format");
        }
        SubCommand::Setup => {
            println!("Setup");
        }
    }

    let bibtex = BibFile::new("@article{key, author = {Author1 \n and Author3}, year = {2025}, title = {Title}} test @coucou{key2, author = {Author2}, title = {Title2}}");

    for entry in bibtex.list_entries() {
        println!("{:?}", entry);
    }

    let linter = LinterState {
        revoked_dois: Default::default(),
        arxiv_latest: Default::default(),
        doi_arxiv: Default::default(),
        arxiv_doi: Default::default(),
    };

    println!("{:?}", bibtex);

    for msg in linter.lint_file(&bibtex, bibtex.list_entries().collect()) {
        println!("{:?}", msg);
    }
    // write to stdout
    let mut out = std::io::stdout();
    write_bibfile(&bibtex, &FormatOptions::default(), &mut out);
}
