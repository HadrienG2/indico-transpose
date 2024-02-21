use clap::Parser;
use log::{debug, warn};
use regex::Regex;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    fs::File,
    rc::Rc,
    sync::{Arc, OnceLock},
};
use time::{Date, Month, OffsetDateTime, Time};

// === CLI starts here ===

/// Translate Indico's per-user registrations into per-course registrations
#[derive(Parser)]
struct Args {
    /// Path to Indico CSV
    #[arg(default_value_t = String::from("registrations.csv".to_owned()))]
    input_path: String,
}

fn main() -> csv::Result<()> {
    // Set up app
    env_logger::init();
    let args = Args::parse();

    // Read out raw CSV records
    let csv_reader = csv::Reader::from_path(args.input_path)?;
    let raw_records = load_raw_records(csv_reader)?;

    // Translate records into a more exploitable data layout
    let registrations = Registrations::new(raw_records);

    // Order people by registration time
    let persons_by_registration_time = registrations
        .persons
        .iter()
        .enumerate()
        .map(|(person_id, person)| (person.registration_time, person_id))
        .collect::<BTreeMap<OffsetDateTime, PersonId>>();
    if log::max_level() >= log::Level::Debug {
        debug!("People ordered by registration time");
        for (date, person_id) in &persons_by_registration_time {
            debug!(
                "- {} ({})",
                registrations.persons[*person_id].identity, date
            );
        }
    }

    // For each module, produce a matching ordered list of who registered
    let mut module_to_ordered_persons =
        HashMap::<ModuleId, Vec<PersonId>>::with_capacity(registrations.modules.len());
    for (_, person_id) in persons_by_registration_time {
        for &module_id in &registrations.persons[person_id].choice_of_modules {
            module_to_ordered_persons
                .entry(module_id)
                .or_default()
                .push(person_id);
        }
    }

    // Order modules by start time
    let modules_by_start_time = registrations
        .modules
        .iter()
        .enumerate()
        .map(|(module_id, module)| (module.start_time, module_id))
        .collect::<BTreeMap<OffsetDateTime, ModuleId>>();

    // Display module registrations
    println!("# Registrations to each module");
    for (_, module_id) in modules_by_start_time {
        println!("\n## {}\n", registrations.modules[module_id].name);
        for (idx, &person_id) in module_to_ordered_persons[&module_id].iter().enumerate() {
            println!("{}. {}", idx + 1, registrations.persons[person_id].identity);
        }
    }
    Ok(())
}

// === Input data from Indico ===

/// Indico registration record
///
/// The optional fields are those which I'm not using yet, but which sounded
/// interesting and which I'm considering for future use.
#[allow(unused)]
#[derive(Debug, Deserialize)]
struct CSVRecord {
    #[serde(rename = "ID")]
    id: Option<usize>,
    #[serde(flatten)]
    identity: Identity,
    #[serde(rename = "Choice of modules")]
    choice_of_modules: Box<str>,
    #[serde(rename = "Registration date", with = "indico_datetime")]
    registration_time: OffsetDateTime,
    #[serde(rename = "Registration state")]
    registration_state: Option<Box<str>>,
}

/// Basic information about people that we want to display in the end
#[derive(Debug, Deserialize)]
struct Identity {
    #[serde(rename = "Name")]
    name: Box<str>,
    #[serde(rename = "Email Address")]
    email_address: Box<str>,
    #[serde(rename = "Affiliation")]
    affiliation: Box<str>,
}
//
impl Display for Identity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "`{} <{}>`", self.name, self.email_address)?;
        if !self.affiliation.is_empty() {
            static SIMPLIFIED_AFFILIATIONS: OnceLock<HashMap<Box<str>, Arc<str>>> = OnceLock::new();
            let simplified_affiliations = SIMPLIFIED_AFFILIATIONS.get_or_init(|| {
                let mut result = HashMap::new();
                let ijclab = Arc::<str>::from("IJCLab");
                result.insert("Laboratoire de Physique des 2 infinis Irène Joliot-Curie, Université Paris-Saclay, CNRS-IN2P3. Université Paris-Saclay, CNRS-IN2P3".into(), ijclab.clone());
                result.insert("IJCLAB - IN2P3 - CNRS".into(), ijclab.clone());
                result.insert("IJCLab - IN2P3 - CNRS".into(), ijclab.clone());
                result
            });
            let affiliation =
                if let Some(simplified) = simplified_affiliations.get(&*self.affiliation) {
                    simplified
                } else {
                    &*self.affiliation
                };
            write!(f, " from {affiliation}")?;
        }
        Ok(())
    }
}

