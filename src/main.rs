use anyhow::anyhow;
use anyhow::Result;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use log::{debug, info};
use powerpack::{output, Icon, Item};
use std::env;
use std::error::Error;

use crate::Input::Clipboard;
use std::iter;
use std::time::Duration;

const ICON_DIR: &str = "/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/";
const CLOCK_ICON: &str = "icon.png";
const CALENDAR_ICON: &str = "/System/Applications/Calendar.app";
const OUTPUT_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

trait ToAlfredItem {
    fn to_utc_item(&self, description: &str) -> Item;
    fn to_localtime_item(&self, description: &str) -> Item;
    fn to_relative_item(&self) -> Item;
    fn to_timestamp_items(&self, description: &str) -> Vec<Item>;
    fn to_output(&self, source: Input) -> Vec<Item>;
}

#[derive(Debug)]
enum Input {
    Clipboard(String),
    Argument(String),
    None,
}

impl ToAlfredItem for NaiveDateTime {
    fn to_utc_item(&self, description: &str) -> Item {
        let utc_dt = DateTime::<Utc>::from_utc(*self, Utc);
        debug!("UTC Datetime: {:?}", utc_dt);

        let formatted_date = utc_dt.format(OUTPUT_DATE_FORMAT);

        Item::new(formatted_date.to_string())
            .subtitle(format!("From {}: UTC", description))
            .icon(Icon::with_file_icon(CALENDAR_ICON))
            .arg(formatted_date.to_string())
    }

    fn to_localtime_item(&self, description: &str) -> Item {
        let local_dt: DateTime<Local> = DateTime::from(DateTime::<Utc>::from_utc(*self, Utc));

        debug!(
            "Local datetime: {:?}, offset: {}",
            local_dt,
            local_dt.offset().to_string()
        );

        let formatted_date = local_dt.format(OUTPUT_DATE_FORMAT);

        Item::new(formatted_date.to_string())
            .subtitle(format!(
                "From {}: Local time ({})",
                description,
                local_dt.offset()
            ))
            .icon(Icon::with_file_icon(CALENDAR_ICON))
            .arg(formatted_date.to_string())
    }

    fn to_relative_item(&self) -> Item {
        let utc_dt = DateTime::<Utc>::from_utc(*self, Utc);
        debug!("UTC: {}", utc_dt.to_rfc3339());
        let dur = utc_dt.signed_duration_since(Utc::now());

        let nanos = dur.num_nanoseconds().unwrap();
        debug!("Duration relative to current time: {}", dur);

        let dur = match dur.to_std() {
            Ok(d) => d,
            Err(_) => Duration::from_nanos((nanos * -1) as u64),
        };
        let ht = humantime::format_duration(dur);
        debug!("Human-readable duration: {}", ht);

        Item::new(ht.to_string())
            .subtitle("Relative time")
            .icon(Icon::with_image(CLOCK_ICON))
            .arg(ht.to_string())
    }

    fn to_timestamp_items(&self, description: &str) -> Vec<Item> {
        let ts_nanos = self.timestamp_nanos();
        debug!("ns: {}", ts_nanos);
        let ts_micros = self.timestamp_nanos() / 1000;
        debug!("µs: {}", ts_micros);
        let ts_millis = self.timestamp_millis();
        debug!("ms: {}", ts_millis);
        let ts_seconds = self.timestamp();
        debug!("s: {}", ts_seconds);

        vec![
            Item::new(ts_seconds.to_string())
                .subtitle(format!("{} in seconds (s)", description))
                .icon(Icon::with_image(CLOCK_ICON))
                .arg(ts_seconds.to_string()),
            Item::new(ts_millis.to_string())
                .subtitle(format!("{} in milliseconds (ms)", description))
                .icon(Icon::with_image(CLOCK_ICON))
                .arg(ts_millis.to_string()),
            Item::new(ts_micros.to_string())
                .subtitle(format!("{} in microseconds (µs)", description))
                .icon(Icon::with_image(CLOCK_ICON))
                .arg(ts_micros.to_string()),
            Item::new(ts_nanos.to_string())
                .subtitle(format!("{} in nanoseconds (ns)", description))
                .icon(Icon::with_image(CLOCK_ICON))
                .arg(ts_nanos.to_string()),
        ]
    }

