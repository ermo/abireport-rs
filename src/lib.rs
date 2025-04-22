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

#![allow(dead_code)] // TODO

use elf::abi::{DT_NEEDED, DT_RPATH, DT_RUNPATH, DT_SONAME};
use elf::endian::AnyEndian;
use elf::{CommonElfData, ElfBytes};
use natural_sort_rs::NaturalSortable;
use std::fmt::Debug;
use std::io::Result;

#[derive(Debug)]
enum ElfKind {
    Executable,
    SharedObject,
    Unknown,
}

#[derive(Debug)]
pub struct AbiCapture {
    elf_kind: ElfKind,           // This seems useful to know
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
pub fn parse_elf(file_name: &str) -> Result<AbiCapture> {
    // TODO: which error type might be useful here...?

    let path = std::path::PathBuf::from(file_name);
    let file_data = std::fs::read(path).expect("Could not read file {file_name:?}.");

    // We want to be able to skip around in the file
    let file_slice = file_data.as_slice();
    let elf_file = ElfBytes::<AnyEndian>::minimal_parse(file_slice)
        .expect("Could not parse {file_name:?} as ELF data.");

    // Find the common ELF sections (we want .dynsym and .dynstr)
    let common_elf_data = elf_file
        .find_common_data()
        .expect("ELF section headers (shdrs) of {file_name:?} should parse.");

    let (ds_imports, ds_exports) = parse_dynsyms_section(&common_elf_data);
    let (dt_needed, dt_rpath, dt_runpath, dt_soname) = parse_dynamic_section(&common_elf_data);

    Ok(AbiCapture {
        elf_kind: ElfKind::Unknown,
        filename: file_name.to_string(),
        dynsym_imports: ds_imports,
        dynsym_exports: ds_exports,
        manual_deps: vec!["Not implemented".to_string()],
        needed_deps: dt_needed,
        optional_deps: vec!["Not implemented".to_string()],
        rpath: dt_rpath,
        runpath: dt_runpath,
        soname: dt_soname,
    })
}

fn parse_dynsyms_section(common_elf_data: &CommonElfData<AnyEndian>) -> (Vec<String>, Vec<String>) {
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
        // TODO: build our own, more discerning visibility function here (cf. clearlinux's abireport tool)
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

fn parse_dynamic_section(
    common_elf_data: &CommonElfData<AnyEndian>,
) -> (
    Vec<String>,    // dt_needed
    Option<String>, // dt_rpath
    Option<String>, // dt_runpath
    Option<String>, // dt_soname
) {
    // default values if everything goes to shit
    let mut dt_needed = vec![];
    let mut dt_rpath = None;
    let mut dt_runpath = None;
    let mut dt_soname = None;

    if let Some(dynamic) = &common_elf_data.dynamic {
        if let Some(dynsyms_strs) = &common_elf_data.dynsyms_strs {
            for entry in dynamic.iter() {
                match entry.d_tag {
                    DT_NEEDED => dt_needed.push(
                        dynsyms_strs
                            .get(entry.d_val().try_into().unwrap())
                            .unwrap()
                            .to_string(),
                    ),
                    DT_RPATH => {
                        dt_rpath = Some(
                            dynsyms_strs
                                .get(entry.d_val().try_into().unwrap())
                                .unwrap()
                                .to_string(),
                        );
                    }
                    DT_RUNPATH => {
                        dt_runpath = Some(
                            dynsyms_strs
                                .get(entry.d_val().try_into().unwrap())
                                .unwrap()
                                .to_string(),
                        );
                    }
                    DT_SONAME => {
                        dt_soname = Some(
                            dynsyms_strs
                                .get(entry.d_val().try_into().unwrap())
                                .unwrap()
                                .to_string(),
                        );
                    }
                    _ => {}
                }
            }
            // we want this in natural sort order
            dt_needed.sort_by(|a, b| a.natural_cmp(b));
        }
    }
    (dt_needed, dt_rpath, dt_runpath, dt_soname)
}
