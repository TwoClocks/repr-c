// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::iter::FromIterator;
// use std::fs::File;
// use std::mem;
// use std::path::PathBuf;
use std::process;

use cly_impl::ast::{Declaration, DeclarationType, TypeVariant};
// use cly_impl::converter::ConversionResult;
mod c;
// use itertools::Itertools;
use repc_impl::target::{system_compiler, Compiler, Target, TARGETS, TARGET_MAP};
use repc_impl::util::align_to;
use repc_tests::read_input_config;
// mod dwarf;
// mod pdb;

// numer to english word
const ENGLISH: [&str; 20] = [
    "ZERO",
    "ONE",
    "TWO",
    "THREE",
    "FOUR",
    "FIVE",
    "SIX",
    "SEVEN",
    "EIGHT",
    "NINE",
    "TEN",
    "ELEVEN",
    "TWELVE",
    "THIRTEEN",
    "FOURTEEN",
    "FIFTEEN",
    "SIXTEEN",
    "SEVENTEEN",
    "EIGHTEEN",
    "NINETEEN",
];

fn main() {
    if let Err(e) = main_() {
        eprintln!("{:?}", e);
        process::exit(1);
    }
}

/// this is ugly ugly ugly code.
/// it's only intended to be run once, to generate the test files
/// for arocc. Don't judge me.
fn main_() -> std::io::Result<()> {
    let mut reprc_zig_map: HashMap<Target, String> = HashMap::new();
    {
        let mut rust_zig_map: HashMap<String, String> = HashMap::new();

        let m_file = std::fs::read_to_string("mapping.text")?;
        m_file.lines().for_each(|l| {
            let x = l.split_once(':').unwrap();
            rust_zig_map.insert(x.0.to_owned(), x.1.to_owned());
        });

        // reprc targets map to mutiple rust targets. So insert by hand.
        for (rust, reprc) in TARGET_MAP {
            //.iter().map(|t| (t.1, t.0)).collect();
            match rust_zig_map.get(rust.to_owned()) {
                Some(z) => {
                    // only insert the first one
                    if !reprc_zig_map.contains_key(reprc) {
                        reprc_zig_map.insert(*reprc, z.clone());
                    }
                }
                None => println!("no mapping for {:?} | {}", reprc, rust),
            }
        }
    }

    let mut dirs = vec![];
    for dir in std::fs::read_dir("testfiles").expect("Can't open testfiles dir") {
        let dir = dir?;
        if dir.file_type()?.is_dir() {
            dirs.push(dir.path());
        }
    }
    dirs.sort();
    for dir in dirs.iter() {
        let config = read_input_config(dir).unwrap();
        let input_path = dir.join("input.txt");
        let input = std::fs::read_to_string(&input_path).expect("");
        let declarations = cly_impl::parse(&input).unwrap();
        // let zig_targets:HashSet<&str> = HashSet::new();

        let targets: Vec<_> = TARGETS
            .iter()
            .filter(|t| config.1.test_target(**t))
            .collect();
        // if targets.contains(&&Target::X86_64UnknownLinuxGnu) {
        let mut comps: HashSet<_> = targets.iter().map(|t| system_compiler(**t)).collect();

        if comps.contains(&Compiler::Clang) && comps.contains(&Compiler::Gcc) {
            comps.remove(&Compiler::Gcc);
        }

        let dir_name = dir.file_name().unwrap().to_str().unwrap();
        let f_name = format!("{}_test.c", dir_name);
        let mut out_str = "// SPDX-License-Identifier: GPL-3.0-or-later\n\n\
        // This test file is auto-generated. do not edit.\n\
        // This file is a derivative work from the test files found\
        // in this repo : https://github.com/mahkoh/repr-c\n\
        // and is under the same licence as the original work.\n\n"
            .to_string();

        let c_code = if comps.contains(&Compiler::Clang) {
            let (code, _ids) = c::generate(&declarations, Compiler::Clang).unwrap();
            Some(code)
        } else {
            None
        };

        let m_code = if comps.contains(&Compiler::Msvc) {
            let (code, _ids) = c::generate(&declarations, Compiler::Msvc).unwrap();
            Some(code)
        } else {
            None
        };

        match (c_code, m_code) {
            (Some(c), None) => writeln!(out_str, "{}", c).unwrap(),
            (None, Some(m)) => writeln!(out_str, "#ifdef MSVC\n{}\n#endif\n", m).unwrap(),
            (Some(c), Some(m)) => merge_two(c, m, &mut out_str),
            (None, None) => unreachable!(),
        };

        #[derive(Debug)]
        struct TargetAst {
            targets: Vec<Target>,
            decls: Vec<Declaration>,
        }

        let mut uniq: Vec<TargetAst> = Vec::new();

        targets.iter().for_each(|target| {
            // let target = &&Target::X86_64UnknownLinuxGnu;
            let tname = target.name();
            let exp_file = dir.join("output").join(format!("{}.expected.txt", tname));
            let exp_str = std::fs::read_to_string(&exp_file).expect("can't open output file");
            let decls = cly_impl::parse(&exp_str).expect("can't open output file");
            // let ast = cly_impl::extract_layouts( &exp_str, &decls).expect("can't parse output file");

            let idx = uniq.iter().position(|u| u.decls == decls);
            match idx {
                None => {
                    uniq.push(TargetAst {
                        targets: vec![**target],
                        decls,
                    });
                }
                Some(i) => {
                    uniq[i].targets.push(**target);
                }
            };
        });

        let t_sets: Vec<HashSet<_>> = uniq
            .iter()
            .map(|a| HashSet::from_iter(a.targets.iter().filter_map(|t| reprc_zig_map.get(t))))
            .collect();
        for o in t_sets.iter() {
            for i in t_sets.iter() {
                if i != o {
                    let dup: HashSet<_> = i.intersection(o).collect();
                    if !dup.is_empty() {
                        panic!("file {:?} zig targets overlap {:?}\n", dir, dup);
                    }
                }
            }
        }
        let mut count = 1;
        for x in uniq {
            write!(out_str, "// MAPPING|{}|", ENGLISH[count]).unwrap();
            x.targets
                .iter()
                .filter_map(|t| {
                    if let Some(z) = reprc_zig_map.get(t) {
                        Some((z, system_compiler(*t)))
                    } else {
                        None
                    }
                })
                .for_each(|t| write!(out_str, "{}:{:?}|", t.0, t.1).unwrap());
            write!(out_str, "END\n").unwrap();
            write!(out_str, "// repr targets ").unwrap();
            for t in x.targets {
                write!(out_str, "{:?}|{:?} ", t, reprc_zig_map.get(&t)).unwrap();
            }
            writeln!(out_str, "").unwrap();
            if count == 1 {
                writeln!(out_str, "#ifdef {}", ENGLISH[count]).unwrap();
            } else {
                writeln!(out_str, "#elif defined({})", ENGLISH[count]).unwrap();
            }
            count += 1;
            for d in x.decls {
                // println!("\n\nname = {}", d.name);
                if let DeclarationType::Type(t) = d.ty {
                    if let Some(type_layout) = t.layout {
                        let align = type_layout.field_alignment_bits / 8;
                        let size = type_layout.size_bits / 8;
                        writeln!(
                            out_str,
                            "_Static_assert(sizeof({}) == {}, \"\");",
                            d.name, size
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(_Alignof({}) == {}, \"\");",
                            d.name, align
                        )
                        .unwrap();
                        writeln!(out_str, "#ifdef EXTRA_TESTS").unwrap();
                        let d_size = if size > 0 {
                            align_to(size, align).unwrap()
                        } else {
                            0
                        };
                        writeln!(
                            out_str,
                            "_Static_assert(sizeof(struct {}_alignment) == {}, \"\");",
                            d.name,
                            d_size + align
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(_Alignof(struct {}_alignment) == {}, \"\");",
                            d.name, align
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(sizeof(struct {}_packed) == {}, \"\");",
                            d.name, size
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(_Alignof(struct {}_packed) == {}, \"\");",
                            d.name, 1
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(sizeof(struct {}_required_alignment) == {}, \"\");",
                            d.name,
                            size + 1
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(_Alignof(struct {}_required_alignment) == {}, \"\");",
                            d.name, 1
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(sizeof(struct {}_size) == {}, \"\");",
                            d.name,
                            size + 2
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(_Alignof(struct {}_size) == {}, \"\");",
                            d.name, 1
                        )
                        .unwrap();
                        writeln!(out_str, "#endif").unwrap();
                        if let TypeVariant::Record(r) = t.variant {
                            if r.fields.len() > 1 {
                                writeln!(out_str, "#ifdef CHECK_OFFSETS").unwrap();
                                // the first one is always zero. Can skip
                                for f in r.fields.into_iter().skip(1) {
                                    if let (Some(name), Some(layout)) = (f.name, f.layout) {
                                        // println!("\n\t\t{} = {}", name, layout.offset_bits);
                                        writeln!(out_str, "_Static_assert(__builtin_bitoffsetof({0},{1}) == {2}, \"\");", d.name, name, layout.offset_bits).unwrap();
                                    }
                                }
                                writeln!(out_str, "#endif").unwrap();
                            }
                        }
                    }
                }
            }
        }
        writeln!(out_str, "#endif").unwrap();
        std::fs::write(&f_name, out_str).unwrap();
    }
    Ok(())
}

fn merge_two(c: String, m: String, out_str: &mut String) {
    let mut c_idx = 0;
    let mut m_idx = 0;
    let c_v: Vec<_> = c.lines().map(|s| s.to_string()).collect();
    let m_v: Vec<_> = m.lines().map(|s| s.to_string()).collect();
    let mut c_a = 0;
    let mut is_open = false;
    while c_idx < c_v.len() && m_idx < m_v.len() {
        if c_v[c_idx + c_a] == m_v[m_idx] {
            if is_open {
                writeln!(out_str, "#else").unwrap();
                let cur = c_idx + c_a;
                while c_idx < cur {
                    writeln!(out_str, "{}", &c_v[c_idx]).unwrap();
                    c_idx += 1;
                    // println!("{} {}", c_idx, c_idx+c_a );
                }
                // println!("le");
                writeln!(out_str, "#endif").unwrap();
                is_open = false;
                c_a = 0;
            }
            writeln!(out_str, "{}", &c_v[c_idx]).unwrap();
            c_idx += 1;
            m_idx += 1;
        } else {
            if !is_open {
                is_open = true;
                writeln!(out_str, "#ifdef MSVC").unwrap();
            }
            writeln!(out_str, "{}", &m_v[m_idx]).unwrap();
            m_idx += 1;
            c_a += 1;
        }
    }
}
