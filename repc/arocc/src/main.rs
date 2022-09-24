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
use lazy_static::lazy_static;
use repc_impl::target::{system_compiler, Compiler, Target, TARGETS};
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
lazy_static! {
    static ref REPR_RUST: HashMap<Target, &'static str> = {
        let mut map = HashMap::new();
        map.insert(Target::Aarch64AppleMacosx, "aarch64-apple-darwin");
        map.insert(Target::Aarch64Fuchsia, "aarch64-fuchsia");
        map.insert(Target::Aarch64LinuxAndroid, "aarch64-linux-android");
        map.insert(Target::Aarch64PcWindowsMsvc, "aarch64-pc-windows-msvc");
        map.insert(Target::Aarch64UnknownFreebsd, "aarch64-unknown-freebsd");
        map.insert(Target::Aarch64UnknownHermit, "aarch64-unknown-hermit");
        map.insert(Target::Aarch64UnknownLinuxGnu, "aarch64-unknown-linux-gnu");
        map.insert(
            Target::Aarch64UnknownLinuxMusl,
            "aarch64-unknown-linux-musl",
        );
        map.insert(Target::Aarch64UnknownNetbsd, "aarch64-unknown-netbsd");
        map.insert(Target::Aarch64UnknownNone, "aarch64-unknown-none");
        map.insert(Target::Aarch64UnknownOpenbsd, "aarch64-unknown-openbsd");
        map.insert(Target::Aarch64UnknownRedox, "aarch64-unknown-redox");
        map.insert(Target::Arm64AppleIos, "aarch64-apple-ios");
        map.insert(Target::Arm64AppleIosMacabi, "aarch64-apple-ios-macabi");
        map.insert(Target::Arm64AppleTvos, "aarch64-apple-tvos");
        map.insert(Target::ArmLinuxAndroideabi, "arm-linux-androideabi");
        map.insert(Target::ArmUnknownLinuxGnueabi, "arm-unknown-linux-gnueabi");
        map.insert(
            Target::ArmUnknownLinuxGnueabihf,
            "arm-unknown-linux-gnueabihf",
        );
        map.insert(Target::Armebv7rUnknownNoneEabi, "armebv7r-none-eabi");
        map.insert(Target::Armebv7rUnknownNoneEabihf, "armebv7r-none-eabihf");
        map.insert(
            Target::Armv4tUnknownLinuxGnueabi,
            "armv4t-unknown-linux-gnueabi",
        );
        map.insert(
            Target::Armv5teUnknownLinuxGnueabi,
            "armv5te-unknown-linux-gnueabi",
        );
        map.insert(
            Target::Armv5teUnknownLinuxUclibcgnueabi,
            "armv5te-unknown-linux-uclibceabi",
        );
        map.insert(
            Target::Armv6UnknownFreebsdGnueabihf,
            "armv6-unknown-freebsd",
        );
        map.insert(
            Target::Armv6UnknownNetbsdelfEabihf,
            "armv6-unknown-netbsd-eabihf",
        );
        map.insert(Target::Armv7AppleIos, "armv7-apple-ios");
        map.insert(Target::Armv7NoneLinuxAndroid, "armv7-linux-androideabi");
        map.insert(
            Target::Armv7UnknownFreebsdGnueabihf,
            "armv7-unknown-freebsd",
        );
        map.insert(
            Target::Armv7UnknownLinuxGnueabi,
            "armv7-unknown-linux-gnueabi",
        );
        map.insert(
            Target::Armv7UnknownLinuxGnueabihf,
            "armv7-unknown-linux-gnueabihf",
        );
        map.insert(
            Target::Armv7UnknownNetbsdelfEabihf,
            "armv7-unknown-netbsd-eabihf",
        );
        map.insert(Target::Armv7aNoneEabi, "armv7a-none-eabi");
        map.insert(Target::Armv7aNoneEabihf, "armv7a-none-eabihf");
        map.insert(Target::Armv7rUnknownNoneEabi, "armv7r-none-eabi");
        map.insert(Target::Armv7rUnknownNoneEabihf, "armv7r-none-eabihf");
        map.insert(Target::Armv7sAppleIos, "armv7s-apple-ios");
        map.insert(Target::AvrUnknownUnknown, "avr-unknown-gnu-atmega328");
        map.insert(
            Target::HexagonUnknownLinuxMusl,
            "hexagon-unknown-linux-musl",
        );
        map.insert(Target::I386AppleIos, "i386-apple-ios");
        map.insert(Target::I586PcWindowsMsvc, "i586-pc-windows-msvc");
        map.insert(Target::I586UnknownLinuxGnu, "i586-unknown-linux-gnu");
        map.insert(Target::I586UnknownLinuxMusl, "i586-unknown-linux-musl");
        map.insert(Target::I686AppleMacosx, "i686-apple-darwin");
        map.insert(Target::I686LinuxAndroid, "i686-linux-android");
        map.insert(Target::I686PcWindowsGnu, "i686-pc-windows-gnu");
        map.insert(Target::I686PcWindowsMsvc, "i686-pc-windows-msvc");
        map.insert(Target::I686UnknownFreebsd, "i686-unknown-freebsd");
        map.insert(Target::I686UnknownHaiku, "i686-unknown-haiku");
        map.insert(Target::I686UnknownLinuxGnu, "i686-unknown-linux-gnu");
        map.insert(Target::I686UnknownLinuxGnu, "i686-wrs-vxworks");
        map.insert(Target::I686UnknownLinuxMusl, "i686-unknown-linux-musl");
        map.insert(Target::I686UnknownNetbsdelf, "i686-unknown-netbsd");
        map.insert(Target::I686UnknownOpenbsd, "i686-unknown-openbsd");
        map.insert(Target::I686UnknownWindows, "i686-unknown-uefi");
        map.insert(
            Target::Mips64UnknownLinuxGnuabi64,
            "mips64-unknown-linux-gnuabi64",
        );
        map.insert(
            Target::Mips64UnknownLinuxMusl,
            "mips64-unknown-linux-muslabi64",
        );
        map.insert(
            Target::Mips64elUnknownLinuxGnuabi64,
            "mips64el-unknown-linux-gnuabi64",
        );
        map.insert(
            Target::Mips64elUnknownLinuxMusl,
            "mips64el-unknown-linux-muslabi64",
        );
        map.insert(Target::MipsUnknownLinuxGnu, "mips-unknown-linux-gnu");
        map.insert(Target::MipsUnknownLinuxMusl, "mips-unknown-linux-musl");
        map.insert(Target::MipsUnknownLinuxUclibc, "mips-unknown-linux-uclibc");
        map.insert(Target::MipselSonyPsp, "mipsel-sony-psp");
        map.insert(Target::MipselUnknownLinuxGnu, "mipsel-unknown-linux-gnu");
        map.insert(Target::MipselUnknownLinuxMusl, "mipsel-unknown-linux-musl");
        map.insert(
            Target::MipselUnknownLinuxUclibc,
            "mipsel-unknown-linux-uclibc",
        );
        map.insert(Target::MipselUnknownNone, "mipsel-unknown-none");
        map.insert(
            Target::Mipsisa32r6UnknownLinuxGnu,
            "mipsisa32r6-unknown-linux-gnu",
        );
        map.insert(
            Target::Mipsisa32r6elUnknownLinuxGnu,
            "mipsisa32r6el-unknown-linux-gnu",
        );
        map.insert(
            Target::Mipsisa64r6UnknownLinuxGnuabi64,
            "mipsisa64r6-unknown-linux-gnuabi64",
        );
        map.insert(
            Target::Mipsisa64r6elUnknownLinuxGnuabi64,
            "mipsisa64r6el-unknown-linux-gnuabi64",
        );
        map.insert(Target::Msp430NoneElf, "msp430-none-elf");
        map.insert(Target::Powerpc64UnknownFreebsd, "powerpc64-unknown-freebsd");
        map.insert(
            Target::Powerpc64UnknownLinuxGnu,
            "powerpc64-unknown-linux-gnu",
        );
        map.insert(
            Target::Powerpc64UnknownLinuxMusl,
            "powerpc64-unknown-linux-musl",
        );
        map.insert(
            Target::Powerpc64leUnknownLinuxGnu,
            "powerpc64le-unknown-linux-gnu",
        );
        map.insert(
            Target::Powerpc64leUnknownLinuxMusl,
            "powerpc64le-unknown-linux-musl",
        );
        map.insert(Target::PowerpcUnknownLinuxGnu, "powerpc-unknown-linux-gnu");
        map.insert(
            Target::PowerpcUnknownLinuxGnuspe,
            "powerpc-unknown-linux-gnuspe",
        );
        map.insert(
            Target::PowerpcUnknownLinuxMusl,
            "powerpc-unknown-linux-musl",
        );
        map.insert(Target::PowerpcUnknownNetbsd, "powerpc-unknown-netbsd");
        map.insert(Target::Riscv32, "riscv32i-unknown-none-elf");
        map.insert(
            Target::Riscv32UnknownLinuxGnu,
            "riscv32gc-unknown-linux-gnu",
        );
        map.insert(Target::Riscv64, "riscv64gc-unknown-none-elf");
        map.insert(
            Target::Riscv64UnknownLinuxGnu,
            "riscv64gc-unknown-linux-gnu",
        );
        map.insert(Target::S390xUnknownLinuxGnu, "s390x-unknown-linux-gnu");
        map.insert(Target::Sparc64UnknownLinuxGnu, "sparc64-unknown-linux-gnu");
        map.insert(Target::Sparc64UnknownNetbsd, "sparc64-unknown-netbsd");
        map.insert(Target::Sparc64UnknownOpenbsd, "sparc64-unknown-openbsd");
        map.insert(Target::SparcUnknownLinuxGnu, "sparc-unknown-linux-gnu");
        map.insert(Target::Sparcv9SunSolaris, "sparcv9-sun-solaris");
        map.insert(Target::Thumbv4tNoneEabi, "thumbv4t-none-eabi");
        map.insert(Target::Thumbv6mNoneEabi, "thumbv6m-none-eabi");
        map.insert(Target::Thumbv7aPcWindowsMsvc, "thumbv7a-pc-windows-msvc");
        map.insert(Target::Thumbv7emNoneEabi, "thumbv7em-none-eabi");
        map.insert(Target::Thumbv7emNoneEabihf, "thumbv7em-none-eabihf");
        map.insert(Target::Thumbv7mNoneEabi, "thumbv7m-none-eabi");
        map.insert(Target::Thumbv8mBaseNoneEabi, "thumbv8m.base-none-eabi");
        map.insert(Target::Thumbv8mMainNoneEabihf, "thumbv8m.main-none-eabihf");
        map.insert(Target::Wasm32UnknownEmscripten, "wasm32-unknown-emscripten");
        map.insert(Target::Wasm32UnknownUnknown, "wasm32-unknown-unknown");
        map.insert(Target::Wasm32Wasi, "wasm32-wasi");
        map.insert(Target::X86_64AppleIos, "x86_64-apple-ios");
        map.insert(Target::X86_64AppleIosMacabi, "x86_64-apple-ios-macabi");
        map.insert(Target::X86_64AppleMacosx, "x86_64-apple-darwin");
        map.insert(Target::X86_64AppleTvos, "x86_64-apple-tvos");
        map.insert(Target::X86_64Elf, "x86_64-linux-kernel");
        map.insert(Target::X86_64Fuchsia, "x86_64-fuchsia");
        map.insert(Target::X86_64LinuxAndroid, "x86_64-linux-android");
        map.insert(Target::X86_64PcSolaris, "x86_64-pc-solaris");
        map.insert(Target::X86_64PcWindowsGnu, "x86_64-pc-windows-gnu");
        map.insert(Target::X86_64PcWindowsMsvc, "x86_64-pc-windows-msvc");
        map.insert(Target::X86_64RumprunNetbsd, "x86_64-rumprun-netbsd");
        map.insert(Target::X86_64UnknownDragonfly, "x86_64-unknown-dragonfly");
        map.insert(Target::X86_64UnknownFreebsd, "x86_64-unknown-freebsd");
        map.insert(Target::X86_64UnknownHaiku, "x86_64-unknown-haiku");
        map.insert(Target::X86_64UnknownHermit, "x86_64-unknown-hermit");
        map.insert(
            Target::X86_64UnknownL4reUclibc,
            "x86_64-unknown-l4re-uclibc",
        );
        map.insert(Target::X86_64UnknownLinuxGnu, "x86_64-unknown-linux-gnu");
        map.insert(
            Target::X86_64UnknownLinuxGnux32,
            "x86_64-unknown-linux-gnux32",
        );
        map.insert(Target::X86_64UnknownLinuxMusl, "x86_64-unknown-linux-musl");
        map.insert(Target::X86_64UnknownNetbsd, "x86_64-unknown-netbsd");
        map.insert(Target::X86_64UnknownOpenbsd, "x86_64-unknown-openbsd");
        map.insert(Target::X86_64UnknownRedox, "x86_64-unknown-redox");
        map.insert(Target::X86_64UnknownWindows, "x86_64-unknown-uefi");
        map
    };
}

