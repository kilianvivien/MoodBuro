//! pdfplumber's `utils/clustering.py`.

/// Group `items` by a key, putting keys within `tolerance` of their neighbour
/// in the same group. Groups come out in ascending key order; inside a group
/// the items keep the order they arrived in.
pub fn cluster_objects<T: Clone>(items: &[T], key: impl Fn(&T) -> f64, tolerance: f64) -> Vec<Vec<T>> {
    // Adding zero folds -0.0 into 0.0, which Python's set and dict treat as one key.
    let key = |item: &T| key(item) + 0.0;
    let mut values: Vec<f64> = items.iter().map(&key).collect();
    values.sort_by(f64::total_cmp);
    values.dedup();

    // Each distinct value's group number.
    let mut group_of: Vec<usize> = Vec::with_capacity(values.len());
    let mut group = 0;
    for (index, value) in values.iter().enumerate() {
        if index > 0 && (tolerance == 0.0 || *value > values[index - 1] + tolerance) {
            group += 1;
        }
        group_of.push(group);
    }

    let mut groups: Vec<Vec<T>> = vec![Vec::new(); if values.is_empty() { 0 } else { group + 1 }];
    for item in items {
        let value = key(item);
        let position = values.partition_point(|candidate| candidate.total_cmp(&value).is_lt());
        groups[group_of[position]].push(item.clone());
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::cluster_objects;

    #[test]
    fn neighbours_chain_into_one_group() {
        // 1, 3 and 5 are each within 2 of the next, so they chain; 9 stands alone.
        let groups = cluster_objects(&[5.0, 1.0, 9.0, 3.0], |value| *value, 2.0);
        assert_eq!(groups, vec![vec![5.0, 1.0, 3.0], vec![9.0]]);
    }

    #[test]
    fn items_keep_their_order_inside_a_group() {
        let groups = cluster_objects(&[("b", 1.0), ("a", 1.0), ("c", 7.0)], |item| item.1, 3.0);
        assert_eq!(groups, vec![vec![("b", 1.0), ("a", 1.0)], vec![("c", 7.0)]]);
    }

    #[test]
    fn nothing_in_nothing_out() {
        assert!(cluster_objects(&[] as &[f64], |value| *value, 3.0).is_empty());
    }
}
