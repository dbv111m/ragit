use chrono::DateTime;
use chrono::offset::Local;
use crate::{Error, Message};
use crate::json_type::JsonType;
use json::JsonValue;
use ragit_fs::{
    WriteMode,
    read_string,
    write_string,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct RecordAt {
    pub path: String,
    pub id: String,
}

// using the same type for integers makes ser/de easier
#[derive(Clone, Copy)]
pub struct Record {
    pub time: u64,
    pub input: u64,
    pub output: u64,

    // dollars per 1 billion tokens
    pub input_weight: u64,
    pub output_weight: u64,
}

impl From<Record> for JsonValue {
    fn from(r: Record) -> JsonValue {
        JsonValue::Array(vec![
            JsonValue::from(r.time),
            JsonValue::from(r.input),
            JsonValue::from(r.output),
            JsonValue::from(r.input_weight),
            JsonValue::from(r.output_weight),
        ])
    }
}

impl TryFrom<JsonValue> for Record {
    type Error = Error;

    fn try_from(j: JsonValue) -> Result<Record, Error> {
        let mut result = vec![];

        for member in j.members() {
            match member.as_u64() {
                Some(n) if result.len() < 5 => {
                    result.push(n);
                },
                Some(_) => {
                    return Err(Error::WrongSchema(String::from("expected an array of length 5, but got more than 5")));
                },
                None => {
                    return Err(Error::JsonTypeError {
                        expected: JsonType::U64,
                        got: member.into(),
                    });
                },
            }
        }

        if result.len() != 5 {
            Err(Error::WrongSchema(format!("expected an array of length 5, but got length {}", result.len())))
        }

        else {
            Ok(Record {
                time: result[0],
                input: result[1],
                output: result[2],
                input_weight: result[3],
                output_weight: result[4],
            })
        }
    }
}

// why do I have to impl it manually?
fn records_from_json(j: JsonValue) -> Result<Vec<Record>, Error> {
    if !j.is_array() {
        return Err(Error::JsonTypeError {
            expected: JsonType::Array,
            got: (&j).into(),
        });
    }

    let mut result = vec![];

    for member in j.members() {
        result.push(Record::try_from(member.clone())?);
    }

    Ok(result)
}

#[derive(Clone)]
pub struct Tracker(pub HashMap<String, Vec<Record>>);  // user_name -> usage

impl From<Tracker> for JsonValue {
    fn from(t: Tracker) -> JsonValue {
        JsonValue::Object(t.0.into_iter().collect())
    }
}

impl TryFrom<JsonValue> for Tracker {
    type Error = Error;

    fn try_from(j: JsonValue) -> Result<Tracker, Error> {
        let mut result = HashMap::new();

        for (k, v) in j.entries() {
            result.insert(
                k.to_string(),
                records_from_json(v.clone())?,
            );
        }

        Ok(Tracker(result))
    }
}

impl Tracker {
    pub fn new() -> Self {
        Tracker(HashMap::new())
    }

    pub fn load_from_file(path: &str) -> Result<Self, Error> {
        let content = read_string(path)?;
        let j = json::parse(&content)?;
        Self::try_from(j)
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Error> {
        let result = JsonValue::from(self.clone());

        Ok(write_string(
            path,
            &result.pretty(4),
            WriteMode::CreateOrTruncate,
        )?)
    }
}

pub fn record_api_usage(
    at: &RecordAt,
    input_count: u64,
    output_count: u64,

    // dollars per 1 billion tokens
    input_weight: u64,
    output_weight: u64,
    clean_up_records: bool,
) -> Result<(), String> {
    let mut tracker = Tracker::load_from_file(&at.path).map_err(|e| format!("{e:?}"))?;
    let new_record = Record {
        time: Local::now().timestamp().max(0) as u64,
        input: input_count,
        output: output_count,
        input_weight,
        output_weight,
    };

    match tracker.0.get_mut(&at.id) {
        Some(mut records) => {
            records.push(new_record);

            if clean_up_records {
                // `records` is always sorted
                let mut new_records = vec![];
                let old = Local::now().timestamp().max(1 << 41) as u64 - (1 << 41);

                for record in records.iter() {
                    if record.time < old {
                        continue;
                    }

                    match new_records.last_mut() {
                        Some(Record {
                            time,
                            input,
                            output,
                            input_weight,
                            output_weight,
                        }) if *time + (1 << 27) > record.time && *input_weight == record.input_weight && *output_weight == record.output_weight => {
                            *time = (*time + record.time) >> 1;
                            *input += record.input;
                            *output += record.output;
                        },
                        _ => {
                            new_records.push(*record);
                        },
                    }
                }

                new_records.sort_by_key(|Record { time, .. }| *time);
                *records = new_records;
            }
        },
        None => {
            tracker.0.insert(at.id.clone(), vec![new_record]);
        },
    }

    tracker.save_to_file(&at.path).map_err(|e| format!("{e:?}"))?;

    Ok(())
}

pub fn get_user_usage_data_after(at: RecordAt, after: DateTime<Local>) -> Option<Vec<Record>> {
    let after = after.timestamp().max(0) as u64;

    match Tracker::load_from_file(&at.path) {
        Ok(tracker) => match tracker.0.get(&at.id) {
            Some(records) => Some(records.iter().filter(
                |Record { time, .. }| *time > after
            ).map(
                |record| record.clone()
            ).collect()),
            None => None,
        },
        _ => None,
    }
}

pub fn get_usage_data_after(path: &str, after: DateTime<Local>) -> Option<Vec<Record>> {
    let after = after.timestamp().max(0) as u64;

    match Tracker::load_from_file(path) {
        Ok(tracker) => {
            let mut result = vec![];

            for records in tracker.0.values() {
                for record in records.iter() {
                    if record.time > after {
                        result.push(record.clone());
                    }
                }
            }

            Some(result)
        },
        _ => None,
    }
}

/// It returns the cost in dollars (in a formatted string), without any currency unit.
pub fn calc_usage(records: &[Record]) -> String {
    // cost * 1B
    let mut total: u64 = records.iter().map(
        |Record {
            time: _,
            input, input_weight,
            output, output_weight,
        }| *input * *input_weight + *output * *output_weight
    ).sum();

    // cost * 1K
    total /= 1_000_000;

    format!("{:.3}", total as f64 / 1_000.0)
}

pub fn dump_pdl(
    messages: &[Message],
    response: &str,
    path: &str,
    metadata: String,
) -> Result<(), Error> {
    let mut markdown = vec![];

    for message in messages.iter() {
        markdown.push(format!(
            "\n\n<|{:?}|>\n\n{}",
            message.role,
            message.content.iter().map(|c| c.to_string()).collect::<Vec<String>>().join(""),
        ));
    }

    markdown.push(format!("\n\n<|Assistant|>\n\n{response}"));
    markdown.push(format!("{}# {metadata} #{}", '{', '}'));  // tera format

    write_string(
        path,
        &markdown.join("\n"),
        WriteMode::CreateOrTruncate,
    )?;

    Ok(())
}
