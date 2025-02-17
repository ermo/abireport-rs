// SPDX-FileCopyrightText: Copyright © 2020-2025 Serpent OS Developers
//
// SPDX-License-Identifier: MPL-2.0

use elf::ElfBytes;
// use elf::ParseError;
use elf::endian::AnyEndian;
// use elf::note::Note;
// use elf::note::NoteGnuBuildId;
// use elf::section::SectionHeader;
use natural_sort_rs::NaturalSortable;

fn main() {

    let file = "/usr/lib/libc.so.6";
    let abi_imports : Vec<String>;
    let abi_exports : Vec<String>;
    (abi_imports, abi_exports) = parse_elf(file);
    println!("\nABI-imports:");
    println!("{file}:");
    for dynsym in abi_imports {
        println!("\t{:?}", dynsym);
    }
    println!("\nABI-exports:");
    println!("{file}:");
    for dynsym in abi_exports {
        println!("\t{:?}", dynsym);
    }
}


fn parse_elf(file_name : &str) -> (Vec<String>, Vec<String>) {
    
    let path = std::path::PathBuf::from(file_name);
    let file_data = std::fs::read(path).expect("Could not read file {file_name:?}.");

    // We want to be able to skip around in the file
    let file_slice = file_data.as_slice();
    let elf_file = ElfBytes::<AnyEndian>::minimal_parse(file_slice).expect("Could not parse {file_name:?} as ELF data.");

    let mut abi_imports = Vec::new();
    let mut abi_exports = Vec::new();

    // Find lazy-parsing types for the common ELF sections (we want .dynsym and .dynstr)
    let common = elf_file.find_common_data().expect("Section headers (shdrs) should parse.");
    let (dynsyms, strtab) = (common.dynsyms.unwrap(), common.dynsyms_strs.unwrap());

    for dynsym in dynsyms {
        // find the type of each symbol (imported or exported)
        // each dynsym entry has a string table entry associated with it
        let ds = strtab.get(dynsym.st_name.try_into().unwrap()).unwrap().to_string();

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
            println!("\t\tIgnoring {:?}: (st_symtype(): {:?}, st_bind(): {:?}, st_vis(): {:?})",
                ds, dynsym.st_symtype(), dynsym.st_bind(), dynsym.st_vis());
        }
    }

    abi_imports.sort_by(|a, b| a.natural_cmp(b));
    abi_exports.sort_by(|a, b| a.natural_cmp(b));
    (abi_imports, abi_exports)
}
