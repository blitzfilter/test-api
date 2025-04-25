use crate::generator::Generator;
use item_core::item_hash::ItemHash;
use item_core::item_model::ItemModel;
use item_core::item_state::ItemState;
use rand::distr::{Alphanumeric, SampleString};
use rand::prelude::IteratorRandom;
use rand::{Rng, random_bool, random_range};
use std::time::Duration;
use strum::IntoEnumIterator;
use time::OffsetDateTime;
use uuid::Uuid;

impl Generator for ItemModel {
    fn generate() -> Self {
        let item_id = Uuid::new_v4().to_string();
        let base_url = format!(
            "https://{}.com",
            Alphanumeric.sample_string(&mut rand::rng(), 10)
        );
        let created = random_iso8601_timestamp();
        let mut item = ItemModel {
            item_id: format!("{base_url}#{item_id}"),
            created: Some(created.clone()),
            source_id: Some(base_url.clone()),
            event_id: Some(format!("{base_url}#{item_id}#{created}")),
            state: rand_opt(0.8, ItemState::iter().choose(&mut rand::rng())).flatten(),
            price: rand_opt(0.8, random_range(5.0..50000.0)),
            category: rand_opt(
                0.8,
                lipsum::lipsum_words(random_range(2..7))
                    .replace(" ", "/")
                    .replace(".", "")
                    .replace(",", ""),
            ),
            name_en: rand_opt(0.8, lipsum::lipsum_title()),
            description_en: rand_opt(0.8, lipsum::lipsum_words(random_range(100..500))),
            name_de: rand_opt(0.8, lipsum::lipsum_title()),
            description_de: rand_opt(0.8, lipsum::lipsum_words(random_range(100..500))),
            url: Some(base_url.clone()),
            image_url: rand_opt(0.8, format!("https://{base_url}/{item_id}.png")),
            hash: None,
        };
        item.hash = Some(item.hash());

        item
    }
}

fn rand_opt<T>(p: f64, t: T) -> Option<T> {
    random_bool(p).then(|| t)
}

fn random_iso8601_timestamp() -> String {
    let now = OffsetDateTime::now_utc();

    // Random offset: ±5 years in seconds
    let max_ms: i64 = 5 * 365 * 24 * 60 * 60 * 1000;
    let offset_secs = rand::rng().random_range(0..=max_ms);

    let random_time = now - Duration::from_millis(offset_secs as u64);
    random_time
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}

#[cfg(test)]
mod tests {
    use crate::generator::Generator;
    use item_core::item_model::ItemModel;

    #[test]
    fn should_generate_random_item() {
        let item = ItemModel::generate();

        assert!(item.created.is_some());
        assert!(item.source_id.is_some());
        assert!(item.event_id.is_some());
        assert!(item.url.is_some());
        assert!(item.hash.is_some());
    }
}
