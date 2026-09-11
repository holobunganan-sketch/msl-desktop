use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct FieldValue {
    pub value: serde_json::Value,
    pub edited_at_ms: i64,
    pub device_id: String,
    pub parents: BTreeSet<String>,
}

impl FieldValue {
    pub fn text(value: &str, edited_at_ms: i64, device_id: &str) -> Self {
        Self::new(
            serde_json::Value::String(value.into()),
            edited_at_ms,
            device_id,
        )
    }

    pub fn number(value: i64, edited_at_ms: i64, device_id: &str) -> Self {
        Self::new(value.into(), edited_at_ms, device_id)
    }

    pub fn null(edited_at_ms: i64, device_id: &str) -> Self {
        Self::new(serde_json::Value::Null, edited_at_ms, device_id)
    }

    pub fn new(value: serde_json::Value, edited_at_ms: i64, device_id: &str) -> Self {
        Self {
            value,
            edited_at_ms,
            device_id: device_id.into(),
            parents: BTreeSet::new(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MergeRecord {
    pub fields: BTreeMap<String, FieldValue>,
}

impl MergeRecord {
    pub fn fixture<const N: usize>(fields: [(&str, FieldValue); N]) -> Self {
        Self {
            fields: fields
                .into_iter()
                .map(|(name, value)| (name.to_string(), value))
                .collect(),
        }
    }

    pub fn with(mut self, name: &str, value: FieldValue) -> Self {
        self.fields.insert(name.to_string(), value);
        self
    }

    pub fn text(&self, name: &str) -> Option<&str> {
        self.fields.get(name)?.value.as_str()
    }

    pub fn number(&self, name: &str) -> Option<i64> {
        self.fields.get(name)?.value.as_i64()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MergeConflict {
    pub field: String,
    pub local: FieldValue,
    pub remote: FieldValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MergeOutcome {
    pub record: MergeRecord,
    pub conflicts: Vec<MergeConflict>,
}

pub fn merge_record(local: &MergeRecord, remote: &MergeRecord) -> MergeOutcome {
    let mut record = local.clone();
    let mut conflicts = Vec::new();
    let common_stamps: BTreeSet<i64> = local
        .fields
        .iter()
        .filter_map(|(name, value)| {
            remote
                .fields
                .get(name)
                .filter(|other| {
                    other.value == value.value && other.edited_at_ms == value.edited_at_ms
                })
                .map(|_| value.edited_at_ms)
        })
        .collect();
    let local_has_newer_elsewhere = local.fields.iter().any(|(name, value)| {
        remote.fields.get(name).is_some_and(|other| {
            value.value != other.value && value.edited_at_ms > other.edited_at_ms
        })
    });
    let remote_has_newer_elsewhere = remote.fields.iter().any(|(name, value)| {
        local.fields.get(name).is_some_and(|other| {
            value.value != other.value && value.edited_at_ms > other.edited_at_ms
        })
    });
    for (name, incoming) in &remote.fields {
        let Some(current) = record.fields.get(name).cloned() else {
            record.fields.insert(name.clone(), incoming.clone());
            continue;
        };
        if current.value == incoming.value {
            if incoming.edited_at_ms > current.edited_at_ms {
                record.fields.insert(name.clone(), incoming.clone());
            }
            continue;
        }
        if current.device_id == incoming.device_id {
            if incoming.edited_at_ms > current.edited_at_ms {
                record.fields.insert(name.clone(), incoming.clone());
            }
            continue;
        }
        if common_stamps.contains(&current.edited_at_ms)
            && !common_stamps.contains(&incoming.edited_at_ms)
        {
            record.fields.insert(name.clone(), incoming.clone());
            continue;
        }
        if common_stamps.contains(&incoming.edited_at_ms)
            && !common_stamps.contains(&current.edited_at_ms)
        {
            continue;
        }
        if local_has_newer_elsewhere && remote_has_newer_elsewhere {
            if incoming.edited_at_ms > current.edited_at_ms {
                record.fields.insert(name.clone(), incoming.clone());
            }
            continue;
        }
        let distance = (incoming.edited_at_ms - current.edited_at_ms).abs();
        if distance > 120_000 {
            if incoming.edited_at_ms > current.edited_at_ms {
                record.fields.insert(name.clone(), incoming.clone());
            }
        } else {
            conflicts.push(MergeConflict {
                field: name.clone(),
                local: current,
                remote: incoming.clone(),
            });
        }
    }
    MergeOutcome { record, conflicts }
}
