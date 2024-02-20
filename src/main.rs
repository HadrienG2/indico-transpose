use clap::Parser;
use regex::Regex;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

/// Information about people that we certainly care about right now
#[allow(unused)]
#[derive(Debug, Deserialize)]
struct Identity {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Email Address")]
    email_address: String,
    #[serde(rename = "Affiliation")]
    affiliation: String,
}

/// Full indico record featuring a bit of extra info we may care about someday
#[allow(unused)]
#[derive(Debug, Deserialize)]
struct Record {
    #[serde(rename = "ID")]
    id: usize,
    #[serde(flatten)]
    identity: Identity,
    #[serde(rename = "Choice of modules")]
    choice_of_modules: String,
    #[serde(rename = "Registration state")]
    registration_state: String,
    #[serde(rename = "Checked in")]
    checked_in: String,
}

/// Translate Indico's per-user registrations into per-course registrations
#[derive(Parser)]
struct Args {
    /// Log raw user registration records for debugging
    #[arg(long, default_value_t = false)]
    debug: bool,

    /// Path to Indico CSV
    #[arg(default_value_t = String::from("registrations.csv".to_owned()))]
    input_path: String,
}

fn main() -> csv::Result<()> {
    // Set up app
    let args = Args::parse();
    let mut csv_reader = csv::Reader::from_path(args.input_path)?;
    let mut registrations_by_module = HashMap::<_, Vec<_>>::new();

    // Parse registration records, group identities by module
    if args.debug {
        eprintln!("DEBUG: Parsing registration records...\n");
    }
    for result in csv_reader.deserialize() {
        if args.debug {
            eprintln!("{result:#?}");
        }
        let record: Record = result?;
        let identity = Rc::new(record.identity);
        for module in record.choice_of_modules.split(';') {
            registrations_by_module
                .entry(module.trim().to_owned())
                .or_default()
                .push(identity.clone());
        }
    }
    if args.debug {
        eprintln!("\n");
    }

    // Sort modules by starting date, if available
    if args.debug {
        eprintln!("DEBUG: Sorting modules by starting date...");
    }
    let date_regex = Regex::new(r"([0-9]{1,2})/([0-9]{1,2})").unwrap();
    let mut modules_by_date = BTreeMap::new();
    for (idx, module_name) in registrations_by_module.keys().enumerate() {
        let timestamp = if let Some((_, [day, month])) =
            date_regex.captures(module_name).map(|cap| cap.extract())
        {
            let day = day.parse::<usize>().unwrap();
            let month = month.parse::<usize>().unwrap();
            month * 100 + day
        } else {
            if args.debug {
                eprintln!("WARNING: Couldn't parse start date of module {module_name}, it will be unordered in output");
            }
            10000 + idx
        };
        modules_by_date.insert(timestamp, module_name);
    }
    if args.debug {
        eprintln!();
    }

    // Display final per-module registration
    println!("# Registrations to each module");
    for (_, module) in modules_by_date {
        println!("\n## {module}\n");
        for registration in &registrations_by_module[module] {
            println!("- {registration:?}");
        }
    }
    Ok(())
}
