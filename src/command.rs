use std::{
    fs::File,
    io,
    net::{Ipv4Addr, SocketAddrV4},
    path::Path,
};

use clap::{Parser, Subcommand};
use thiserror::Error;
use tiny_http::{Method, Request, Response, Server, StatusCode};

use crate::{
    index::{TermFreqIndex, index_directory, read::read_index, write::write_index},
    search::{Searcher, TfIdfSearcher},
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
    },
}

pub fn handle(args: Args) -> Result<(), CommandError> {
    match args.commands {
        Commands::Index {
            dir,
            recursive,
            output,
        } => handle_index(&dir, recursive, &output)?,

        Commands::Read { path } => handle_read(&path)?,
        Commands::Serve { index_path, port } => {
            handle_serve(&index_path, Ipv4Addr::new(127, 0, 0, 1), port)?
        }
    }
    Ok(())
}

fn handle_index(dir: &str, recursive: bool, output: &str) -> Result<(), CommandError> {
    let mut term_freq_indexes = TermFreqIndex::new();
    let dir_path = Path::new(&dir);
    let output = Path::new(output);
    term_freq_indexes.extend(index_directory(&dir_path, recursive));
    write_index(&term_freq_indexes, &output)?;
    println!("Saved {:?}", output);

    Ok(())
}

fn handle_read(path: &str) -> Result<(), CommandError> {
    let path = Path::new(path);
    let term_freq_indexes = read_index(&path)?;

    println!(
        "{path:?} contains {count} files",
        count = term_freq_indexes.len()
    );

    Ok(())
}

fn handle_serve(index_path: &str, addr: Ipv4Addr, port: u16) -> Result<(), CommandError> {
    let path = Path::new(index_path);
    let tf_idf_searcher = TfIdfSearcher::new(read_index(&path)?);

    let server =
        Server::http(SocketAddrV4::new(addr, port)).expect("ERROR: cound not start HTTP server");
    println!("INFO: listening at http://127.0.0.1:{port}");
    loop {
        let request = server.recv().expect("ERROR: receive request failure");
        serve_request(&tf_idf_searcher, request);
    }
}

fn serve_request(searcher: &impl Searcher, mut request: Request) {
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

            let result = searcher.search(&body);

            for (path, rank) in result.iter().take(30) {
                println!("{path} => {rank}", path = path.display());
            }

            request
                .respond(Response::from_string("ok"))
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
