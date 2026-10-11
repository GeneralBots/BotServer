mod ooxml_extract;
mod rtf;
mod types;

pub use types::{ChunkMetadata, DocumentFormat, DocumentMetadata, TextChunk};

use anyhow::Result;
use log::{debug, info, warn};
use std::collections::HashMap;
use std::path::Path;
use tokio::io::AsyncReadExt;
use botlib::security::command_guard::SafeCommand;


#[derive(Debug)]
pub struct DocumentProcessor {
    chunk_size: usize,
    chunk_overlap: usize,
}


mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

