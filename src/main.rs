extern crate kstat;

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

fn print_header() {
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

fn print_stats(curr: &ZoneHash, old: &Option<ZoneHash>) {
    let mut keys: Vec<_> = curr.keys().collect();
    keys.sort_by_key(|k| k.abs()); // Is there a better way to just sort without abs

    for key in keys {
        let stat = &curr[key];
        let zonename = &read_string(stat.data.get("zonename").unwrap());
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

        write_output!(zonename, ten_ms, one_hundred_ms, one_second, ten_second);
    }
}

fn main() {
    let mut counter = 0;
    let mut old: Option<ZoneHash> = None;
    let reader =
        KstatReader::new(None, None, None, Some("zone_vfs")).expect("failed to create reader");
    let _interval = Some(1);

    print_header();
    loop {
        let stats = reader.read().expect("failed to read kstats");
        let curr = zone_hashmap(stats);

        if counter > 5 {
            print_header();
            counter = 0;
        }
        print_stats(&curr, &old);
        let _ = ::std::io::stderr().flush();

        // move curr -> old
        old = Some(curr);
        counter += 1;

        // hardcoded sleep for now
        thread::sleep(time::Duration::from_secs(5));
    }
}
