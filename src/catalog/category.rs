const MAIN_CATEGORIES: [(&str, &str); 11] = [
    ("Development", "Developer"),
    ("Game", "Games"),
    ("Graphics", "Graphics"),
    ("AudioVideo", "Media"),
    ("Audio", "Media"),
    ("Video", "Media"),
    ("Network", "Internet"),
    ("Office", "Productivity"),
    ("Education", "Education"),
    ("Science", "Education"),
    ("Settings", "System"),
];
const FALLBACK: &str = "Utilities";

pub fn folder_name<'a>(categories: impl IntoIterator<Item = &'a [String]>) -> &'static str {
    let mut tally = [0u8; MAIN_CATEGORIES.len()];
    for list in categories {
        let found = MAIN_CATEGORIES
            .iter()
            .position(|(category, _)| list.iter().any(|item| item == category));
        if let Some(index) = found {
            tally[index] = tally[index].saturating_add(1);
        }
    }
    tally
        .iter()
        .enumerate()
        .max_by_key(|&(index, count)| (*count, std::cmp::Reverse(index)))
        .filter(|&(_, count)| *count > 0)
        .map_or(FALLBACK, |(index, _)| MAIN_CATEGORIES[index].1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn shared_category_names_the_folder() {
        let first = list(&["GTK", "Development", "IDE"]);
        let second = list(&["Development"]);
        assert_eq!(
            folder_name([first.as_slice(), second.as_slice()]),
            "Developer"
        );
    }

    #[test]
    fn unknown_categories_fall_back_to_utilities() {
        let only = list(&["X-Custom"]);
        assert_eq!(folder_name([only.as_slice()]), FALLBACK);
    }
}
