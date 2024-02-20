use clap::Parser;
use serde::Deserialize;
use std::{collections::HashMap, rc::Rc};

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
        println!("DEBUG: Parsing registration records...\n");
    }
    for result in csv_reader.deserialize() {
        if args.debug {
            println!("{result:#?}");
        }
        let record: Record = result?;
        let identity = Rc::new(record.identity);
        for module in record.choice_of_modules.split(';') {
            registrations_by_module
                .entry(module.to_owned())
                .or_default()
                .push(identity.clone());
        }
    }
    if args.debug {
        println!("\n");
    }

    // Display final per-module registration
    println!("# Registrations to each module\n");
    for (module, registrations) in registrations_by_module {
        println!("## {module}\n");
        for registration in registrations {
            println!("- {registration:?}");
        }
        println!();
    }
    Ok(())
}
