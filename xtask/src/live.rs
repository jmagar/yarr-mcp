use anyhow::{bail, Result};

pub mod guard;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Suite {
    Guard,
    Cli,
    Rest,
    Mcp,
    Services,
    All,
}

pub fn run(args: &[String]) -> Result<()> {
    let options = Options::parse(args)?;
    match options.suite {
        Suite::Guard => {
            let guarded = guard::load(None, options.allow_partial)?;
            println!(
                "PASS guard complete shart env: {} services",
                guarded.services.len()
            );
            Ok(())
        }
        _ => bail!("suite {:?} is not implemented yet", options.suite),
    }
}

#[derive(Debug)]
struct Options {
    suite: Suite,
    allow_partial: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self> {
        let mut suite = Suite::All;
        let mut allow_partial = false;
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                "--allow-partial" => allow_partial = true,
                "--suite" => {
                    index += 1;
                    let value = args.get(index).map(String::as_str).unwrap_or("");
                    suite = match value {
                        "guard" => Suite::Guard,
                        "cli" => Suite::Cli,
                        "rest" => Suite::Rest,
                        "mcp" => Suite::Mcp,
                        "services" => Suite::Services,
                        "all" => Suite::All,
                        _ => bail!("unknown live suite: {value}"),
                    };
                }
                other => bail!("unknown live option: {other}"),
            }
            index += 1;
        }
        Ok(Self {
            suite,
            allow_partial,
        })
    }
}

fn print_help() {
    println!("cargo xtask live --suite <guard|cli|rest|mcp|services|all>");
    println!("  --allow-partial  Only permitted for legacy live-read-smoke guard checks");
}
