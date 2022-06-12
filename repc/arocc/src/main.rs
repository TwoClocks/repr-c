// SPDX-License-Identifier: GPL-3.0-or-later
use anyhow::{anyhow, bail, Context, Result};
use cly_impl::ast::Declaration;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use repc_impl::target::{system_compiler, Compiler, Target, TARGETS};
use repc_tests::{read_input_config, GlobalConfig, InputConfig};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{ErrorKind, Write};
use std::path::Path;
use std::process;
use std::process::Command;

#[path="../../test-generator/src/c.rs"]
mod c;
// mod dwarf;
// mod pdb;

fn main() {
    if let Err(e) = main_() {
        eprintln!("{:?}", e);
        process::exit(1);
    }
}

/// this is ugly ugly ugly code.
/// it's only intended to be run once, to generate the test files
/// for arocc. Don't judge me.
fn main_() -> Result<()> {
    // let userconfig: GlobalConfig = toml::from_str(&std::fs::read_to_string("config.toml")?)?;

    let mut dirs = vec![];
    for dir in std::fs::read_dir("testfiles").expect("Can't open testfiles dir") {
        let dir = dir?;
        if dir.file_type()?.is_dir() { //&& userconfig.test_test(dir.file_name().to_str().unwrap()) {
            dirs.push(dir.path());
        }
    }
    dirs.sort();
    dirs.iter().for_each(|dir| {
        let config = read_input_config(dir)
            .with_context(|| anyhow!("cannot read config in {}", dir.display())).expect("aksljdh");
        let input_path = dir.join("input.txt");
        let input = std::fs::read_to_string(&input_path).expect("akjhsd");
        // let hash = {
        //     let mut hash = DefaultHasher::new();
        //     input.hash(&mut hash);
        //     config.0.hash(&mut hash);
        //     hash.finish()
        // };
        let declarations = cly_impl::parse(&input)
            .with_context(|| anyhow!("Parsing of {} failed", input_path.display())).expect("failed to parse input file");

        let targets:Vec<_> = TARGETS.iter().filter(|t| { config.1.test_target(**t) }).collect();
        let mut comps:HashSet<_> = targets.iter().map(|t|{system_compiler(**t)}).collect();

        if comps.contains( &Compiler::Clang ) && comps.contains( &Compiler::Gcc ) {
            comps.remove(&Compiler::Gcc);
        }

        let dir_name = dir.file_name().unwrap().to_str().unwrap();
        let f_name = format!("{}_test.c",dir_name);
        println!("{}",f_name);

        let c_code = if comps.contains( &Compiler::Clang ) {
            let (code, ids) = c::generate(&declarations,Compiler::Clang).unwrap();
            Some(code)
        } else {
            None
        };

        let m_code = if comps.contains( &Compiler::Msvc ) {
            let (code, ids) = c::generate(&declarations,Compiler::Msvc).unwrap();
            Some(code)
        } else {
            None
        };

        match (c_code, m_code) {
            (Some(c),None) => std::fs::write(f_name, c).expect("die die die"),
            (None,Some(m)) => std::fs::write(f_name, m).expect("die die die"),
            (Some(c), Some(m)) => merge_two(c,m,f_name),
            (None, None) => unreachable!()
        };





    });
    Ok(())
}


fn merge_two(c:String, m:String, f_name:String) {
    let mut f_out = File::create(f_name).expect("die die die");
    let mut c_idx = 0;
    let mut m_idx = 0;
    let c_v:Vec<_> = c.lines().map(|s|s.to_string()).collect();
    let m_v:Vec<_> = m.lines().map(|s|s.to_string()).collect();
    let mut c_a = 0;
    let mut is_open = false;
    while c_idx < c_v.len() && m_idx < m_v.len() {
        if c_v[c_idx + c_a] == m_v[m_idx] {
            if is_open {
                writeln!(f_out, "#else" );
                let cur = c_idx+c_a;
                while c_idx < cur {
                    writeln!(f_out, "{}", c_v[c_idx] );
                    c_idx += 1;
                    // println!("{} {}", c_idx, c_idx+c_a );
                }
                // println!("le");
                writeln!(f_out, "#endif" );
                is_open = false;
                c_a = 0;
            }
            writeln!(f_out, "{}", c_v[c_idx] );
            c_idx += 1;
            m_idx += 1;
        } else {
            if !is_open {
                is_open = true;
                writeln!(f_out, "#ifdef MSVC" );
            }
            writeln!(f_out, "{}", m_v[m_idx] );
            m_idx += 1;
            c_a += 1;
        }
    }
}

fn up_to_date(hash: u64, expected: &Path) -> Result<bool> {
    let input = match std::fs::read_to_string(expected) {
        Ok(i) => i,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.into()),
    };
    let last = match input.lines().last() {
        Some(l) => l,
        None => return Ok(false),
    };
    let suffix = match last.strip_prefix("// hash: ") {
        Some(s) => s,
        None => return Ok(false),
    };
    match u64::from_str_radix(suffix, 16) {
        Ok(n) if n == hash => Ok(true),
        _ => Ok(false),
    }
}