    fn to_output(&self, source: Input) -> Vec<Item> {
        debug!("Creating outputs for input source: {:?}", source);
        match source {
            Clipboard(query) => {
                let mut items = match query.parse::<i64>() {
                    Ok(_) => vec![
                        self.to_localtime_item("timestamp from clipboard"),
                        self.to_utc_item("timestamp from clipboard"),
                        self.to_relative_item(),
                    ],
                    Err(_) => self.to_timestamp_items("Time since epoch"),
                };
                items.extend(Utc::now().naive_utc().to_output(Input::None));
                items
            }
            Input::Argument(query) => {
                let mut items = match query.parse::<i64>() {
                    Ok(_) => vec![
                        self.to_localtime_item("timestamp"),
                        self.to_utc_item("timestamp"),
                        self.to_relative_item(),
                    ],
                    Err(_) => self.to_timestamp_items("Time since epoch"),
                };
                items.extend(Utc::now().naive_utc().to_output(Input::None));
                items
            }
            Input::None => {
                let mut items = self.to_timestamp_items("Current time");
                items.extend(vec![
                    self.to_localtime_item("Current time"),
                    self.to_utc_item("Current time"),
                ]);
                items
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );

    debug!("Starting timestamp workflow");

    // Alfred passes in a single argument for the user query.
    let arg = env::args().nth(1);
    let query = arg.as_deref().unwrap_or("").trim().to_string();

    info!("Argument query: {:?}", query);
    let clipboard_content = match cli_clipboard::get_contents() {
        Ok(content) => Some(content),
        Err(e) => {
            debug!("Did not get clipboard contents or non-string: {}", e);
            None
        }
    };

    output(run_workflow(query, clipboard_content)?)?;
    Ok(())
}

fn run_workflow(
    query: String,
    clipboard_content: Option<String>,
) -> Result<Vec<Item>, Box<dyn Error>> {
    let mut items = vec![];

    if query.is_empty() {
        if let Some(content) = clipboard_content {
            match parse_datetime(content.as_str()) {
                Ok(dt) => items.extend(dt.to_output(Clipboard(content))),
                Err(e) => {
                    debug!("Couldn't parse clipboard to date: {}", e)
                }
            };
        }
    }

    if !query.is_empty() {
        match parse_datetime(&query) {
            Ok(dt) => {
                items.extend(dt.to_output(Input::Argument(query)));
            }
            Err(e) => {
                debug!(
                    "Failed to parse input '{}', giving up. Final error: {}",
                    query, e
                );
                output(iter::once(
                    Item::new("Error")
                        .subtitle(format!("Failed to parse '{}' to a date", query))
                        .icon(Icon::with_image(
                            format!("{}/AlertStopIcon.icns", ICON_DIR).as_str(),
                        )),
                ))?;
                return Err(Box::from(e));
            }
        };
    }

    if items.is_empty() {
        items.extend(Utc::now().naive_utc().to_output(Input::None));
    }

    Ok(items)
}

fn parse_datetime(s: &str) -> Result<NaiveDateTime> {
    if s.is_empty() {
        return Err(anyhow!("Empty string"));
    }
    parse_timestamp(s)
        .or(parse_iso8601(s))
        .or(parse_rfc2822(s))
        .or(parse_date_and_time(s))
        .or(parse_date(s))
        .or(parse_time(s))
}

fn parse_timestamp(s: &str) -> Result<NaiveDateTime> {
    debug!("Attempting to parse timestamp: {}", s);

    if s.len() > 22 {
        return Err(anyhow!(
            "String too long ({} characters). Timestamps have at most 20 digits.",
            s.len()
        ));
    }

    let ts = s.parse::<i64>()?;

    let mut seconds = ts;
    let mut exp = 0;

    while seconds > u32::MAX as i64 {
        seconds /= 1000;
        exp += 3;
    }

    let nanos = (ts % 10_i64.pow(exp)) * (10_i64.pow(9 - exp));

    debug!(
        "Creating datetime with seconds: {}, nanos: {}",
        seconds, nanos
    );

    let naive_dt = NaiveDateTime::from_timestamp_opt(seconds, nanos as u32);
    match naive_dt {
        None => Err(anyhow!("Not a timestamp: {}", s)),
        Some(dt) => Ok(dt),
    }
}

fn parse_iso8601(s: &str) -> Result<NaiveDateTime> {
    debug!("Attempting to parse ISO8601 format");
    Ok(s.parse::<DateTime<Utc>>()?.naive_utc())
}

fn parse_rfc2822(s: &str) -> Result<NaiveDateTime> {
    debug!("Attempting to parse RFC 2822 format");
    Ok(DateTime::parse_from_rfc2822(s)?.naive_utc())
}

fn parse_date_and_time(s: &str) -> Result<NaiveDateTime> {
    debug!("Attempting to parse date and time");
    let naive = NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")?;
    debug!("Parsed naive DateTime: {:?}", naive);
    let local = Local.from_local_datetime(&naive).unwrap();
    debug!("Converted to local DateTime: {:?}", local);
    Ok(local.naive_utc())
}

fn parse_date(s: &str) -> Result<NaiveDateTime> {
    debug!("Attempting to parse date");
    let naive = NaiveDate::parse_from_str(s, "%Y-%m-%d")?;
    debug!("Parsed naive Date: {:?}", naive);
    let local = Local.from_local_date(&naive).unwrap();
    debug!("Converted to local Date: {:?}", local);
    Ok(local.naive_utc().and_hms(0, 0, 0))
}

fn parse_time(s: &str) -> Result<NaiveDateTime> {
    debug!("Attempting to parse time");
    let naive = NaiveTime::parse_from_str(s, "%H:%M:%S")?;
    debug!("Parsed naive time: {:?}", naive);
    let local_date = Local::today();
    debug!("Local Date: {:?}", local_date);
    let local_datetime = local_date.and_time(naive).unwrap();
    debug!("Local DateTime: {:?}", local_datetime);
    Ok(local_datetime.naive_utc())
}

#[cfg(test)]
mod tests {
    use crate::run_workflow;
    use powerpack::Item;
    use pretty_assertions::assert_eq;
    use rstest::rstest;
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    struct TestItem {
        arg: String,
        title: String,
    }

    #[rstest]
    #[case("1662796800", "2022-09-10 08:00:00")]
    #[case("1662796800000", "2022-09-10 08:00:00")]
    #[case("1662796800000000", "2022-09-10 08:00:00")]
    fn it_parses_valid_timestamps(#[case] input: &str, #[case] expected_date_str: &str) {
        let items = run_workflow(input.to_string(), None).unwrap();
        assert_item_matches(&items[1], expected_date_str)
    }

    #[rstest]
    #[case("2022-09-10T10:00:00Z", 1662804000)]
    #[case("2022-09-10T10:00:00 +00:00", 1662804000)]
    #[case("2022-09-10T10:00:00 +02:00", 1662796800)]
    #[case("Sat, 10 Sep 2022 10:00:00 +0200", 1662796800)]
    #[case("2022-09-10 10:00:00", 1662796800)]
    #[case("2022-09-10", 1662768000)]
    fn it_parses_valid_strings(#[case] input: &str, #[case] expected_timestamp: i32) {
        let items = run_workflow(input.to_string(), None).unwrap();
        assert_item_matches(&items[0], &expected_timestamp.to_string())
    }

    #[rstest]
    fn it_ignores_clipboard_if_input_present() {
        let items = run_workflow(
            "2022-09-10T10:00:00Z".to_string(),
            Some("1970-01-01T00:00:00Z".to_string()),
        )
        .unwrap();
        assert_item_matches(&items[0], "1662804000")
    }

    #[rstest]
    fn it_uses_clipboard_if_no_input_present() {
        let items = run_workflow("".to_string(), Some("1970-01-01T00:00:00Z".to_string())).unwrap();
        assert_item_matches(&items[0], "0")
    }

    fn assert_item_matches(item: &Item, expected: &str) {
        let serialised = serde_json::to_string(item).unwrap();
        let deserialised: TestItem = serde_json::from_str(&serialised).unwrap();

        assert_eq!(deserialised.title, expected);
        assert_eq!(deserialised.arg, deserialised.title);
    }
}
