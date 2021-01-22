use anyhow::{anyhow, Context, Result};
use clap::{App, Arg};
use repr_c::target::{Target, TARGETS};
use std::fs::File;
use std::io::{stdin, Read};
use std::process;

fn args() -> (&'static dyn Target, Option<String>) {
    let possible_targets: Vec<_> = TARGETS.iter().map(|t| t.name()).collect();
    let matches = App::new("c-layout")
        .arg(
            Arg::with_name("print-targets")
                .long("print-targets")
                .help("Prints all available targets"),
        )
        .arg(
            Arg::with_name("target")
                .long("target")
                .takes_value(true)
                .help("Sets the target")
                .possible_values(&possible_targets),
        )
        .arg(Arg::with_name("input").required(false))
        .get_matches();
    if matches.is_present("print-targets") {
        for t in TARGETS {
            println!("{}", t.name());
        }
        process::exit(0);
    }
    let target = matches.value_of("target").unwrap_or(env!("TARGET"));
    let target = TARGETS
        .into_iter()
        .copied()
        .filter(|t| t.name() == target)
        .next();
    let target = match target {
        None => {
            eprintln!("The default target {} is not available.", env!("TARGET"));
            eprintln!("Specify a different target with the --target option.");
            eprintln!("Print all available targets with the --print-targets flag.");
            process::exit(1);
        }
        Some(t) => t,
    };
    (target, matches.value_of("input").map(|s| s.to_owned()))
}

fn main() {
    if let Err(e) = main_() {
        eprintln!("{:#}", e);
    }
}

fn main_() -> Result<()> {
    let (target, file) = args();
    let mut input = String::new();
    match file {
        Some(p) => File::open(&p)
            .with_context(|| anyhow!("cannot open {}", p))?
            .read_to_string(&mut input)
            .with_context(|| anyhow!("cannot read from {}", p))?,
        _ => stdin()
            .read_to_string(&mut input)
            .context("cannot read from stdin")?,
    };
    let res = c_layout_priv::parse(&input).context("Parsing failed")?;
    let layouts = c_layout_priv::compute_layouts(&input, &res, target)
        .context("Layout computation failed")?;
    let res = c_layout_priv::enhance_declarations(&res, &layouts);
    print!("{}", c_layout_priv::printer(&input, &res));
    Ok(())
}
