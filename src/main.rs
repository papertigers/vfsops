extern crate clap;
extern crate kstat;

use clap::{Arg, App};
use kstat::kstat_named::KstatNamedData;
use kstat::{KstatData, KstatReader};
use std::collections::HashMap;
use std::io::Write;
use std::{thread, time};

macro_rules! print_fmt {
    () => ("{:>8} {:>10} {:>10} {:>10} {:>10}");
}

macro_rules! write_output(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), print_fmt!(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);

fn print_header(hide: bool) {
    if hide {
        return;
    }
    write_output!("zone", "10ms_ops", "100ms_ops", "1s_ops", "10s_ops");
}

type VfsData = Vec<KstatData>;
type ZoneHash = HashMap<i32, KstatData>;

struct Stats {
    ten_ms: u64,
    one_hundred_ms: u64,
    one_second: u64,
    ten_second: u64,
}

/// Consume VfsData and return it back as 'instance_id: KstatData'
fn zone_hashmap(data: VfsData) -> ZoneHash {
    data.into_iter().map(|i| (i.instance, i)).collect()
}

fn read_string(data: &KstatNamedData) -> &String {
    match data {
        KstatNamedData::DataString(val) => val,
        _ => panic!("NamedData is not a String"),
    }
}

fn read_u64(data: &KstatNamedData) -> u64 {
    match data {
        KstatNamedData::DataUInt64(val) => *val,
        _ => panic!("NamedData is not a u64"),
    }
}

fn get_stats(data: &HashMap<String, KstatNamedData>) -> Stats {
    let ten_ms = read_u64(data.get("10ms_ops").unwrap());
    let one_hundred_ms = read_u64(data.get("100ms_ops").unwrap());
    let one_second = read_u64(data.get("1s_ops").unwrap());
    let ten_second = read_u64(data.get("10s_ops").unwrap());
    Stats {
        ten_ms,
        one_hundred_ms,
        one_second,
        ten_second,
    }
}

fn print_stats(curr: &ZoneHash, old: &Option<ZoneHash>, zone: &Option<String>, all: &bool) {
    let mut keys: Vec<_> = curr.keys().collect();
    keys.sort_by_key(|k| k.abs()); // Is there a better way to just sort without abs

    for key in keys {
        let stat = &curr[key];
        let zonename = &read_string(stat.data.get("zonename").unwrap());
        if zone.is_some() {
            if zone.as_ref().unwrap() != *zonename {
                continue;
            }
        }
        let len = if zonename.len() >= 8 { 8 } else { zonename.len() };
        let zonename = &read_string(stat.data.get("zonename").unwrap())[0..len];

        let stats = get_stats(&stat.data);
        if old.is_none() {
            write_output!(zonename, stats.ten_ms, stats.one_hundred_ms,
                stats.one_second, stats.ten_second);
            continue;
        }
        // We know its Some
        let old = old.as_ref().unwrap();
        let instance = &stat.instance;

        // If a zone appeared during the middle of our run skip it
        if !old.contains_key(instance) { continue; };
        let old_stats = get_stats(&old[instance].data);

        let ten_ms = stats.ten_ms - old_stats.ten_ms;
        let one_hundred_ms = stats.one_hundred_ms - old_stats.one_hundred_ms;
        let one_second = stats.one_second - old_stats.one_second;
        let ten_second = stats.ten_second - old_stats.ten_second;

        if !all {
            if ten_ms == 0 && one_hundred_ms == 0 && one_second == 0 && ten_second == 0 {
                continue;
            }
        }

        write_output!(zonename, ten_ms, one_hundred_ms, one_second, ten_second);
    }
}

fn main() {
    let about = r#"
The vfsops utility reports vfs operation outliers. A count of the number of operations are grouped
into 10ms, 100ms, 1s, and 10s buckets. The first line of output represents a zone's outliers since
boot, while sequential lines show how many operations have occured during the INTERVAL. By default
vfsops will only output zones that have a non zero value for all buckets.
        "#;
    let matches = App::new("vfsops")
        .version("0.1.0")
        .author("Mike Zeller <mike@mikezeller.net")
        .about("Report VFS op outliers by bucket group")
        .long_about(about)
        .arg(Arg::with_name("H")
            .short("H")
            .help("Don't print the header"))
        .arg(Arg::with_name("z")
            .short("z")
            .long("zone")
            .help("Print data for a specific zonename")
            .value_name("ZONE"))
        .arg(Arg::with_name("Z")
            .short("Z")
            .help("Print zones with no activity"))
        .arg(Arg::with_name("INTERVAL")
            .help("Print results per inverval rather than per second")
            .required(true)
            .index(1))
        .arg(Arg::with_name("COUNT")
            .help("Print for n times and exit")
            .required(false)
            .index(2))
        .get_matches();

    let hide_header = matches.is_present("H");
    let show_all_zones = matches.is_present("Z");
    let zone_filter = match matches.value_of("z") {
        Some(zone) => Some(String::from(zone)),
        None => None,
    };
    let interval = match matches.value_of("INTERVAL") {
        None => 1,
        Some(val) => match val.parse::<i32>() {
            Ok(i) => i,
            Err(_) => {
                println!("please provide a valid INTERVAL");
                ::std::process::exit(1);
            }
        }
    };

    let count = match matches.value_of("COUNT") {
        None => None,
        Some(val) => match val.parse::<i32>() {
            Ok(i) => Some(i),
            Err(_) => {
                println!("please provide a valid COUNT");
                ::std::process::exit(1);
            }
        }
    };

    let mut header_interval = 0;
    let mut nloops = 0;
    let mut old: Option<ZoneHash> = None;
    let reader =
        KstatReader::new(None, None, None, Some("zone_vfs")).expect("failed to create reader");

    print_header(hide_header);
    loop {
        // there must be a better way to do this?
        if count.is_some() {
            if nloops >= *count.as_ref().unwrap() {
                break;
            }
        }
        let stats = reader.read().expect("failed to read kstats");
        let curr = zone_hashmap(stats);

        if header_interval > 5 {
            print_header(hide_header);
            header_interval = 0;
        }
        print_stats(&curr, &old, &zone_filter, & show_all_zones);
        let _ = ::std::io::stderr().flush();

        // move curr -> old
        old = Some(curr);
        header_interval += 1;
        if count.is_some() {
            nloops += 1;
        }

        thread::sleep(time::Duration::from_secs(interval as u64));
    }
}
