use sit_core::{Item, Record, Repository};
use rayon::prelude::*;

pub fn command<MI: Send + Sync>(repo: Repository<MI>) -> i32 {
    let items: Vec<_> = repo.item_iter().expect("can't list items").collect();
    let valid = items.into_par_iter()
        .map(|mut item| {
            item.set_integrity_check(false);
            let all_records: Vec<_> = item.record_iter().expect("can't list records").flat_map(|v| v).collect();
            item.set_integrity_check(true);
            let checked_records: Vec<_> = item.record_iter().expect("can't list records").flat_map(|v| v).collect();
            let invalid_records: Vec<_> = all_records.into_iter()
                .filter(|r| !checked_records.iter().any(|r_| r_.hash() == r.hash()))
                .collect();
            let valid = invalid_records.is_empty();
            for record in invalid_records {
                println!("{} {}", item.id(), record.encoded_hash());
            }
            valid
        })
        .reduce(|| true, |a, b| a && b);
    if valid {
        0
    } else {
        1
    }
}