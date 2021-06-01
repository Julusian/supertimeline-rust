extern crate serde_path_to_error;
extern crate supertimeline;
extern crate supertimeline_json;

use std::env;
use std::fs;
use std::time::Instant;
use supertimeline::get_state;
use supertimeline::resolve_all_states;
use supertimeline::resolve_timeline;
use supertimeline::ResolveOptions;

use supertimeline_json::object::JsonTimelineObject;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: dump.json 10")
    } else {
        let filename = &args[1];
        let iterations = args[2].parse::<usize>().unwrap();

        println!("Running {} iterations of {}", iterations, filename);

        let raw_tl = fs::read_to_string(filename).unwrap();
        let processed_tl = supertimeline_json::hack::mangle_json_enable(&raw_tl).unwrap();

        // let mut deserializer = serde_json::Deserializer::from_str(&processed_tl);
        // let parsed: Vec<JsonTimelineObject> =
        //     serde_path_to_error::deserialize(&mut deserializer).unwrap();

        {
            let mut times = Vec::new();
            let mut a = 0;

            println!("Starting");
            for i in 0..iterations {
                let options = ResolveOptions {
                    time: 1597158621470 + (i as u64 * 1000),
                    limit_count: None,
                    limit_time: None,
                };

                let mut deserializer = serde_json::Deserializer::from_str(&processed_tl);
                let parsed: Vec<JsonTimelineObject> =
                    serde_path_to_error::deserialize(&mut deserializer).unwrap();

                let start = Instant::now();

                let resolved = resolve_timeline(&parsed, options).unwrap();
                // let states = resolve_all_states(&resolved, None).unwrap();

                // let state = get_state(&states, 1597158621470 + 5000, None);

                let duration = start.elapsed();
                times.push(duration.as_nanos());

                a += resolved.objects.len();
            }

            let sum: u128 = times.iter().sum();
            let avg = ((sum as f64) / (times.len() as f64)) / 1000000.0;
            println!(
                "Completed {} resolve iterations in {}ms, averaging {}ms (Ignore: {} objs)",
                iterations, sum, avg, a
            );
        }

        {
            let mut times = Vec::new();
            let mut a = 0;

            println!("Starting");
            for i in 0..iterations {
                let options = ResolveOptions {
                    time: 1597158621470 + (i as u64 * 1000),
                    limit_count: None,
                    limit_time: None,
                };

                let mut deserializer = serde_json::Deserializer::from_str(&processed_tl);
                let parsed: Vec<JsonTimelineObject> =
                    serde_path_to_error::deserialize(&mut deserializer).unwrap();
                let resolved = resolve_timeline(&parsed, options).unwrap();

                let start = Instant::now();

                let states = resolve_all_states(&resolved, None).unwrap();

                // let state = get_state(&states, 1597158621470 + 5000, None);

                let duration = start.elapsed();
                times.push(duration.as_nanos());

                a += states.next_events.len();
            }

            let sum: u128 = times.iter().sum();
            let avg = ((sum as f64) / (times.len() as f64)) / 1000000.0;
            println!(
                "Completed {} allStates iterations in {}ms, averaging {}ms (Ignore: {} events)",
                iterations, sum, avg, a
            );
        }

        {
            let mut times = Vec::new();
            let mut a = 0;

            println!("Starting");
            for i in 0..iterations {
                let options = ResolveOptions {
                    time: 1597158621470 + (i as u64 * 1000),
                    limit_count: None,
                    limit_time: None,
                };

                let mut deserializer = serde_json::Deserializer::from_str(&processed_tl);
                let parsed: Vec<JsonTimelineObject> =
                    serde_path_to_error::deserialize(&mut deserializer).unwrap();
                let resolved = resolve_timeline(&parsed, options).unwrap();
                let states = resolve_all_states(&resolved, None).unwrap();

                let start = Instant::now();

                let time = &states.next_events[i % &states.next_events.len()];
                let state = get_state(&states, time.time, None);

                let duration = start.elapsed();
                times.push(duration.as_nanos());

                a += state.next_events.len();
            }

            let sum: u128 = times.iter().sum();
            let avg = ((sum as f64) / (times.len() as f64)) / 1000000.0;
            println!(
                "Completed {} getState iterations in {}ms, averaging {}ms (Ignore: {} events)",
                iterations, sum, avg, a
            );
        }
    }
}
