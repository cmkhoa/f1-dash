use anyhow::Error;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct StateService {
    state: Arc<RwLock<Value>>,
}

impl StateService {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(Value::Object(serde_json::Map::new()))),
        }
    }

    pub async fn get_state(&self) -> Result<Value, Error> {
        let state = self.state.read().await;
        Ok(state.clone())
    }

    pub async fn get_state_string(&self) -> Result<String, Error> {
        let state = self.state.read().await;
        Ok(state.to_string())
    }

    pub async fn set_state(&self, new_state: Value) -> Result<(), Error> {
        let mut state = self.state.write().await;
        *state = new_state;
        Ok(())
    }

    pub async fn update_state(&self, update: Value) -> Result<(), Error> {
        let mut state = self.state.write().await;
        merge(&mut state, update);
        Ok(())
    }
}

pub fn merge(base: &mut Value, update: Value) {
    match (base, update) {
        (Value::Object(prev), Value::Object(update)) => {
            for (k, v) in update {
                merge(prev.entry(k).or_insert(Value::Null), v);
            }
        }
        (Value::Array(prev), Value::Object(update)) => {
            for (k, v) in update {
                if let Ok(index) = k.parse::<usize>() {
                    if let Some(item) = prev.get_mut(index) {
                        merge(item, v);
                    } else {
                        prev.push(v);
                    }
                }
            }
        }
        (a, b) => *a = b,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{StateService, merge};

    #[test]
    fn merge_keeps_existing_object_fields() {
        let mut state = json!({
            "TimingData": {
                "Lines": {
                    "1": { "Position": "1", "LastLapTime": "1:30.000" }
                }
            }
        });

        merge(
            &mut state,
            json!({ "TimingData": { "Lines": { "1": { "Position": "2" } } } }),
        );

        assert_eq!(
            state,
            json!({
                "TimingData": {
                    "Lines": {
                        "1": { "Position": "2", "LastLapTime": "1:30.000" }
                    }
                }
            })
        );
    }

    #[test]
    fn merge_updates_an_array_item_by_its_sparse_index() {
        let mut state = json!({
            "RaceControlMessages": [
                { "Message": "Session started" },
                { "Message": "DRS enabled" }
            ]
        });

        merge(
            &mut state,
            json!({ "RaceControlMessages": { "1": { "Message": "Safety car" } } }),
        );

        assert_eq!(
            state["RaceControlMessages"][0]["Message"],
            "Session started"
        );
        assert_eq!(state["RaceControlMessages"][1]["Message"], "Safety car");
    }

    #[tokio::test]
    async fn state_service_returns_an_independent_snapshot() {
        let service = StateService::new();
        service
            .set_state(json!({ "TrackStatus": "1" }))
            .await
            .expect("state assignment succeeds");

        let mut snapshot = service.get_state().await.expect("state is readable");
        snapshot["TrackStatus"] = json!("5");

        assert_eq!(
            service.get_state().await.expect("state is readable")["TrackStatus"],
            "1"
        );
    }
}
