use anyhow::anyhow;
use anyhow::Result;
use chrono::{DateTime, Local, NaiveDateTime, Utc};
use log::{debug, info};
use powerpack::{output, Icon, Item};
use std::env;
use std::error::Error;

use std::iter;
use std::time::Duration;

const ICON_DIR: &str = "/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/";
const CLOCK_ICON: &str = "icon.png";
const CALENDAR_ICON: &str = "/System/Applications/Calendar.app";
const OUTPUT_DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

trait ToAlfredItem {
    fn to_utc_item(&self, description: &str) -> Item<'static>;
    fn to_localtime_item(&self, description: &str) -> Item<'static>;
    fn to_relative_item(&self) -> Item<'static>;
    fn to_timestamp_items(&self, description: &str) -> Vec<Item<'static>>;
}

impl ToAlfredItem for NaiveDateTime {
    fn to_utc_item(&self, description: &str) -> Item<'static> {
        let utc_dt = DateTime::<Utc>::from_utc(*self, Utc);
        debug!("UTC Datetime: {:?}", utc_dt);
        Item::new(utc_dt.format(OUTPUT_DATE_FORMAT).to_string())
            .subtitle(format!("From {}: UTC", description))
            .icon(Icon::with_file_icon(CALENDAR_ICON))
            .arg(utc_dt.timestamp().to_string())
    }

    fn to_localtime_item(&self, description: &str) -> Item<'static> {
        let local_dt: DateTime<Local> = DateTime::from(DateTime::<Utc>::from_utc(*self, Utc));

        debug!(
            "Local datetime: {:?}, offset: {}",
            local_dt,
            local_dt.offset().to_string()
        );

        Item::new(local_dt.format(OUTPUT_DATE_FORMAT).to_string())
            .subtitle(format!(
                "From {}: Local time ({})",
                description,
                local_dt.offset()
            ))
            .icon(Icon::with_file_icon(CALENDAR_ICON))
            .arg(local_dt.to_string())
    }

    fn to_relative_item(&self) -> Item<'static> {
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

    fn to_timestamp_items(&self, description: &str) -> Vec<Item<'static>> {
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
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );

    debug!("Starting timestamp workflow");

    // Alfred passes in a single argument for the user query.
    let arg = env::args().nth(1);
    let query = arg.as_deref().unwrap_or("").trim();

    info!("Query: {:?}", query);

    let current_ts = current_timestamps(Utc::now().naive_utc());

    if query.is_empty() {
        output(current_ts)?;
        return Ok(());
    }

    match parse_datetime(query) {
        Ok(dt) => {
            let is_numeric = query.parse::<i64>().is_ok();
            let mut items = match is_numeric {
                true => vec![
                    dt.to_localtime_item("timestamp"),
                    dt.to_utc_item("timestamp"),
                    dt.to_relative_item(),
                ],
                false => dt.to_timestamp_items("Time since epoch"),
            };
            items.extend(current_ts);
            output(items)?;
            Ok(())
        }
        Err(e) => {
            debug!("Failed to parse '{}', giving up. Final error: {}", query, e);
            output(iter::once(
                Item::new("Error")
                    .subtitle(format!("Failed to parse '{}' to a date", query))
                    .icon(powerpack::Icon::with_image(
                        format!("{}/AlertStopIcon.icns", ICON_DIR).as_str(),
                    )),
            ))?;
            Err(Box::from(e))
        }
    }
}

fn current_timestamps(datetime: NaiveDateTime) -> Vec<Item<'static>> {
    debug!("Creating timestamps for {:?}", datetime);

    let mut items = datetime.to_timestamp_items("Current time");
    items.extend(vec![
        datetime.to_localtime_item("current time"),
        datetime.to_utc_item("current time"),
    ]);
    items
}

fn parse_datetime(s: &str) -> Result<NaiveDateTime> {
    parse_timestamp(s).or(parse_iso8601(s)).or(parse_rfc2822(s))
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
