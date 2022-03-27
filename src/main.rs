use chrono::{DateTime, Local, NaiveDateTime, Utc};
use log::{debug, info};
use powerpack::{output, Icon, Item};
use std::env;
use std::error::Error;
use std::iter;

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "debug"),
    );

    debug!("Starting timestamp workflow");

    // Alfred passes in a single argument for the user query.
    let arg = env::args().nth(1);
    let query = arg.as_deref().unwrap_or("");

    info!("Query: {:?}", query);

    if query.is_empty() {
        output(current_timestamps(Utc::now()))?;
        return Ok(());
    }

    let maybe_timestamp = query.parse::<i64>();
    if let Ok(timestamp) = maybe_timestamp {
        output(parse_timestamp(timestamp))?;
        Ok(())
    } else {
        match parse_date_string(query) {
            Ok(items) => {
                output(items)?;
                Ok(())
            }
            Err(e) => {
                output(iter::once(
                    Item::new("Error")
                        .subtitle(format!("Failed to parse '{:?}': {}", query, e.as_str()))
                        .icon(powerpack::Icon::with_type("public.script")),
                ))?;
                Ok(())
            }
        }
    }
}

fn current_timestamps(datetime: DateTime<Utc>) -> Vec<Item<'static>> {
    debug!("Creating timestamps for {:?}", datetime);
    let ts_nanos = datetime.timestamp_nanos().to_string();
    debug!("ns: {}", ts_nanos);
    let ts_micros = (datetime.timestamp_nanos() / 1000).to_string();
    debug!("µs: {}", ts_micros);
    let ts_millis = datetime.timestamp_millis().to_string();
    debug!("ms: {}", ts_millis);
    let ts_seconds = datetime.timestamp().to_string();
    debug!("s: {}", ts_seconds);

    vec![
        Item::new(ts_seconds.clone())
            .subtitle("Current timestamp in seconds")
            .icon(Icon::with_image("icon.png"))
            .arg(ts_seconds),
        Item::new(ts_millis.clone())
            .subtitle("Current timestamp in milliseconds")
            .icon(Icon::with_image("icon.png"))
            .arg(ts_millis),
        Item::new(ts_micros.clone())
            .subtitle("Current timestamp in microseconds")
            .icon(Icon::with_image("icon.png"))
            .arg(ts_micros),
        Item::new(ts_nanos.clone())
            .subtitle("Current timestamp in nanoseconds")
            .icon(Icon::with_image("icon.png"))
            .arg(ts_nanos),
    ]
}

fn parse_timestamp(ts: i64) -> Vec<Item<'static>> {
    debug!("Attempting to parse timestamp: {}", ts);

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

    let utc_dt =
        DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(seconds, nanos as u32), Utc);
    debug!("UTC Datetime: {:?}", utc_dt);

    let local_dt: DateTime<Local> = DateTime::from(utc_dt);
    debug!(
        "Local datetime: {:?}, offset: {}",
        local_dt,
        local_dt.offset().to_string()
    );

    vec![
        Item::new(utc_dt.to_rfc3339())
            .subtitle("UTC")
            .icon(Icon::with_image("icon.png"))
            .arg(utc_dt.to_string()),
        Item::new(local_dt.to_rfc3339())
            .subtitle(format!("Local time ({})", local_dt.offset().to_string()))
            .icon(Icon::with_image("icon.png"))
            .arg(local_dt.to_string()),
    ]
}

fn parse_date_string(_s: &str) -> Result<Vec<Item<'static>>, String> {
    Err("Not implemented".to_string())
}
