// SPDX-FileCopyrightText: Copyright © 2020-2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

//! The idea has to be that we have a top-level AbiReport struct which takes a collection of
//! AbiCapture and AbiHash structs.
//! These AbiCapture structs should ideally have a stable sort order (human numeric sorted?) after one of their
//! properties (filename probably?)
//! This sorting then yields an index per AbiCapture, which can then be used to create an AbiReport HashMap
//! that maps symbols to AbiCapture.filename indices?
//!
//! This would enable us to answer the questions:
//! 1. "Which file(s) has the symbol x?"
//! 2. "Which symbols does file x have?"
//! 3. "Which filename has soname x?"
//! 4. "Which soname has filename x?"
//!
//! For 1., this enables us to look at the *_deps vectors and use those as constraints when searching for
//! matching symbols.
//!

// use elf::ParseError;
// use elf::note::Note;
// use elf::note::NoteGnuBuildId;
// use elf::section::SectionHeader;
use abireport_rs::parse_elf;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    let files = &args[1..];

    if !files.is_empty() {
        for file in files {
            // Instantiating as symlink_metadata ensures that symlinks aren't followed
            let metadata = fs::symlink_metadata(file)
                .expect("{file} could not be parsed as symlink_metadata.");
            if !metadata.is_dir() && !metadata.is_symlink() {
                if let Some(abi_capture) =
                    Some(parse_elf(file).expect("{file} is not an ELF format file."))
                {
                    println!("{:#?}", abi_capture);
                }
            } else {
                println!("{file} is either a directory or a symlink. Skipping.")
            }
        }
    }
}

// let abi = AbiInfo {
//     filename: file_name.to_string(),
//     imports: abi_imports,
//     // imports_hash: ,
//     exports: abi_exports,
//     // exports_hash: ,
//     manual_deps: vec!["manual_deps not implemented".to_string()],
//     needed_deps: vec!["needed_deps not implemented".to_string()],
//     optional_deps: vec!["optional_deps not implemented".to_string()],
//     rpath: Some("rpath not implemented".to_string()),
//     runpath: Some("runpath not implemented".to_string()),
//     soname: Some("soname no implemented".to_string()),
// };
