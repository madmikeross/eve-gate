use std::sync::Arc;

use futures::StreamExt;
use neo4rs::{Graph, query};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct System {
    pub constellation_id: Option<i64>,
    pub name: Option<String>,
    pub planets: Option<Vec<i64>>,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub security_class: Option<String>,
    pub security_status: f64,
    pub star_id: Option<i64>,
    pub stargates: Option<Vec<i64>>,
    pub system_id: i64,
}

pub(crate) async fn get_graph_client() -> Arc<Graph> {
    let uri = "bolt://localhost:7687";
    let user = "neo4j";
    let pass = "neo4jneo4j"; // assumes you have accessed via the browser and updated pass
    Arc::new(Graph::new(uri, user, pass).await.unwrap())
}

pub(crate) async fn system_id_exists(graph: &Graph, system_id: i64) -> Result<bool, neo4rs::Error> {
    let system_exists = "MATCH (s:System {system_id: $system_id}) RETURN COUNT(s) as count LIMIT 1";
    let mut result = graph.execute(query(system_exists).param("system_id", system_id)).await?;

    match result.next().await? {
        Some(row) => Ok(row.get::<i64>("count").map_or(false, |count| count > 0)),
        None => Ok(false)
    }
}

pub(crate) async fn save_system(graph: &Arc<Graph>, system: &System) -> Result<(), neo4rs::Error> {
    let create_statement = "
        CREATE (s:System {
            system_id: $system_id,
            name: $name,
            constellation_id: $constellation_id,
            security_status: $security_status,
            star_id: $star_id,
            security_class: $security_class,
            x: $x,
            y: $y,
            z: $z,
            planets: $planets,
            stargates: $stargates
        })";

    let constellation_id = serde_json::to_string(&system.constellation_id).unwrap();
    let planets_json = serde_json::to_string(&system.planets).unwrap();
    let security_class_param = system.security_class.as_ref().map(|s| s.as_str()).unwrap_or("");
    let name_param = system.name.as_ref().map(|s| s.as_str()).unwrap_or("");
    let stargates = serde_json::to_string(&system.stargates).unwrap();
    let star_id = serde_json::to_string(&system.star_id).unwrap();

    graph.run(query(&create_statement)
        .param("system_id", system.system_id)
        .param("name", name_param)
        .param("constellation_id", constellation_id)
        .param("security_status", system.security_status)
        .param("star_id", star_id)
        .param("security_class", security_class_param)
        .param("x", system.x)
        .param("y", system.y)
        .param("z", system.z)
        .param("planets", planets_json)
        .param("stargates", stargates))
        .await?;

    Ok(())
}