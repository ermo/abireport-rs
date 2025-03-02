// SPDX-FileCopyrightText: Copyright © 2020-2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

#![allow(dead_code)] // TODO

use elf::abi;
use elf::endian::AnyEndian;
use elf::{CommonElfData, ElfBytes};
// use elf::ParseError;
// use elf::note::Note;
// use elf::note::NoteGnuBuildId;
// use elf::section::SectionHeader;
use natural_sort_rs::NaturalSortable;
use std::env;
use std::io::Result;

fn main() {
    let args: Vec<String> = env::args().collect();

    let files = &args[1..];

    if files.len() > 0 {
        for file in files {
            let abi_info = parse_elf(file).expect("{file} is not an ELF format file.");
            println!("\nABI-imports:");
            println!("{file}:");
            for dynsym in abi_info.dynsym_imports {
                println!("\t{:?}", dynsym);
            }
            println!("\nABI-exports:");
            println!("{file}:");
            for dynsym in abi_info.dynsym_exports {
                println!("\t{:?}", dynsym);
            }
        }
    }
}

#[derive(Debug)]
struct AbiInfo {
    filename: String,            // Stuff that needs to can instantiate this as a Pathbuf
    dynsym_imports: Vec<String>, // the string version of symbols (deliberately unversioned for now)
    //    dynsym_imports_hash: ,
    dynsym_exports: Vec<String>, // the string version of symbols (deliberately unversioned for now)
    //    dynsym_exports_hash: ,
    manual_deps: Vec<String>, // deps added manually by a packager (could be useful?)
    needed_deps: Vec<String>, // dynamically linked at build time (via DT_NEEDED)
    optional_deps: Vec<String>, // dynamically linked and opened at runtime (via dlopen() calls)
    rpath: Option<String>, // DT_RPATH if available (needs to be analysed _after_ any patchelf manipulation)
    runpath: Option<String>, // DT_RUNPATH if available (needs to be analysed _after_ any patchelf manipulation)
    soname: Option<String>,  // DT_SONAME if available (this will be empty for executables)
}

/// All the info we need for ABI parsing purposes.
fn parse_elf(file_name: &str) -> Result<AbiInfo> {
    // TODO: which error type might be useful here...?

    let path = std::path::PathBuf::from(file_name);
    let file_data = std::fs::read(path).expect("Could not read file {file_name:?}.");

    // We want to be able to skip around in the file
    let file_slice = file_data.as_slice();
    let elf_file = ElfBytes::<AnyEndian>::minimal_parse(file_slice)
        .expect("Could not parse {file_name:?} as ELF data.");

    // Find lazy-parsing types for the common ELF sections (we want .dynsym and .dynstr)
    let common_elf_data = elf_file
        .find_common_data()
        .expect("Section headers (shdrs) of {file_name:?} should parse.");

    let (ds_imports, ds_exports) = parse_dynsyms(&common_elf_data);

    Ok(AbiInfo {
        filename: file_name.to_string(),
        dynsym_imports: ds_imports,
        dynsym_exports: ds_exports,
        manual_deps: vec!["Not implemented".to_string()],
        needed_deps: vec!["Not implemented".to_string()],
        optional_deps: vec!["Not implemented".to_string()],
        rpath: parse_rpath(&common_elf_data),
        runpath: parse_runpath(&common_elf_data),
        soname: parse_soname(&common_elf_data),
    })
}

fn parse_dynsyms<'data>(
    common_elf_data: &CommonElfData<'data, AnyEndian>,
) -> (Vec<String>, Vec<String>) {
    let (dynsyms, strtab) = (
        common_elf_data.dynsyms.as_ref().unwrap(),
        common_elf_data.dynsyms_strs.as_ref().unwrap(),
    );

    // The fields that will eventually be moved into an ABI struct as the return value
    let mut abi_imports: Vec<String> = Vec::new();
    let mut abi_exports: Vec<String> = Vec::new();

    for dynsym in dynsyms.iter() {
        // find the type of each symbol (imported or exported)
        // each dynsym entry has a string table entry associated with it
        let ds = strtab
            .get(dynsym.st_name.try_into().unwrap())
            .unwrap()
            .to_string();

        let imported = dynsym.is_undefined();
        // st_vis() returns > 0 if flags other than STB_GLOBAL or STB_WEAK are set
        let exported = !dynsym.is_undefined() && dynsym.st_vis() == 0;

        // Not sure this is the most elegant way, but...
        if imported {
            // we import (= rely on) undefined symbols (currenly the only constraint)
            // println!("\t\tImporting {:?}: (st_symtype(): {:?}, st_bind(): {:?}, st_vis(): {:?})",
            //     ds, dynsym.st_symtype(), dynsym.st_bind(), dynsym.st_vis());
            abi_imports.push(ds);

        // this implicitly matches !is_undefined()
        } else if exported {
            // only export defined and visible symbols for now (= global or weak/overridable)
            // println!("\t\tExporting {:?}: (st_symtype(): {:?}, st_bind(): {:?}, st_vis(): {:?})",
            //     ds, dynsym.st_symtype(), dynsym.st_bind(), dynsym.st_vis());
            abi_exports.push(ds);
        } else {
            // defined but not visible, only printed for completeness sake for now
            println!(
                "\t\tIgnoring {:?}: (st_symtype(): {:?}, st_bind(): {:?}, st_vis(): {:?})",
                ds,
                dynsym.st_symtype(),
                dynsym.st_bind(),
                dynsym.st_vis()
            );
        }
    }

    abi_imports.sort_by(|a, b| a.natural_cmp(b));
    abi_exports.sort_by(|a, b| a.natural_cmp(b));
    (abi_imports, abi_exports)
}

fn parse_rpath<'data>(common_elf_data: &CommonElfData<'data, AnyEndian>) -> Option<String> {
    // parse DT_RPATH
    let (dynamic, strtab) = (
        common_elf_data.dynamic.as_ref().unwrap(),
        common_elf_data.symtab_strs.as_ref().unwrap(),
    );
    let rpath_strtab_index: usize = dynamic
        .get(abi::DT_RPATH.try_into().unwrap()) // because we don't know the usize in advance
        .unwrap()
        .d_tag
        .try_into()
        .unwrap();
    Some(strtab.get(rpath_strtab_index).unwrap().to_owned())
}

fn parse_runpath<'data>(common_elf_data: &CommonElfData<'data, AnyEndian>) -> Option<String> {
    // parse DT_RPATH
    let (dynamic, strtab) = (
        common_elf_data.dynamic.as_ref().unwrap(),
        common_elf_data.symtab_strs.as_ref().unwrap(),
    );
    let runpath_strtab_index: usize = dynamic
        .get(abi::DT_RPATH.try_into().unwrap()) // because we don't know the usize in advance
        .unwrap()
        .d_tag
        .try_into()
        .unwrap();
    Some(strtab.get(runpath_strtab_index).unwrap().to_owned())
}

fn parse_soname<'data>(common_elf_data: &CommonElfData<'data, AnyEndian>) -> Option<String> {
    // parse DT_RPATH
    let (dynamic, strtab) = (
        common_elf_data.dynamic.as_ref().unwrap(),
        common_elf_data.symtab_strs.as_ref().unwrap(),
    );
    let soname_strtab_index: usize = dynamic
        .get(abi::DT_SONAME.try_into().unwrap()) // because we don't know the usize in advance
        .unwrap()
        .d_tag
        .try_into()
        .unwrap();
    Some(strtab.get(soname_strtab_index).unwrap().to_owned())
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