// Date/time format used by Indico
time::serde::format_description!(
    indico_datetime,
    OffsetDateTime,
    "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond][offset_hour sign:mandatory]:[offset_minute]"
);

/// Load CSV records
fn load_raw_records(mut csv_reader: csv::Reader<File>) -> csv::Result<Vec<CSVRecord>> {
    debug!("Loading CSV registration records...");
    let mut result = Vec::new();
    for record in csv_reader.deserialize() {
        debug!("- {record:#?}");
        let record: CSVRecord = record?;
        result.push(record);
    }
    Ok(result)
}

// === Pre-digested data ===

/// Exploitable version of the Indico registration records
#[derive(Debug, Default)]
struct Registrations {
    /// List of people who registered to courses
    persons: Vec<Person>,

    /// List of pedagogical modules
    modules: Vec<Module>,
}

/// Index of a person within Registrations::persons
type PersonId = usize;

/// Index of a module within Registrations::modules
type ModuleId = usize;

/// What we need to know about someone who registered to modules
#[derive(Debug)]
struct Person {
    /// Information identifying this person
    identity: Identity,

    /// Which modules they chose to attend
    choice_of_modules: Vec<ModuleId>,

    /// Time at which they registered
    registration_time: OffsetDateTime,
}

/// What we know about a module
#[derive(Debug)]
struct Module {
    /// Name of the module
    name: Rc<str>,

    /// Date and time at which the module will start
    start_time: OffsetDateTime,
}
//
impl Module {
    /// Create a new module entry from the module name in Indico CSV
    fn new(module_name: &str) -> Self {
        debug!("- Registered new module: {module_name}");
        static START_TIME_REGEX: OnceLock<Regex> = OnceLock::new();
        let start_time_regex = START_TIME_REGEX.get_or_init(|| {
            Regex::new(
                r"([0-9]{1,2})/([0-9]{1,2})(?: \+ [a-z]+. [0-9]+/[0-9]+)?, ([0-9]{1,2})[:h]([0-9]{1,2})",
            )
            .expect("Regex was manually checked")
        });
        let start_time = if let Some((_, day_month_hour_min)) = start_time_regex
            .captures(module_name)
            .map(|cap| cap.extract())
        {
            let [day, month, hour, min] = day_month_hour_min.map(|s| s.parse::<usize>().unwrap());
            OffsetDateTime::new_utc(
                Date::from_calendar_date(2024, Month::January.nth_next(month as u8 - 1), day as u8)
                    .expect("Module date should be valid"),
                Time::from_hms(hour as u8, min as u8, 0).expect("Module time should be valid"),
            )
        } else {
            warn!(
                "Couldn't parse start time of module \"{module_name}\", it will be unordered in output"
            );
            OffsetDateTime::new_utc(Date::MAX, Time::MIDNIGHT)
        };
        Self {
            name: module_name.into(),
            start_time,
        }
    }
}

impl Registrations {
    /// Translate raw Indico records into a more exploitable form
    fn new(raw_records: Vec<CSVRecord>) -> Self {
        debug!("Post-processing registration records...");
        let mut result = Self::default();
        let mut module_to_id = HashMap::new();
        for CSVRecord {
            identity,
            choice_of_modules,
            registration_time,
            ..
        } in raw_records
        {
            let module_ids = choice_of_modules
                .split(';')
                .map(|module_name| {
                    let module_name = module_name.trim();
                    // Have we seen this module before?
                    let module_id = if let Some(module_id) = module_to_id.get(module_name) {
                        // Reuse previous module ID
                        *module_id
                    } else {
                        // Post-process module name, deduce module ID
                        let module = Module::new(module_name);
                        let module_name = module.name.clone();
                        let module_id = result.modules.len();
                        result.modules.push(module);
                        module_to_id.insert(module_name, module_id);
                        module_id
                    };
                    module_id
                })
                .collect();
            result.persons.push(Person {
                identity,
                choice_of_modules: module_ids,
                registration_time,
            })
        }
        result
    }
}
