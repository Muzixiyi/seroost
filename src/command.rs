use std::{
    fs::File,
    io,
    net::{Ipv4Addr, SocketAddrV4},
    path::Path,
};

use clap::{Parser, Subcommand, ValueEnum};
use thiserror::Error;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use waken_snowball::Algorithm;

use crate::{
    index::{
        index_directory,
        read::read_index,
        term_processor::{Lowercase, Processor, Raw, Stemming, TermProcessor, Uppercase},
        write::write_index,
    },
    search::{BM25Searcher, Searcher},
};

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Parser)]
pub struct Args {
    #[command(subcommand)]
    pub commands: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Index a directory of XML files
    Index {
        /// The directory to index
        #[arg(short, long)]
        dir: String,
        /// Recursively index subdirectories
        #[arg(short, long)]
        recursive: bool,
        /// The output file path
        #[arg(short, long, default_value = "indexes/index.json")]
        output: String,
        /// The term strategy to use
        #[arg(short, long, default_value = "raw")]
        strategy: TermStrategy,
    },
    /// Read an index from a JSON file
    Read {
        #[arg(short, long)]
        path: String,
    },
    Serve {
        /// Search from a index JSON file
        #[arg(short, long, default_value = "indexes/index.json")]
        index_path: String,
        /// The server port
        #[arg(short, long, default_value = "6999")]
        port: u16,
        /// The term strategy to use
        #[arg(short, long, default_value = "raw")]
        strategy: TermStrategy,
    },
}

/// The term strategy to use for indexing
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TermStrategy {
    Raw,
    Lowercase,
    Uppercase,
    /// Convert term to lowercase for better matching, as the stemmer cannot handle uppercase words
    Stemming,
}

impl From<&TermStrategy> for Processor {
    fn from(value: &TermStrategy) -> Self {
        match value {
            TermStrategy::Raw => Processor::Raw(Raw::default()),
            TermStrategy::Lowercase => Processor::Lowercase(Lowercase::default()),
            TermStrategy::Uppercase => Processor::Uppercase(Uppercase::default()),
            TermStrategy::Stemming => {
                Processor::Stemming(Stemming::new(Algorithm::English.stemmer()))
            }
        }
    }
}

impl TermStrategy {
    pub fn processor(&self) -> Processor {
        self.into()
    }
}

pub fn handle(args: Args) -> Result<(), CommandError> {
    match args.commands {
        Commands::Index {
            dir,
            recursive,
            output,
            strategy,
        } => handle_index(&dir, recursive, &output, strategy.processor())?,

        Commands::Read { path } => handle_read(&path)?,
        Commands::Serve {
            index_path,
            port,
            strategy,
        } => handle_serve(
            &index_path,
            Ipv4Addr::new(127, 0, 0, 1),
            port,
            strategy.processor(),
        )?,
    }
    Ok(())
}

fn handle_index(
    dir: &str,
    recursive: bool,
    output: &str,
    term_processor: impl TermProcessor,
) -> Result<(), CommandError> {
    let dir_path = Path::new(&dir);
    let output = Path::new(output);

    let model = index_directory(&dir_path, recursive, &term_processor);

    write_index(&model, &output)?;
    println!("Saved {:?}", output);

    Ok(())
}

fn handle_read(path: &str) -> Result<(), CommandError> {
    let path = Path::new(path);
    let model = read_index(&path)?;

    println!("{path:?} contains {count} files", count = model.doc_count());

    Ok(())
}

fn handle_serve(
    index_path: &str,
    addr: Ipv4Addr,
    port: u16,
    term_processor: impl TermProcessor,
) -> Result<(), CommandError> {
    let path = Path::new(index_path);
    let searcher = BM25Searcher::new(read_index(&path)?);

    let server =
        Server::http(SocketAddrV4::new(addr, port)).expect("ERROR: cound not start HTTP server");
    println!("INFO: listening at http://127.0.0.1:{port}");
    loop {
        let request = server.recv().expect("ERROR: receive request failure");
        serve_request(&searcher, request, &term_processor);
    }
}

fn serve_request(
    searcher: &impl Searcher,
    mut request: Request,
    term_processor: &impl TermProcessor,
) {
    println!(
        "INFO: received request! method: {:?}, url: {:?}",
        request.method(),
        request.url()
    );
    match (request.method(), request.url()) {
        (Method::Post, "/api/search") => {
            let mut body = String::new();
            request
                .as_reader()
                .read_to_string(&mut body)
                .expect("ERROR: cound not read body");

            let result = searcher.search(&body, term_processor);

            let result_json =
                match serde_json::to_string(&result.iter().take(30).collect::<Vec<_>>()) {
                    Ok(json) => json,
                    Err(e) => {
                        eprintln!("ERROR: could not serialize result: {e}");
                        return;
                    }
                };

            request
                .respond(
                    Response::from_string(result_json).with_header(
                        Header::from_bytes(b"Content-Type", b"application/json;charset=utf-8")
                            .expect(
                                "ERROR: failed to create Content-Type header from static bytes",
                            ),
                    ),
                )
                .expect("ERROR: respond failure");
        }
        (Method::Get, "/" | "/index.html") => serve_static_file(request, "static/index.html"),
        (Method::Get, "/index.js") => serve_static_file(request, "static/index.js"),
        _ => serve_404(request),
    }
}

fn serve_static_file(request: Request, file_path: &str) {
    let error_message = format!("ERROR: can't open response file {file_path}");
    let response = Response::from_file(File::open(file_path).expect(&error_message));
    request.respond(response).expect("ERROR: respond failure");
}

fn serve_404(request: Request) {
    request
        .respond(Response::from_string("404").with_status_code(StatusCode(404)))
        .expect("ERROR: respond failure");
}
