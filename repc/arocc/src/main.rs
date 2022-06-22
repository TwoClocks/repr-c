// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;
use std::fmt::Write;
// use std::fs::File;
// use std::mem;
// use std::path::PathBuf;
use std::process;

use cly_impl::ast::{Declaration, DeclarationType, TypeVariant};
// use cly_impl::converter::ConversionResult;
use repc_impl::target::{system_compiler, Compiler, Target, TARGETS};
use repc_tests::read_input_config;

#[path = "../../test-generator/src/c.rs"]
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
fn main_() -> std::io::Result<()> {
    // let userconfig: GlobalConfig = toml::from_str(&std::fs::read_to_string("config.toml")?)?;

    // let skip: HashSet<&str> = HashSet::from([
    //     // typdef align
    //     "0007", "0008", "0010", "0011", "0014", "0028", "0029", "0044", "0045", "0046", "0058",
    //     "0066", "0070", "0080", "0081", "0082", "0084", "0085", "0086",
    //     // // enum attr packed issue.
    //     "0055", "0060", "0062", // assert
    //     "0050", // clang fails
    //     "0083",
    // ]);

    let mut dirs = vec![];
    for dir in std::fs::read_dir("testfiles").expect("Can't open testfiles dir") {
        let dir = dir?;
        if dir.file_type()?.is_dir() {
            dirs.push(dir.path());
        }
    }
    dirs.sort();
    dirs.iter().for_each(|dir| {
        let config = read_input_config(dir).unwrap();
        let input_path = dir.join("input.txt");
        let input = std::fs::read_to_string(&input_path).expect("akjhsd");
        // let hash = {
        //     let mut hash = DefaultHasher::new();
        //     input.hash(&mut hash);
        //     config.0.hash(&mut hash);
        //     hash.finish()
        // };
        let declarations = cly_impl::parse(&input).unwrap();

        let targets:Vec<_> = TARGETS.iter().filter(|t| { config.1.test_target(**t) }).collect();
        // if targets.contains(&&Target::X86_64UnknownLinuxGnu) {
            let mut comps: HashSet<_> = targets.iter().map(|t| { system_compiler(**t) }).collect();

            if comps.contains(&Compiler::Clang) && comps.contains(&Compiler::Gcc) {
                comps.remove(&Compiler::Gcc);
            }

            let dir_name = dir.file_name().unwrap().to_str().unwrap();
            let f_name = format!("{}_test.c", dir_name);
            // println!("{}", f_name);
            let mut out_str = "// SPDX-License-Identifier: GPL-3.0-or-later\n\n\
            // This test file is auto-generated. do not edit.\n\
            // This file is a derivative work from the test files found\
            // in this repo : https://github.com/mahkoh/repr-c\n\
            // and is under the same licence as the original work.\n\n".to_string();

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
                (None, None) => unreachable!()
            };

            #[derive(Debug)]
            struct TargetAst {
                targets: Vec<Target>,
                decls: Vec<Declaration>,
            }

            let mut uniq: Vec<TargetAst> = Vec::new();

            targets.iter().for_each( |target| {
                // let target = &&Target::X86_64UnknownLinuxGnu;
                let tname = target.name();
                let exp_file = dir.join("output").join(format!("{}.expected.txt", tname));
                let exp_str = std::fs::read_to_string(&exp_file).expect("can't open output file");
                let decls = cly_impl::parse(&exp_str).expect("can't open output file");
                // let ast = cly_impl::extract_layouts( &exp_str, &decls).expect("can't parse output file");

                let idx = uniq.iter().position(|u| {
                    u.decls == decls
                });
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
                // println!("{:?}",ast);

                // panic!("done");
                // });
                let mut first = true;
                for x in uniq {
                    if first {
                        first = false;
                        write!(out_str, "#if ").unwrap();
                    } else {
                        write!(out_str, "#elif ").unwrap();
                    }
                    let t_def = x.targets.iter()
                        .map(|s| format!("defined({})",s.name().to_string().replace("_","").replace("-","_").replace(".","").to_uppercase()))
                        .reduce(|a,b| format!("{} || {}", a, b));
                    let mut line_len=4;

                    for part in t_def.unwrap().split_inclusive("||") {
                        if line_len > 125 {
                            writeln!(out_str," \\").unwrap();
                            line_len = 0;
                        }
                        write!(out_str, "{} ", part).unwrap();
                        line_len += part.len();
                    }
                    writeln!(out_str,"").unwrap();
                    // writeln!(out_str, "{}", t_def.unwrap()).unwrap();
                    for d in x.decls {
                        // println!("\n\nname = {}", d.name);
                        if let DeclarationType::Type(t) = d.ty {
                            if let Some(type_layout) = t.layout {
                                let align = type_layout.field_alignment_bits / 8;
                                let size = type_layout.size_bits / 8;
                                writeln!(out_str, "_Static_assert(sizeof({}) == {}, \"record {0} wrong sizeof\");", d.name, size).unwrap();
                                writeln!(out_str, "_Static_assert(_Alignof({}) == {}, \"record {0} wrong alignment\");", d.name, align).unwrap();
                                writeln!(out_str, "#ifdef EXTRA_TESTS").unwrap();
                                let d_size = if size > 0 { size.max(align) } else { 0 };
                                writeln!(out_str, "_Static_assert(sizeof(struct {}_alignment) == {}, \"record {0} wrong sizeof\");", d.name, align+d_size).unwrap();
                                writeln!(out_str, "_Static_assert(_Alignof(struct {}_alignment) == {}, \"record {0} wrong alignment\");", d.name, align).unwrap();
                                writeln!(out_str, "_Static_assert(sizeof(struct {}_packed) == {}, \"record {0} wrong sizeof\");", d.name, size).unwrap();
                                writeln!(out_str, "_Static_assert(_Alignof(struct {}_packed) == {}, \"record {0} wrong alignment\");", d.name,1 ).unwrap();
                                writeln!(out_str, "_Static_assert(sizeof(struct {}_required_alignment) == {}, \"record {0} wrong sizeof\");", d.name, size+1).unwrap();
                                writeln!(out_str, "_Static_assert(_Alignof(struct {}_required_alignment) == {}, \"record {0} wrong alignment\");", d.name,1 ).unwrap();
                                writeln!(out_str, "_Static_assert(sizeof(struct {}_size) == {}, \"record {0} wrong sizeof\");", d.name, size+2).unwrap();
                                writeln!(out_str, "_Static_assert(_Alignof(struct {}_size) == {}, \"record {0} wrong alignment\");", d.name,1 ).unwrap();
                                writeln!(out_str, "#endif").unwrap();
                                if let TypeVariant::Record(r) = t.variant {
                                    if r.fields.len() > 1 {
                                        writeln!(out_str, "#ifdef CHECK_OFFSETS").unwrap();
                                        // the first one is always zero. Can skip
                                        for f in r.fields.into_iter().skip(1) {
                                            if let (Some(name), Some(layout)) = (f.name, f.layout) {
                                                // println!("\n\t\t{} = {}", name, layout.offset_bits);
                                                writeln!(out_str, "_Static_assert(__builtin_bitoffsetof({0},{1}) == {2}, \"field {0}.{1} wrong bit offset\");", d.name, name, layout.offset_bits).unwrap();
                                            }
                                        }
                                        writeln!(out_str, "#endif").unwrap();
                                    }
                                }
                            }
                        }
                    }
                    // std::fs::write(&f_name, out_str).unwrap();
                    //
                    // // panic!("done");
                    // break;
                    // println!("{:?}", x.ast.types);
                    // panic!("done");
                }
            writeln!(out_str, "#endif").unwrap();
            std::fs::write(&f_name, out_str).unwrap();
        });
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
