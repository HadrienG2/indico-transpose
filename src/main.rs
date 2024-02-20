#![allow(unused)]

use serde::Deserialize;
use std::{collections::HashMap, rc::Rc};

#[derive(Debug, Deserialize)]
struct Identity {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Email Address")]
    email_address: String,
    #[serde(rename = "Affiliation")]
    affiliation: String,
}

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

fn main() -> csv::Result<()> {
    let mut rdr = csv::Reader::from_path("registrations.csv")?;
    let mut registrations_by_module = HashMap::<_, Vec<_>>::new();
    for result in rdr.deserialize() {
        let record: Record = result?;
        let identity = Rc::new(record.identity);
        for module in record.choice_of_modules.split(';') {
            registrations_by_module
                .entry(module.to_owned())
                .or_default()
                .push(identity.clone());
        }
    }

    println!("# Registrations\n");
    for (module, registrations) in registrations_by_module {
        println!("## {module}\n");
        for registration in registrations {
            println!("- {registration:?}");
        }
        println!();
    }
    Ok(())
}