fn main() {
    if let Err(e) = main_() {
        eprintln!("{:?}", e);
        process::exit(1);
    }
}

/// this is ugly ugly ugly code.
/// it's only intended to be run once, to generate the test files
/// for arocc. Don't judge me.
/// this is meant to be run in the /reprc/tests directory.
/// you will also need a `mapping.txt` file in the same directoy.
/// The zig code in /repr/arocc generrates this file.
fn main_() -> std::io::Result<()> {
    let mut reprc_zig_map: HashMap<Target, String> = HashMap::new();
    {
        let mut rust_zig_map: HashMap<String, String> = HashMap::new();

        let m_file = std::fs::read_to_string("mapping.text")?;
        m_file.lines().for_each(|l| {
            let x = l.split_once(':').unwrap();
            rust_zig_map.insert(x.0.to_owned(), x.1.to_owned());
        });

        for reprc in TARGETS {
            assert!(!reprc_zig_map.contains_key(reprc));
            // the reprc targets are more accurate,
            // so see if a mapping exits and use that.
            // except for windows targets. Use the rustc ones.
            let repr = rust_zig_map.get(reprc.name());
            let rust = match REPR_RUST.get(reprc) {
                Some(r) => rust_zig_map.get(*r),
                None => None,
            };
            match (repr, rust) {
                (Some(r), None) => _ = reprc_zig_map.insert(*reprc, r.clone()),
                (None, Some(u)) => _ = reprc_zig_map.insert(*reprc, u.clone()),
                (Some(r), Some(u)) => {
                    if r.contains("windows") || r.contains("uefi") {
                        reprc_zig_map.insert(*reprc, u.clone());
                    } else {
                        reprc_zig_map.insert(*reprc, r.clone());
                    };
                }
                (None, None) => println!("no map for {:?}", reprc),
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
        // This file is a derivative work from the test files found\n\
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
                        let d_size = if size > 0 {
                            align_to(size, align).unwrap()
                        } else {
                            0
                        };
                        writeln!(
                            out_str,
                            "_Static_assert(sizeof(struct {}_extra_alignment) == {}, \"\");",
                            d.name,
                            d_size + align
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(_Alignof(struct {}_extra_alignment) == {}, \"\");",
                            d.name, align
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(sizeof(struct {}_extra_packed) == {}, \"\");",
                            d.name, size
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(_Alignof(struct {}_extra_packed) == {}, \"\");",
                            d.name, 1
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(sizeof(struct {}_extra_required_alignment) == {}, \"\");",
                            d.name,
                            size + 1
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(_Alignof(struct {}_extra_required_alignment) == {}, \"\");",
                            d.name, 1
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(sizeof(struct {}_extra_size) == {}, \"\");",
                            d.name,
                            size + 2
                        )
                        .unwrap();
                        writeln!(
                            out_str,
                            "_Static_assert(_Alignof(struct {}_extra_size) == {}, \"\");",
                            d.name, 1
                        )
                        .unwrap();
                        if let TypeVariant::Record(r) = t.variant {
                            if r.fields.len() > 1 {
                                writeln!(out_str, "#ifndef SKIP_OFFSETS").unwrap();
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
