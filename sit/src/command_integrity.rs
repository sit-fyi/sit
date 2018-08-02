use sit_core::{Record, Repository, record::RecordContainer, hash::HashingAlgorithm};
#[cfg(feature = "deprecated-items")]
use rayon::prelude::*;

pub fn command<MI: Send + Sync>(mut repo: Repository<MI>) -> i32 {
    repo.set_integrity_check(false);
    let hashing_algorithm = repo.config().hashing_algorithm().clone();
    #[cfg(not(feature = "deprecated-items"))]
    let invalid_records_in_items: Vec<String> = vec![];
    #[cfg(feature = "deprecated-items")]
    let (valid_items, invalid_records_in_items): (_, Vec<_>) = {
        let items: Vec<_> = repo.item_iter().expect("can't list items").collect();
        let results: Vec<_> = items.into_par_iter()
            .flat_map(|item| {
                 invalid_records(item, &hashing_algorithm).expect("can't list records")
            }).collect();
        let valid = results.is_empty();
        for record in results.iter() {
            println!("{} {}", record.item_id(), record.encoded_hash());
        }
        (valid, results.iter().map(|record| record.encoded_hash()).collect())
    };
    #[cfg(not(feature = "deprecated-items"))]
    let valid_items = true;
    let invalid_records = invalid_records(repo, &hashing_algorithm).expect("can't list records");
    let valid = invalid_records.is_empty();
    for record in invalid_records.iter().filter(|r| !invalid_records_in_items.iter().any(|r_| r_ == &r.encoded_hash())) {
        println!("{}", record.encoded_hash());
    }
    if valid_items && valid {
        0
    } else {
        1
    }
}

fn invalid_records<RC: RecordContainer>(container: RC, hashing_algorithm: &HashingAlgorithm) -> Result<Vec<RC::Record>, RC::Error> {
    let all_records = container.record_iter()?;
    for record in container.record_iter()?.flat_map(|v| v) {
        eprintln!("{} {:?}", record.encoded_hash().as_ref(), record.integrity_intact(hashing_algorithm));
    }
    Ok(all_records.flat_map(|v| v)
        .filter(|r| !r.integrity_intact(hashing_algorithm))
        .collect())
}

